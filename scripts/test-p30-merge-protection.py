#!/usr/bin/env python3
from __future__ import annotations
import json,subprocess,sys,tempfile
from pathlib import Path
R=Path(__file__).resolve().parents[1];E=R/'scripts/release-evidence.py';V=(R/'VERSION').read_text().strip();C=subprocess.check_output([sys.executable,str(R/'scripts/release-candidate.py'),'id','--root',str(R)],text=True).strip()
def run(*a):return subprocess.run([sys.executable,str(E),*map(str,a)],cwd=R,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=True)
with tempfile.TemporaryDirectory() as td:
 td=Path(td);a=td/'a.json';b=td/'b.json';m=td/'m.json';run('init','--version',V,'--candidate',C,'--output',a);run('init','--version',V,'--candidate',C,'--output',b)
 da=json.loads(a.read_text());db=json.loads(b.read_text());pa=next(x for x in da['checks'] if x['id']=='rust-linux');pb=next(x for x in db['checks'] if x['id']=='rust-linux')
 pa.update({'status':'pass','run_url':'https://example.invalid/run/1','commit_sha':'abc','evidence':'test','recorded_at':'2026-08-19T00:00:00+00:00'});pb.update({'status':'blocked','recorded_at':'2026-08-20T00:00:00+00:00'});a.write_text(json.dumps(da));b.write_text(json.dumps(db));run('merge','--version',V,'--candidate',C,'--output',m,a,b);d=json.loads(m.read_text());assert next(x for x in d['checks'] if x['id']=='rust-linux')['status']=='pass'
print('p30 merge downgrade protection: PASS')
