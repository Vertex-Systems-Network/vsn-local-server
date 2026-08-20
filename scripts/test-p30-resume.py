#!/usr/bin/env python3
from __future__ import annotations
import json,subprocess,sys,tempfile
from pathlib import Path
R=Path(__file__).resolve().parents[1];V=(R/'VERSION').read_text().strip();E=R/'scripts/release-evidence.py';C=R/'scripts/release-candidate.py';P=R/'scripts/p30-run-pack.py'
def run(a,**kw):return subprocess.run(a,cwd=R,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False,**kw)
candidate=subprocess.check_output([sys.executable,str(C),'id','--root',str(R)],text=True).strip()
with tempfile.TemporaryDirectory(prefix='vsn-p30-resume-') as td:
 t=Path(td);resume=t/'resume.json';artifact=t/'rust.log';artifact.write_text('synthetic hosted regression evidence\n')
 subprocess.run([sys.executable,str(E),'init','--version',V,'--candidate',candidate,'--output',str(resume)],check=True)
 subprocess.run([sys.executable,str(E),'record','--file',str(resume),'--id','rust-linux','--status','pass','--platform','linux','--evidence','resume-regression','--artifact',str(artifact),'--run-url','https://github.com/example/vsn/actions/runs/1','--commit-sha','0123456789012345678901234567890123456789'],check=True)
 p=run([sys.executable,str(P),'--pack','linux-core','--resume-ledger',str(resume),'--output-dir',str(t/'out')])
 ledgers=list((t/'out').rglob('evidence.json'));assert len(ledgers)==1,p.stdout
 report=t/'eval.json';subprocess.run([sys.executable,str(E),'evaluate','--file',str(ledgers[0]),'--report',str(report)],check=True)
 d=json.loads(report.read_text());assert d['satisfied']==1,d;assert 'rust-linux' not in d['blocked'],d
 summary=json.loads(next((t/'out').rglob('summary.json')).read_text());rows={x['id']:x['status'] for x in summary['results']};assert rows['rust-linux']=='pass',rows;assert 'rust-linux' in summary['protected_passes'],summary
print('p30 resume regression: PASS')
