#!/usr/bin/env python3
"""Generate a non-secret, candidate-bound runner attestation for P30 pack evidence."""
from __future__ import annotations
import argparse,hashlib,json,os,platform,shutil,subprocess,sys
from datetime import datetime,timezone
from pathlib import Path
from p30_platform import canonical_platform
ROOT=Path(__file__).resolve().parents[1]
CAND=ROOT/'scripts/release-candidate.py'

def now(): return datetime.now(timezone.utc).isoformat()
def sha(path:Path):
 h=hashlib.sha256()
 with path.open('rb') as f:
  for b in iter(lambda:f.read(1024*1024),b''): h.update(b)
 return h.hexdigest()
def tool_version(name,args=('--version',)):
 p=shutil.which(name)
 if not p:return {'available':False,'path':None,'version':None}
 try:
  r=subprocess.run([p,*args],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=15,check=False)
  line=(r.stdout or '').strip().splitlines()[0][:512] if (r.stdout or '').strip() else f'exit={r.returncode}'
 except Exception as e: line=f'error:{type(e).__name__}'
 return {'available':True,'path':p,'version':line}
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--root',default=str(ROOT));ap.add_argument('--output',required=True);ap.add_argument('--pack');a=ap.parse_args();root=Path(a.root).resolve()
 version=(root/'VERSION').read_text().strip(); candidate=subprocess.check_output([sys.executable,str(root/'scripts/release-candidate.py'),'id','--root',str(root)],text=True).strip()
 manifest=root/'SOURCE_SHA256SUMS.txt'
 data={
  'schema_version':1,'product_version':version,'candidate_id':candidate,'pack_id':a.pack,
  'recorded_at':now(),'host':{'system':canonical_platform(),'release':platform.release(),'machine':platform.machine().lower(),'python':platform.python_version()},
  'source_manifest_sha256':sha(manifest) if manifest.is_file() else None,
  'tools':{n:tool_version(n) for n in ['cargo','rustc','rustup','node','npm','python3','dpkg-deb','pwsh','dotnet','codesign','pkgbuild','productbuild','docker','podman','bwrap','psql','pg_dump','gh']},
  'ci':{'github_actions':os.getenv('GITHUB_ACTIONS')=='true','run_id':os.getenv('GITHUB_RUN_ID'),'run_attempt':os.getenv('GITHUB_RUN_ATTEMPT'),'repository':os.getenv('GITHUB_REPOSITORY'),'commit_sha':os.getenv('GITHUB_SHA')}
 }
 out=Path(a.output);out.parent.mkdir(parents=True,exist_ok=True);out.write_text(json.dumps(data,indent=2,sort_keys=True)+'\n');print(out)
if __name__=='__main__':main()
