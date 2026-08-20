#!/usr/bin/env python3
"""Create a candidate-bound PASS evidence fragment for one successful certification job."""
from __future__ import annotations
import argparse, subprocess, sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
VERSION=(ROOT/'VERSION').read_text().strip()
EVID=ROOT/'scripts/release-evidence.py'; CAND=ROOT/'scripts/release-candidate.py'

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('--output',required=True)
    ap.add_argument('--platform',required=True)
    ap.add_argument('--controls',nargs='+',required=True)
    ap.add_argument('--run-url',required=True)
    ap.add_argument('--commit-sha',required=True)
    ap.add_argument('--evidence',required=True)
    a=ap.parse_args()
    candidate=subprocess.check_output([sys.executable,str(CAND),'id','--root',str(ROOT)],text=True).strip()
    subprocess.run([sys.executable,str(EVID),'init','--version',VERSION,'--candidate',candidate,'--output',a.output],check=True)
    for cid in a.controls:
        subprocess.run([sys.executable,str(EVID),'record','--file',a.output,'--id',cid,'--status','pass','--platform',a.platform,
                        '--run-url',a.run_url,'--commit-sha',a.commit_sha,'--evidence',a.evidence],check=True)
    subprocess.run([sys.executable,str(EVID),'evaluate','--file',a.output],check=True)
if __name__=='__main__': raise SystemExit(main())
