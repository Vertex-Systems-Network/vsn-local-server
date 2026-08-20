#!/usr/bin/env python3
from __future__ import annotations
import argparse,hashlib,json,os,shutil,subprocess,sys,tempfile,zipfile
from datetime import datetime,timezone
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];EVID=ROOT/'scripts/release-evidence.py';SYNC=ROOT/'scripts/p30-sync-status.py';STORE=ROOT/'docs/p30-evidence-store';INDEX=ROOT/'docs/p30-evidence-journal.json'
ACTIVE={'active'}
def now():return datetime.now(timezone.utc).isoformat()
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def run(a):
 p=subprocess.run(a,cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False)
 if p.returncode:raise RuntimeError(p.stdout[-8000:])
 return p.stdout
def cand():return subprocess.check_output([sys.executable,str(ROOT/'scripts/release-candidate.py'),'id','--root',str(ROOT)],text=True).strip()
def blank_index():return {'schema_version':3,'product_version':(ROOT/'VERSION').read_text().strip(),'candidate_id':cand(),'updated_at':now(),'entries':[],'operations':[],'checkpoints':[]}
def load_index():
 if not INDEX.exists():return blank_index()
 d=json.loads(INDEX.read_text())
 if d.get('schema_version')!=3 or d.get('product_version')!=(ROOT/'VERSION').read_text().strip() or d.get('candidate_id')!=cand():raise RuntimeError('evidence journal identity/schema mismatch')
 return d
def write_index(d):d['updated_at']=now();INDEX.write_text(json.dumps(d,indent=2,sort_keys=True)+'\n')
def evalp(p):return json.loads(run([sys.executable,str(EVID),'evaluate','--file',str(p)]))
def rebuild(d,dry=False):
 version=d['product_version'];candidate=d['candidate_id']
 with tempfile.TemporaryDirectory(prefix='vsn-p30-rebuild-') as td:
  td=Path(td);base=td/'base.json';out=td/'current.json';run([sys.executable,str(EVID),'init','--version',version,'--candidate',candidate,'--output',str(base)]);inputs=[str(base)]
  for e in d['entries']:
   if e.get('state') in ACTIVE:inputs.append(str(ROOT/'docs'/e['evidence_ref']))
  if len(inputs)>1:run([sys.executable,str(EVID),'merge','--version',version,'--candidate',candidate,'--output',str(out),*inputs])
  else:out.write_bytes(base.read_bytes())
  report=evalp(out)
  if dry:return out.read_bytes(),report
  (ROOT/'docs/release-evidence-current.json').write_bytes(out.read_bytes());run([sys.executable,str(SYNC),'write']);return report
def verify_store(d):
 errs=[];seen=set()
 for e in d['entries']:
  b=e.get('bundle_sha256')
  if b in seen:errs.append(f'duplicate bundle {b}')
  seen.add(b);p=ROOT/'docs'/e.get('evidence_ref','')
  if not p.is_file():errs.append(f'missing evidence {e.get("evidence_ref")}');continue
  if sha(p)!=e.get('evidence_sha256'):errs.append(f'evidence hash mismatch {e.get("evidence_ref")}')
 return errs
def import_result(bundle,sha_file=None):
 from p30_result_lib import verify
 with tempfile.TemporaryDirectory(prefix='vsn-p30-import-') as td:
  v=verify(Path(bundle),root=ROOT,sha_file=sha_file,extract_to=Path(td));d=load_index();existing=next((x for x in d['entries'] if x['bundle_sha256']==v['bundle_sha256']),None)
  if existing:
   if existing['state']!='active':raise RuntimeError('bundle already journaled in non-active state; use governance operation')
   return {'ok':True,'replay':True,'bundle_sha256':v['bundle_sha256']}
  STORE.mkdir(parents=True,exist_ok=True);src=Path(v['evidence_file']);dst=STORE/f"{v['bundle_sha256']}.json";dst.write_bytes(src.read_bytes());entry={'bundle_sha256':v['bundle_sha256'],'pack_id':v['pack_id'],'imported_at':now(),'state':'active','evidence_ref':f'p30-evidence-store/{dst.name}','evidence_sha256':sha(dst),'reason':None,'superseded_by':None,'quarantined_at':None,'revoked_at':None}
  d['entries'].append(entry);d['operations'].append({'operation':'import','bundle_sha256':v['bundle_sha256'],'recorded_at':now(),'reason':None});write_index(d);report=rebuild(d);return {'ok':True,'replay':False,'bundle_sha256':v['bundle_sha256'],'report':report}
def checkpoint(path):
 d=load_index();errs=verify_store(d)
 if errs:raise RuntimeError('; '.join(errs))
 out=Path(path);out.parent.mkdir(parents=True,exist_ok=True)
 with zipfile.ZipFile(out,'w',zipfile.ZIP_DEFLATED,compresslevel=9) as z:
  z.writestr('P30_CHECKPOINT.json',json.dumps({'schema_version':1,'product_version':d['product_version'],'candidate_id':d['candidate_id'],'journal_sha256':hashlib.sha256(INDEX.read_bytes()).hexdigest(),'created_at':now()},indent=2,sort_keys=True)+'\n');z.write(INDEX,'docs/p30-evidence-journal.json');z.write(ROOT/'docs/release-evidence-current.json','docs/release-evidence-current.json')
  for e in d['entries']:z.write(ROOT/'docs'/e['evidence_ref'],f"docs/{e['evidence_ref']}")
 digest=sha(out);d['checkpoints'].append({'sha256':digest,'created_at':now(),'file':out.name});write_index(d);return {'checkpoint':str(out),'sha256':digest}
