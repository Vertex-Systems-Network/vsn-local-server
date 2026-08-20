#!/usr/bin/env python3
"""Dispatch one P30 runner pack with GitHub CLI, wait, download partial evidence, and collect it."""
from __future__ import annotations
import argparse,json,shutil,subprocess,sys,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]; VERSION=(ROOT/'VERSION').read_text().strip(); CAND=ROOT/'scripts/release-candidate.py'
def run(args,check=True,capture=True):
 p=subprocess.run(args,cwd=ROOT,text=True,stdout=subprocess.PIPE if capture else None,stderr=subprocess.STDOUT if capture else None,check=False)
 if check and p.returncode: raise SystemExit((p.stdout or '')+f'\ncommand failed: {args}')
 return p

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--pack',required=True,choices=['linux-core','windows-core','macos-core','security-nightly']);ap.add_argument('--repo');ap.add_argument('--ref');ap.add_argument('--output-dir',default='dist-p30-github');ap.add_argument('--dry-run',action='store_true');a=ap.parse_args()
 if not a.dry_run and not shutil.which('gh'): raise SystemExit('gh CLI is required')
 cand=run([sys.executable,str(CAND),'id','--root',str(ROOT)]).stdout.strip()
 cmd=['gh','workflow','run','p30-run-pack.yml','-f',f'pack={a.pack}'];
 if a.ref: cmd += ['--ref',a.ref]
 if a.repo: cmd += ['-R',a.repo]
 if a.dry_run: print(json.dumps({'dispatch':cmd,'candidate_id':cand,'product_version':VERSION},indent=2));return
 run(cmd)
 # Find newest workflow_dispatch run for the requested workflow. Candidate verification happens after download.
 listcmd=['gh','run','list','--workflow','p30-run-pack.yml','--event','workflow_dispatch','--limit','10','--json','databaseId,createdAt,status,conclusion,headSha']
 if a.repo:listcmd += ['-R',a.repo]
 runs=json.loads(run(listcmd).stdout); runrow=runs[0] if runs else None
 if not runrow: raise SystemExit('could not locate dispatched p30-run-pack run')
 rid=str(runrow['databaseId']); watch=['gh','run','watch',rid,'--compact'];
 if a.repo:watch += ['-R',a.repo]
 # Do not require overall success: partial evidence is valuable.
 run(watch,check=False,capture=False)
 out=Path(a.output_dir)/f'{a.pack}-run-{rid}';out.mkdir(parents=True,exist_ok=True)
 dl=['gh','run','download',rid,'--pattern',f'vsn-p30-pack-{a.pack}','--dir',str(out)]
 if a.repo:dl += ['-R',a.repo]
 run(dl,check=False)
 ledgers=list(out.rglob('evidence.json'))
 if not ledgers: raise SystemExit(f'no evidence.json downloaded from run {rid}')
 merged=out/'evidence-merged.json'
 merge=[sys.executable,str(ROOT/'scripts/release-evidence.py'),'merge','--version',VERSION,'--candidate',cand,'--output',str(merged)]+[str(x) for x in ledgers]
 run(merge)
 report=out/'report.json';run([sys.executable,str(ROOT/'scripts/release-evidence.py'),'evaluate','--file',str(merged),'--report',str(report)])
 print(json.dumps({'run_id':rid,'pack':a.pack,'candidate_id':cand,'evidence':str(merged),'report':str(report)},indent=2))
if __name__=='__main__': main()
