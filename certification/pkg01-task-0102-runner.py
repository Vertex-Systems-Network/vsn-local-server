#!/usr/bin/env python3
from __future__ import annotations
import argparse, hashlib, json, os, platform, shutil, subprocess
from datetime import datetime, timezone
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
PRODUCT_VERSION='0.38.1'
RUST_VERSION='1.97.1'
TARGET='x86_64-unknown-linux-gnu'
EXPECTED_REPO='Vertex-Systems-Network/vsn-local-server'
TOOLS=('rustc','cargo','rustfmt','cargo-clippy')

def now(): return datetime.now(timezone.utc).isoformat()
def sha256(path:Path)->str:
    h=hashlib.sha256()
    with path.open('rb') as f:
        for b in iter(lambda:f.read(1024*1024),b''): h.update(b)
    return h.hexdigest()
def run(a): return subprocess.run(a,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False)
def tool(name):
    p=shutil.which(name)
    if not p: return {'path':None,'version':None,'output':'missing','rc':127}
    r=run([p,'--version']); out=(r.stdout or '').strip(); parts=out.split()
    return {'path':p,'version':parts[1] if len(parts)>1 else None,'output':out,'rc':r.returncode}
def release_identity():
    p=ROOT/'docs'/'release-candidate-current.json'
    m=ROOT/'SOURCE_SHA256SUMS.txt'
    d=json.loads(p.read_text())
    candidate=d.get('candidate_id')
    if not isinstance(candidate,str) or len(candidate)!=64:
        raise RuntimeError('invalid current release candidate id')
    if d.get('product_version')!=PRODUCT_VERSION:
        raise RuntimeError('release candidate product version mismatch')
    if not m.is_file():
        raise RuntimeError('SOURCE_SHA256SUMS.txt missing')
    return candidate,sha256(m)
def main():
    ap=argparse.ArgumentParser();ap.add_argument('--output',default='dist-pkg01/task-0102-result.json');a=ap.parse_args()
    candidate,manifest_sha=release_identity()
    vals={t:tool(t) for t in TOOLS}
    system=platform.system().lower();machine=platform.machine().lower()
    repo=os.getenv('GITHUB_REPOSITORY');run_id=os.getenv('GITHUB_RUN_ID');run_attempt=os.getenv('GITHUB_RUN_ATTEMPT');commit=os.getenv('GITHUB_SHA');event=os.getenv('GITHUB_EVENT_NAME')
    gha=os.getenv('GITHUB_ACTIONS')=='true'
    exact=(vals['rustc']['version']==RUST_VERSION and vals['cargo']['version']==RUST_VERSION and all(vals[t]['path'] and vals[t]['rc']==0 for t in TOOLS))
    host_ok=system=='linux' and machine in {'x86_64','amd64'}
    ci_ok=gha and repo==EXPECTED_REPO and bool(run_id) and bool(commit) and len(commit)==40
    att={'schema_version':2,'product_version':PRODUCT_VERSION,'candidate_id':candidate,'pack_id':'pkg01-build-foundation-0102','recorded_at':now(),'host':{'system':system,'release':platform.release(),'machine':machine,'python':platform.python_version()},'source_manifest_sha256':manifest_sha,'ci':{'github_actions':gha,'event_name':event,'run_id':run_id,'run_attempt':run_attempt,'repository':repo,'commit_sha':commit}}
    run_url=f'https://github.com/{repo}/actions/runs/{run_id}' if gha and repo and run_id else 'local://pkg01-build-foundation-0102'
    d={'schema_version':3,'package_id':'PKG-01','task_id':'01.02','candidate_id':candidate,'product_version':PRODUCT_VERSION,'rust_toolchain':RUST_VERSION,'target':TARGET,'recorded_at':now(),'tools':vals,'runner_attestation':att,'provenance':{'run_url':run_url,'commit_sha':commit,'repository':repo,'github_actions':gha,'event_name':event,'run_id':run_id,'run_attempt':run_attempt},'all_pass':bool(exact and host_ok and ci_ok)}
    out=Path(a.output);out.parent.mkdir(parents=True,exist_ok=True);out.write_text(json.dumps(d,indent=2,sort_keys=True)+'\n')
    Path(str(out)+'.sha256').write_text(f'{sha256(out)}  {out.name}\n')
    print(json.dumps({'all_pass':d['all_pass'],'candidate_id':candidate,'source_manifest_sha256':manifest_sha,'tools':vals,'host':att['host'],'ci':att['ci'],'output':str(out)},indent=2,sort_keys=True))
    return 0 if d['all_pass'] else 4
if __name__=='__main__': raise SystemExit(main())