def restore_checkpoint(path,dry=False):
 p=Path(path)
 with tempfile.TemporaryDirectory(prefix='vsn-p30-checkpoint-') as td:
  td=Path(td)
  with zipfile.ZipFile(p) as z:
   if z.testzip():raise RuntimeError('checkpoint ZIP CRC failure')
   for n in z.namelist():
    q=Path(n)
    if q.is_absolute() or '..' in q.parts:raise RuntimeError('unsafe checkpoint path')
   z.extractall(td)
  meta=json.loads((td/'P30_CHECKPOINT.json').read_text());j=td/'docs/p30-evidence-journal.json';d=json.loads(j.read_text())
  if meta.get('candidate_id')!=cand() or d.get('candidate_id')!=cand() or d.get('product_version')!=(ROOT/'VERSION').read_text().strip():raise RuntimeError('checkpoint candidate/version mismatch')
  if hashlib.sha256(j.read_bytes()).hexdigest()!=meta.get('journal_sha256'):raise RuntimeError('checkpoint journal hash mismatch')
  for e in d['entries']:
   ep=td/'docs'/e['evidence_ref']
   if not ep.is_file() or sha(ep)!=e['evidence_sha256']:raise RuntimeError('checkpoint evidence mismatch')
  if dry:return {'ok':True,'dry_run':True,'entries':len(d['entries'])}
  STORE.mkdir(parents=True,exist_ok=True)
  for e in d['entries']:shutil.copy2(td/'docs'/e['evidence_ref'],ROOT/'docs'/e['evidence_ref'])
  write_index(d);report=rebuild(d);return {'ok':True,'entries':len(d['entries']),'report':report}
def main():
 ap=argparse.ArgumentParser();ap.add_argument('command',choices=['list','verify','import','rebuild','revoke','restore','quarantine','unquarantine','supersede','checkpoint','restore-checkpoint']);ap.add_argument('--bundle');ap.add_argument('--sha256');ap.add_argument('--bundle-sha');ap.add_argument('--replacement-sha');ap.add_argument('--reason');ap.add_argument('--path');ap.add_argument('--dry-run',action='store_true');a=ap.parse_args()
 try:
  if a.command=='import':print(json.dumps(import_result(a.bundle,a.sha256),indent=2,sort_keys=True));return
  if a.command=='checkpoint':print(json.dumps(checkpoint(a.path),indent=2,sort_keys=True));return
  if a.command=='restore-checkpoint':print(json.dumps(restore_checkpoint(a.path,a.dry_run),indent=2,sort_keys=True));return
  d=load_index()
  if a.command=='list':print(json.dumps(d,indent=2,sort_keys=True));return
  if a.command=='verify':
   errs=verify_store(d);rebuilt,rep=rebuild(d,True)
   cur=json.loads((ROOT/'docs/release-evidence-current.json').read_text());reb=json.loads(rebuilt.decode());cur.pop('updated_at',None);reb.pop('updated_at',None);same=cur==reb
   print(json.dumps({'ok':not errs and same,'errors':errs,'authoritative_matches_rebuild':same,'active':sum(1 for e in d['entries'] if e['state']=='active'),'satisfied':rep['satisfied']},indent=2));raise SystemExit(0 if not errs and same else 2)
  if a.command=='rebuild':
   rb,rep=rebuild(d,True) if a.dry_run else (None,rebuild(d,False));print(json.dumps({'ok':True,'dry_run':a.dry_run,'report':rep},indent=2,sort_keys=True));return
  if not a.bundle_sha:raise RuntimeError('--bundle-sha required')
  e=next((x for x in d['entries'] if x['bundle_sha256']==a.bundle_sha),None)
  if not e:raise RuntimeError('bundle not found in journal')
  op=a.command
  if op in {'revoke','quarantine','supersede'} and not a.reason:raise RuntimeError('--reason required')
  if op=='revoke':e['state']='revoked';e['revoked_at']=now();e['reason']=a.reason
  elif op=='restore':e['state']='active';e['revoked_at']=None;e['reason']=None;e['superseded_by']=None;e['quarantined_at']=None
  elif op=='quarantine':e['state']='quarantined';e['quarantined_at']=now();e['reason']=a.reason
  elif op=='unquarantine':e['state']='active';e['quarantined_at']=None;e['reason']=None
  elif op=='supersede':
   if not a.replacement_sha:raise RuntimeError('--replacement-sha required')
   r=next((x for x in d['entries'] if x['bundle_sha256']==a.replacement_sha and x['state']=='active'),None)
   if not r:raise RuntimeError('replacement bundle must already be active')
   e['state']='superseded';e['superseded_by']=a.replacement_sha;e['reason']=a.reason
  d['operations'].append({'operation':op,'bundle_sha256':a.bundle_sha,'recorded_at':now(),'reason':a.reason,'replacement_sha':a.replacement_sha})
  if a.dry_run:
   _,rep=rebuild(d,True);print(json.dumps({'ok':True,'dry_run':True,'state':e['state'],'report':rep},indent=2));return
  write_index(d);rep=rebuild(d,False);print(json.dumps({'ok':True,'state':e['state'],'report':rep},indent=2,sort_keys=True))
 except Exception as ex:print(str(ex),file=sys.stderr);raise SystemExit(2)
if __name__=='__main__':main()
