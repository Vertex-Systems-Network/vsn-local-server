#!/usr/bin/env python3
from __future__ import annotations
import argparse,hashlib,json,os,subprocess,sys,zipfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]

def run(a):
 p=subprocess.run(a,cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False)
 if p.returncode:raise RuntimeError(p.stdout[-10000:])
 return p.stdout

def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--output-dir',default=str(ROOT/'dist-pkg01/final'));a=ap.parse_args();out=Path(a.output_dir);out.mkdir(parents=True,exist_ok=True)
 st=json.loads(run([sys.executable,str(ROOT/'scripts/pkg01-linux-core.py'),'status']))
 if not st.get('complete') or st.get('valid_passes')!=6:raise SystemExit('PKG-01 cannot finalize until all 6 controls are valid PASS')
 run([sys.executable,str(ROOT/'scripts/release-candidate.py'),'verify','--root',str(ROOT),'--file','docs/release-candidate-current.json'])
 run([sys.executable,str(ROOT/'scripts/p30-evidence-governance.py'),'verify'])
 run([sys.executable,str(ROOT/'scripts/release-gate.py')])
 # Update status after all gates, then build a deterministic integrity manifest for the distributable source package.
 st=json.loads(run([sys.executable,str(ROOT/'scripts/pkg01-linux-core.py'),'status']))
 version=(ROOT/'VERSION').read_text().strip();banned={'.git','target','node_modules','dist','dist-p30','dist-pkg01','__pycache__'};files=[]
 for p in ROOT.rglob('*'):
  if not p.is_file():continue
  rel=p.relative_to(ROOT)
  if rel.as_posix()=='SOURCE_SHA256SUMS.txt':continue
  if any(part in banned for part in rel.parts):continue
  files.append(rel)
 files=sorted(files,key=lambda x:x.as_posix());lines=[f"{sha(ROOT/r)}  {r.as_posix()}" for r in files];(ROOT/'SOURCE_SHA256SUMS.txt').write_text('\n'.join(lines)+'\n')
 name=f'vsn-platform-batch-{version}-PKG01-COMPLETE';zpath=out/f'{name}.zip'
 if zpath.exists():zpath.unlink()
 with zipfile.ZipFile(zpath,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=9) as z:
  for rel in files+[Path('SOURCE_SHA256SUMS.txt')]:
   p=ROOT/rel;info=zipfile.ZipInfo(rel.as_posix(),date_time=(2026,8,20,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;mode=0o100755 if rel.as_posix().startswith(('scripts/','packaging/')) or rel.suffix in {'.sh','.py','.ps1'} else 0o100644;info.external_attr=mode<<16;z.writestr(info,p.read_bytes(),compress_type=zipfile.ZIP_DEFLATED,compresslevel=9)
 digest=sha(zpath);side=out/f'{name}.sha256';side.write_text(f'{digest}  {zpath.name}\n')
 payload={'ok':True,'package_id':'PKG-01','product_version':version,'candidate_id':st['candidate_id'],'valid_passes':6,'required_passes':6,'bundle':str(zpath),'sha256':digest,'sha256_file':str(side)};print(json.dumps(payload,indent=2,sort_keys=True));return 0
if __name__=='__main__':raise SystemExit(main())
