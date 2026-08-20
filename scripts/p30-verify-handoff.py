#!/usr/bin/env python3
"""Verify a P30 handoff ZIP and optionally execute its declared pack after safe verification."""
from __future__ import annotations
import argparse,hashlib,json,subprocess,sys,tempfile,zipfile,shutil
from pathlib import Path,PurePosixPath
MAX_FILES=5000;MAX_MEMBER=1024**3;MAX_TOTAL=2*1024**3

def sha(p):
 h=hashlib.sha256()
 with Path(p).open('rb') as f:
  for b in iter(lambda:f.read(1024*1024),b''):h.update(b)
 return h.hexdigest()
def safe_name(n):
 p=PurePosixPath(n)
 if not n or n.startswith('/') or p.is_absolute() or '..' in p.parts:raise RuntimeError(f'unsafe ZIP path: {n}')
 return p
def extract(bundle,dst):
 with zipfile.ZipFile(bundle) as z:
  bad=z.testzip()
  if bad:raise RuntimeError(f'ZIP CRC failure: {bad}')
  infos=[i for i in z.infolist() if not i.is_dir()]
  if not infos or len(infos)>MAX_FILES:raise RuntimeError('invalid handoff member count')
  roots={safe_name(i.filename).parts[0] for i in infos}
  if len(roots)!=1:raise RuntimeError('handoff must contain exactly one root directory')
  seen=set();total=0;base=Path(dst).resolve();base.mkdir(parents=True,exist_ok=True)
  for i in infos:
   if i.filename in seen:raise RuntimeError(f'duplicate ZIP member: {i.filename}')
   seen.add(i.filename);total+=i.file_size
   if i.file_size>MAX_MEMBER or total>MAX_TOTAL:raise RuntimeError('handoff ZIP exceeds size ceiling')
   rel=safe_name(i.filename);target=(base/Path(*rel.parts)).resolve()
   if base not in target.parents:raise RuntimeError('handoff ZIP path escape')
   target.parent.mkdir(parents=True,exist_ok=True)
   with z.open(i) as src,target.open('wb') as out:shutil.copyfileobj(src,out,1024*1024)
  return base/next(iter(roots))

def main():
 ap=argparse.ArgumentParser();ap.add_argument('bundle');ap.add_argument('--sha256');ap.add_argument('--execute',action='store_true');ap.add_argument('--output-dir',default='dist-p30');a=ap.parse_args();bundle=Path(a.bundle)
 if not bundle.is_file():raise SystemExit('handoff bundle not found')
 got=sha(bundle)
 if a.sha256:
  expected=Path(a.sha256).read_text().split()[0].strip().lower()
  if got!=expected:raise SystemExit('handoff bundle SHA-256 mismatch')
 try:
  with tempfile.TemporaryDirectory(prefix='vsn-p30-handoff-') as td:
   root=extract(bundle,td);meta_path=root/'P30_HANDOFF.json';manifest=root/'SOURCE_SHA256SUMS.txt'
   if not meta_path.is_file():raise RuntimeError('P30_HANDOFF.json missing')
   meta=json.loads(meta_path.read_text())
   if meta.get('schema_version')!=1:raise RuntimeError('unsupported handoff schema')
   if not manifest.is_file():raise RuntimeError('embedded source manifest missing')
   if hashlib.sha256(manifest.read_bytes()).hexdigest()!=meta.get('source_manifest_sha256'):raise RuntimeError('embedded source manifest digest mismatch')
   declared=[]
   for line in manifest.read_text().splitlines():
    if not line.strip():continue
    expected,rel=line.split('  ',1);p=root/rel;declared.append(rel)
    if not p.is_file() or sha(p)!=expected:raise RuntimeError(f'handoff source mismatch: {rel}')
   if len(declared)!=meta.get('source_file_count'):raise RuntimeError('handoff source file count mismatch')
   actual={p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file()}
   if actual!=set(declared)|{'SOURCE_SHA256SUMS.txt','P30_HANDOFF.json'}:raise RuntimeError('handoff contains unlisted or missing files')
   version=(root/'VERSION').read_text().strip()
   if version!=meta.get('product_version'):raise RuntimeError('handoff product version mismatch')
   candidate=subprocess.check_output([sys.executable,str(root/'scripts/release-candidate.py'),'id','--root',str(root)],text=True).strip()
   if candidate!=meta.get('candidate_id'):raise RuntimeError('handoff candidate fingerprint mismatch')
   packs=json.loads((root/'certification/p30-runner-packs.json').read_text());pack=next((x for x in packs.get('packs',[]) if x.get('id')==meta.get('pack_id')),None)
   if not pack:raise RuntimeError('handoff pack is not declared by embedded runner-pack manifest')
   for key in ['platform','execution_mode','controls','required_tools']:
    if meta.get(key)!=pack.get(key):raise RuntimeError(f'handoff {key} mismatch')
   result={'ok':True,'bundle_sha256':got,'product_version':version,'candidate_id':candidate,'pack_id':meta.get('pack_id'),'source_file_count':len(declared)};print(json.dumps(result,indent=2,sort_keys=True))
   if a.execute:
    cmd=[sys.executable,str(root/'scripts/p30-run-pack.py'),'--pack',meta['pack_id'],'--output-dir',str(Path(a.output_dir).resolve())];raise SystemExit(subprocess.run(cmd,cwd=root).returncode)
 except Exception as e:
  print(str(e),file=sys.stderr);return 2
 return 0
if __name__=='__main__':raise SystemExit(main())
