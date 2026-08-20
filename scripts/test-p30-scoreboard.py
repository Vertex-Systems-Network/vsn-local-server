#!/usr/bin/env python3
from __future__ import annotations
import json,subprocess,sys,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
EVID=ROOT/'scripts/release-evidence.py'
SCORE=ROOT/'scripts/p30-scoreboard.py'
CAND=ROOT/'scripts/release-candidate.py'
VERSION=(ROOT/'VERSION').read_text().strip()
CANDIDATE=subprocess.check_output([sys.executable,str(CAND),'id','--root',str(ROOT)],text=True).strip()

def run(*args): return subprocess.run([sys.executable,*map(str,args)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=True).stdout

def score(path): return json.loads(run(SCORE,'--evidence',path))
with tempfile.TemporaryDirectory() as td:
    p=Path(td)/'e.json';run(EVID,'init','--version',VERSION,'--candidate',CANDIDATE,'--output',p)
    s=score(p)
    assert s['certification_satisfied']==0
    assert abs(s['p30_completion_exact_percent']-66.0)<1e-9
    assert abs(s['overall_completion_exact_percent']-98.9032)<1e-4
    # One provenance-bearing PASS must expose sub-percentage movement even though the whole headline still rounds to 99.
    run(EVID,'record','--file',p,'--id','rust-linux','--status','pass','--platform','linux','--run-url','https://example.invalid/runs/1','--commit-sha','abc123','--evidence','test/rust-linux')
    s=score(p)
    assert s['certification_satisfied']==1
    assert abs(s['p30_completion_exact_percent']-(66+34/21))<1e-4
    assert s['headline_completion_percent']==99
    # Complete every remaining control with provenance; exact score and certification must reach 100 only at 21/21.
    d=json.loads(p.read_text())
    remaining=[r['id'] for r in d['checks'] if r['id']!='rust-linux']
    for n,cid in enumerate(remaining,2):
        run(EVID,'record','--file',p,'--id',cid,'--status','pass','--platform','cross-platform','--run-url',f'https://example.invalid/runs/{n}','--commit-sha','abc123','--evidence',f'test/{cid}')
    s=score(p)
    assert s['certification_satisfied']==21 and s['stable_release_certified'] is True
    assert s['p30_completion_exact_percent']==100.0 and s['overall_completion_exact_percent']==100.0
print('p30 scoreboard regression tests: PASS')
