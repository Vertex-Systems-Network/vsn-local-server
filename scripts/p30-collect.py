#!/usr/bin/env python3
"""Collect/merge P30 evidence ledgers from workflow artifact directories and recalculate roadmap."""
from __future__ import annotations
import argparse, json, subprocess, sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
VERSION=(ROOT/'VERSION').read_text().strip()
EVID=ROOT/'scripts/release-evidence.py'; CAND=ROOT/'scripts/release-candidate.py'
PROGRESS=ROOT/'scripts/p30-progress.py'

def discover(paths):
    out=[]
    for raw in paths:
        p=Path(raw)
        candidates=[p] if p.is_file() else sorted(p.rglob('*.json')) if p.exists() else []
        for c in candidates:
            try:d=json.loads(c.read_text())
            except Exception:continue
            if isinstance(d,dict) and d.get('schema_version') in {1,2,3,4} and isinstance(d.get('checks'),list) and d.get('product_version'):
                out.append(c)
    # deterministic, de-duplicated paths
    return list(dict.fromkeys(x.resolve() for x in out))

def run(cmd):
    return subprocess.run(cmd,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False)

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('inputs',nargs='+',help='Evidence JSON files or directories containing workflow evidence artifacts')
    ap.add_argument('--version',default=VERSION)
    ap.add_argument('--output',default=str(ROOT/'docs/release-evidence-current.json'))
    ap.add_argument('--report',default=str(ROOT/'docs/release-evidence-current-report.json'))
    ap.add_argument('--write-roadmap',action='store_true')
    ap.add_argument('--require-certified',action='store_true')
    a=ap.parse_args()
    files=discover(a.inputs)
    candidate=subprocess.check_output([sys.executable,str(CAND),'id','--root',str(ROOT)],text=True).strip()
    if not files:raise SystemExit('no evidence ledgers discovered')
    for f in files:
        d=json.loads(f.read_text())
        if d.get('product_version')!=a.version:raise SystemExit(f'version mismatch: {f} has {d.get("product_version")}, expected {a.version}')
        if d.get('candidate_id')!=candidate:raise SystemExit(f'candidate mismatch: {f}')
    merge=[sys.executable,str(EVID),'merge','--version',a.version,'--candidate',candidate,'--output',a.output,*map(str,files)]
    p=run(merge)
    if p.returncode:print(p.stdout,file=sys.stderr);raise SystemExit(p.returncode)
    eval_cmd=[sys.executable,str(EVID),'evaluate','--file',a.output,'--report',a.report]
    if a.require_certified:eval_cmd.append('--require-certified')
    p=run(eval_cmd);print(p.stdout,end='')
    # Always show the evidence-derived product progress; write roadmap only when requested.
    prog=[sys.executable,str(PROGRESS),'--evidence',a.output]
    if a.write_roadmap:prog.append('--write')
    q=run(prog);print(q.stdout,end='')
    if q.returncode:raise SystemExit(q.returncode)
    if p.returncode:raise SystemExit(p.returncode)
if __name__=='__main__':raise SystemExit(main())
