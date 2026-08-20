#!/usr/bin/env python3
"""Bounded PostgreSQL backup/verify/restore helper for VSN Control Plane DR."""
from __future__ import annotations
import argparse, hashlib, json, os, shutil, subprocess, sys, tempfile, time
from pathlib import Path

MAX_BACKUP_BYTES=32*1024*1024*1024
TIMEOUT_SECONDS=30*60

def sha256_file(path:Path)->str:
    h=hashlib.sha256()
    with path.open('rb') as f:
        for chunk in iter(lambda:f.read(1024*1024),b''): h.update(chunk)
    return h.hexdigest()

def tool(name:str)->str:
    p=shutil.which(name)
    if not p: raise SystemExit(f'{name} is required')
    return p

def pg_env(dsn_env:str)->dict[str,str]:
    dsn=os.environ.get(dsn_env,'').strip()
    if not dsn: raise SystemExit(f'{dsn_env} is not configured')
    if any(c in dsn for c in '\r\n\0'): raise SystemExit('PostgreSQL DSN contains control characters')
    env=os.environ.copy();env['PGDATABASE']=dsn
    return env

def run(args:list[str],env:dict[str,str])->None:
    try:r=subprocess.run(args,env=env,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=TIMEOUT_SECONDS,check=False)
    except subprocess.TimeoutExpired: raise SystemExit(f'command timed out: {args[0]}')
    if r.returncode!=0: raise SystemExit(f'{Path(args[0]).name} failed: {r.stderr.decode("utf-8","replace")[:4096]}')

def backup(args):
    out=Path(args.output).expanduser().resolve();out.parent.mkdir(parents=True,exist_ok=True)
    if out.exists() and not args.overwrite: raise SystemExit('backup output already exists; pass --overwrite to replace it')
    with tempfile.NamedTemporaryFile(prefix='.vsn-dr-',suffix='.dump',dir=out.parent,delete=False) as tf: temp=Path(tf.name)
    try:
        run([tool('pg_dump'),'--format=custom','--no-owner','--no-privileges','--file',str(temp)],pg_env(args.dsn_env))
        size=temp.stat().st_size
        if size<=0 or size>MAX_BACKUP_BYTES: raise SystemExit('backup size is outside safety bounds')
        digest=sha256_file(temp);os.replace(temp,out)
        manifest={'version':1,'kind':'vsn-control-plane-postgres','created_at_unix_ms':int(time.time()*1000),'file':out.name,'bytes':size,'sha256':digest,'format':'pg_dump-custom','dsn_env':args.dsn_env}
        mp=Path(str(out)+'.manifest.json');mp.write_text(json.dumps(manifest,indent=2)+'\n')
        print(json.dumps({'ok':True,'backup':str(out),'manifest':str(mp),'bytes':size,'sha256':digest},indent=2))
    finally:
        temp.unlink(missing_ok=True)

def load_manifest(path:Path,manifest_path:Path|None):
    mp=manifest_path or Path(str(path)+'.manifest.json')
    data=json.loads(mp.read_text());expected=data.get('sha256','');actual=sha256_file(path)
    if expected!=actual: raise SystemExit('backup SHA-256 does not match manifest')
    if data.get('kind')!='vsn-control-plane-postgres' or data.get('format')!='pg_dump-custom': raise SystemExit('backup manifest type is invalid')
    return data,mp

def verify(args):
    path=Path(args.file).expanduser().resolve();data,mp=load_manifest(path,Path(args.manifest).resolve() if args.manifest else None)
    run([tool('pg_restore'),'--list',str(path)],os.environ.copy())
    print(json.dumps({'ok':True,'backup':str(path),'manifest':str(mp),'bytes':path.stat().st_size,'sha256':data['sha256']},indent=2))

def restore(args):
    if args.confirm!='RESTORE_CONTROL_PLANE': raise SystemExit('restore requires --confirm RESTORE_CONTROL_PLANE')
    path=Path(args.file).expanduser().resolve();load_manifest(path,Path(args.manifest).resolve() if args.manifest else None)
    run([tool('pg_restore'),'--clean','--if-exists','--no-owner','--no-privileges','--exit-on-error','--dbname','postgres',str(path)],pg_env(args.dsn_env))
    print(json.dumps({'ok':True,'restored':str(path),'dsn_env':args.dsn_env,'warning':'run Control Plane /ready and consistency validation before accepting traffic'},indent=2))

def main():
    ap=argparse.ArgumentParser();sub=ap.add_subparsers(dest='cmd',required=True)
    b=sub.add_parser('backup');b.add_argument('--dsn-env',default='VSN_CONTROL_POSTGRES_DSN');b.add_argument('--output',required=True);b.add_argument('--overwrite',action='store_true');b.set_defaults(fn=backup)
    v=sub.add_parser('verify');v.add_argument('--file',required=True);v.add_argument('--manifest');v.set_defaults(fn=verify)
    r=sub.add_parser('restore');r.add_argument('--dsn-env',default='VSN_CONTROL_POSTGRES_DSN');r.add_argument('--file',required=True);r.add_argument('--manifest');r.add_argument('--confirm',required=True);r.set_defaults(fn=restore)
    args=ap.parse_args();args.fn(args)
if __name__=='__main__':main()
