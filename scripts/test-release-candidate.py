#!/usr/bin/env python3
from pathlib import Path
import json,shutil,subprocess,sys,tempfile
ROOT=Path(__file__).resolve().parents[1]; TOOL=ROOT/'scripts/release-candidate.py'; E=ROOT/'scripts/release-evidence.py'
def run(*a,cwd=None,ok=True):
 p=subprocess.run([sys.executable,*map(str,a)],cwd=cwd,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
 if ok and p.returncode:raise SystemExit(p.stdout)
 return p
def main():
 with tempfile.TemporaryDirectory() as td:
  t=Path(td)/'r';t.mkdir();(t/'VERSION').write_text('9.9.9\n');(t/'scripts').mkdir();(t/'scripts/a.py').write_text('a');(t/'docs').mkdir();(t/'docs/release-evidence-current.json').write_text('{}')
  a=json.loads(run(TOOL,'show','--root',t).stdout);(t/'docs/release-evidence-current.json').write_text('{"changed":true}')
  b=json.loads(run(TOOL,'show','--root',t).stdout);assert a['candidate_id']==b['candidate_id']
  (t/'scripts/a.py').write_text('b');c=json.loads(run(TOOL,'show','--root',t).stdout);assert c['candidate_id']!=a['candidate_id']
  (t/'certification').mkdir();(t/'certification/pack.json').write_text('{}');d=json.loads(run(TOOL,'show','--root',t).stdout);assert d['candidate_id']!=c['candidate_id']
 with tempfile.TemporaryDirectory() as td:
  td=Path(td);f1=td/'a.json';f2=td/'b.json';out=td/'m.json'
  run(E,'init','--version','9.9.9','--candidate','a'*64,'--output',f1);run(E,'init','--version','9.9.9','--candidate','b'*64,'--output',f2)
  p=run(E,'merge','--version','9.9.9','--output',out,f1,f2,ok=False);assert p.returncode!=0 and 'candidate mismatch' in p.stdout
 print('release candidate regression PASS')
if __name__=='__main__':main()
