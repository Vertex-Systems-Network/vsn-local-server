#!/usr/bin/env python3
from __future__ import annotations
import argparse,json,subprocess,sys,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def run(args):
 p=subprocess.run(args,cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False)
 if p.returncode: raise SystemExit(p.stdout)
 return p.stdout
def main():
 ap=argparse.ArgumentParser();ap.add_argument('command',choices=['write','check']);ap.add_argument('--evidence',default=str(ROOT/'docs/release-evidence-current.json'));ap.add_argument('--docs-dir',default=str(ROOT/'docs'));ap.add_argument('--roadmap',default=str(ROOT/'docs/roadmap-status.json'));a=ap.parse_args();e=Path(a.evidence).resolve();docs=Path(a.docs_dir).resolve();road=Path(a.roadmap).resolve()
 with tempfile.TemporaryDirectory(prefix='vsn-p30-sync-') as td:
  td=Path(td);out=td/'docs';out.mkdir();rmap=td/'roadmap.json';rmap.write_bytes(road.read_bytes())
  report=json.loads(run([sys.executable,str(ROOT/'scripts/release-evidence.py'),'evaluate','--file',str(e)]));(out/'release-evidence-current-report.json').write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')
  run([sys.executable,str(ROOT/'scripts/p30-progress.py'),'--evidence',str(e),'--roadmap',str(rmap),'--write'])
  (out/'roadmap-status.json').write_bytes(rmap.read_bytes())
  run([sys.executable,str(ROOT/'scripts/p30-scoreboard.py'),'--evidence',str(e),'--json-output',str(out/'p30-certification-status.json'),'--markdown-output',str(out/'p30-certification-status.md')])
  run([sys.executable,str(ROOT/'scripts/p30-fastest-path.py')])
  for n in ['p30-fastest-path.json','p30-fastest-path.md']:
   src=ROOT/'docs'/n
   if src.exists():(out/n).write_bytes(src.read_bytes())
  run([sys.executable,str(ROOT/'scripts/p30-runner-plan.py'),'--evidence',str(e),'--json-output',str(out/'p30-runner-plan.json'),'--markdown-output',str(out/'p30-runner-plan.md')])
  names=[p.name for p in out.iterdir()]
  if a.command=='write':
   docs.mkdir(parents=True,exist_ok=True)
   for n in names:(docs/n).write_bytes((out/n).read_bytes())
   print(json.dumps({'ok':True,'written':sorted(names)},indent=2));return
  stale=[]
  for n in names:
   cur=docs/n
   if not cur.exists() or cur.read_bytes()!=(out/n).read_bytes():stale.append(n)
  print(json.dumps({'ok':not stale,'stale':stale},indent=2));raise SystemExit(0 if not stale else 2)
if __name__=='__main__':main()
