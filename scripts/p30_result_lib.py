#!/usr/bin/env python3
from __future__ import annotations
import hashlib,json,shutil,subprocess,sys,tempfile,zipfile
from pathlib import Path,PurePosixPath
ROOT=Path(__file__).resolve().parents[1]
class ResultError(RuntimeError):pass

def sha(p:Path):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for b in iter(lambda:f.read(1024*1024),b''):h.update(b)
 return h.hexdigest()
def safe_name(n):
 p=PurePosixPath(n)
 if not n or n.startswith('/') or p.is_absolute() or '..' in p.parts:raise ResultError(f'unsafe zip path: {n}')
 return p
def safe_extract(bundle:Path,dst:Path):
 with zipfile.ZipFile(bundle) as z:
  if z.testzip():raise ResultError('ZIP CRC failure')
  infos=[i for i in z.infolist() if not i.is_dir()]
  if len(infos)>5000:raise ResultError('too many ZIP members')
  roots={safe_name(i.filename).parts[0] for i in infos}
  if len(roots)!=1:raise ResultError('result ZIP must have one root')
  total=0;seen=set();root=dst/next(iter(roots));dst=dst.resolve();dst.mkdir(parents=True,exist_ok=True)
  for i in infos:
   if i.filename in seen:raise ResultError('duplicate ZIP member')
   seen.add(i.filename);total+=i.file_size
   if i.file_size>1024**3 or total>2*1024**3:raise ResultError('result ZIP exceeds size ceiling')
   rel=safe_name(i.filename);t=(dst/Path(*rel.parts)).resolve()
   if dst not in t.parents:raise ResultError('ZIP path escape')
   t.parent.mkdir(parents=True,exist_ok=True)
   with z.open(i) as src,t.open('wb') as out:shutil.copyfileobj(src,out)
  return root
def eval_evidence(p:Path,root=ROOT):
 q=subprocess.run([sys.executable,str(root/'scripts/release-evidence.py'),'evaluate','--file',str(p)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
 if q.returncode:raise ResultError(q.stdout)
 return json.loads(q.stdout)
def build(run:Path,outdir:Path,root=ROOT):
 for n in ['summary.json','evidence.json','report.json','runner-attestation.json']:
  if not (run/n).is_file():raise ResultError(f'missing {n}')
 summary=json.loads((run/'summary.json').read_text());evidence=json.loads((run/'evidence.json').read_text());report=json.loads((run/'report.json').read_text());att=json.loads((run/'runner-attestation.json').read_text())
 version=(root/'VERSION').read_text().strip();cand=subprocess.check_output([sys.executable,str(root/'scripts/release-candidate.py'),'id','--root',str(root)],text=True).strip();pack=summary.get('pack_id')
 if any(x.get('product_version')!=version or x.get('candidate_id')!=cand for x in [summary,evidence,att]):raise ResultError('result identity mismatch')
 if eval_evidence(run/'evidence.json',root)!=report:raise ResultError('stale result report')
 files=[]
 for p in sorted(x for x in run.rglob('*') if x.is_file()):
  rel=p.relative_to(run).as_posix();files.append({'path':rel,'sha256':sha(p),'size':p.stat().st_size})
 meta={'schema_version':1,'bundle_type':'p30-result','product_version':version,'candidate_id':cand,'pack_id':pack,'files':files,'evidence_sha256':sha(run/'evidence.json'),'attestation_sha256':sha(run/'runner-attestation.json')}
 outdir.mkdir(parents=True,exist_ok=True);base=f'vsn-p30-result-{pack}-{version}-{cand[:12]}';zpath=outdir/f'{base}.zip'
 with zipfile.ZipFile(zpath,'w',zipfile.ZIP_DEFLATED,compresslevel=9) as z:
  z.writestr(f'{base}/P30_RESULT.json',json.dumps(meta,indent=2,sort_keys=True)+'\n')
  for r in files:z.write(run/r['path'],f"{base}/{r['path']}")
 side=zpath.with_suffix('.zip.sha256');side.write_text(f'{sha(zpath)}  {zpath.name}\n')
 return {'bundle':str(zpath),'sha256_file':str(side),'bundle_sha256':sha(zpath),'pack_id':pack,'candidate_id':cand,'product_version':version}
def verify(bundle:Path,root=ROOT,sha_file=None,extract_to=None):
 if sha_file and sha(bundle)!=Path(sha_file).read_text().split()[0]:raise ResultError('bundle SHA mismatch')
 own=None
 if extract_to is None:own=tempfile.TemporaryDirectory(prefix='vsn-p30-result-');extract_to=Path(own.name)
 rr=safe_extract(bundle,Path(extract_to));mp=rr/'P30_RESULT.json'
 if not mp.is_file():raise ResultError('not a P30 result bundle')
 m=json.loads(mp.read_text());version=(root/'VERSION').read_text().strip();cand=subprocess.check_output([sys.executable,str(root/'scripts/release-candidate.py'),'id','--root',str(root)],text=True).strip()
 if m.get('product_version')!=version or m.get('candidate_id')!=cand:raise ResultError('result version/candidate mismatch')
 declared={r['path']:r for r in m.get('files',[])};actual={p.relative_to(rr).as_posix() for p in rr.rglob('*') if p.is_file() and p.name!='P30_RESULT.json'}
 if set(declared)!=actual:raise ResultError('result file set mismatch')
 for rel,r in declared.items():
  p=rr/rel
  if p.stat().st_size!=r['size'] or sha(p)!=r['sha256']:raise ResultError(f'result file mismatch: {rel}')
 ev=rr/'evidence.json';att=rr/'runner-attestation.json';summary=json.loads((rr/'summary.json').read_text());report=json.loads((rr/'report.json').read_text())
 if sha(ev)!=m['evidence_sha256'] or sha(att)!=m['attestation_sha256']:raise ResultError('result digest mismatch')
 if eval_evidence(ev,root)!=report:raise ResultError('result report mismatch')
 if summary.get('candidate_id')!=cand or summary.get('pack_id')!=m.get('pack_id'):raise ResultError('result summary mismatch')
 return {'ok':True,'bundle_sha256':sha(bundle),'candidate_id':cand,'product_version':version,'pack_id':m.get('pack_id'),'extracted_root':str(rr),'evidence_file':str(ev)}
