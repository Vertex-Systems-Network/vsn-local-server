#!/usr/bin/env python3
"""Build a candidate-bound portable P30 certification handoff bundle."""
from __future__ import annotations
import argparse,hashlib,json,os,subprocess,sys,zipfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
VERSION=(ROOT/'VERSION').read_text().strip(); MAN=ROOT/'SOURCE_SHA256SUMS.txt'
CAND=ROOT/'scripts/release-candidate.py'; PACKS=ROOT/'certification/p30-runner-packs.json'

def sha(p):
 h=hashlib.sha256();
 with p.open('rb') as f:
  for b in iter(lambda:f.read(1024*1024),b''): h.update(b)
 return h.hexdigest()

def main():
 ap=argparse.ArgumentParser(); ap.add_argument('--pack',required=True); ap.add_argument('--output-dir',default='dist-p30-handoff'); a=ap.parse_args()
 packs=json.loads(PACKS.read_text()); pack=next((x for x in packs['packs'] if x['id']==a.pack),None)
 if not pack: raise SystemExit(f'unknown pack: {a.pack}')
 if not MAN.is_file(): raise SystemExit('SOURCE_SHA256SUMS.txt missing; freeze source before handoff')
 # Verify frozen source before packaging.
 files=[]
 for line in MAN.read_text().splitlines():
  if not line.strip(): continue
  expected,rel=line.split('  ',1); p=ROOT/rel
  if not p.is_file() or sha(p)!=expected: raise SystemExit(f'source manifest mismatch: {rel}')
  files.append(rel)
 candidate=subprocess.check_output([sys.executable,str(CAND),'id','--root',str(ROOT)],text=True).strip()
 current=json.loads((ROOT/'docs/release-candidate-current.json').read_text())
 if current.get('candidate_id')!=candidate: raise SystemExit('committed release candidate is stale')
 out=Path(a.output_dir); out.mkdir(parents=True,exist_ok=True)
 base=f'vsn-p30-{a.pack}-{VERSION}-{candidate[:12]}'
 zip_path=out/f'{base}.zip'; meta_path=out/f'{base}.handoff.json'
 meta={'schema_version':1,'product_version':VERSION,'candidate_id':candidate,'pack_id':a.pack,'platform':pack['platform'],'execution_mode':pack['execution_mode'],'controls':pack['controls'],'required_tools':pack['required_tools'],'source_manifest_sha256':sha(MAN),'source_file_count':len(files),'launch':['python','scripts/p30-run-pack.py','--pack',a.pack,'--output-dir','dist-p30']}
 meta_path.write_text(json.dumps(meta,indent=2,sort_keys=True)+'\n')
 with zipfile.ZipFile(zip_path,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=9) as z:
  for rel in files: z.write(ROOT/rel,arcname=f'{base}/{rel}')
  z.write(MAN,arcname=f'{base}/SOURCE_SHA256SUMS.txt')
  z.writestr(f'{base}/P30_HANDOFF.json',json.dumps(meta,indent=2,sort_keys=True)+'\n')
 checksum=sha(zip_path); (out/f'{base}.zip.sha256').write_text(f'{checksum}  {zip_path.name}\n')
 print(json.dumps({**meta,'bundle':str(zip_path),'bundle_sha256':checksum},indent=2,sort_keys=True))
if __name__=='__main__': main()
