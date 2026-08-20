#!/usr/bin/env python3
from __future__ import annotations
import hashlib, json, subprocess, sys, tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]; TOOL=ROOT/'scripts/release-evidence.py'; CAND='a'*64; VERSION=(ROOT/'VERSION').read_text().strip()
def run(*args):return subprocess.run([sys.executable,str(TOOL),*args],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False)
def setrow(path,cid,status,recorded,evidence='test',run_url='https://example.invalid/run/1'):
    d=json.loads(path.read_text());r=next(x for x in d['checks'] if x['id']==cid);r.update(status=status,recorded_at=recorded,evidence=evidence,run_url=run_url);path.write_text(json.dumps(d))
def main():
    with tempfile.TemporaryDirectory() as td:
        td=Path(td);a=td/'a.json';b=td/'b.json';m=td/'m.json'
        for p in (a,b):
            q=run('init','--version',VERSION,'--candidate',CAND,'--output',str(p));assert q.returncode==0,q.stdout
        setrow(a,'rust-linux','fail','2026-08-18T00:00:00+00:00')
        setrow(b,'rust-linux','pass','2026-08-19T00:00:00+00:00')
        q=run('merge','--version',VERSION,'--candidate',CAND,'--output',str(m),str(a),str(b));assert q.returncode==0,q.stdout
        d=json.loads(m.read_text());assert next(x for x in d['checks'] if x['id']=='rust-linux')['status']=='pass'
        setrow(a,'rust-linux','fail','2026-08-20T00:00:00+00:00')
        q=run('merge','--version',VERSION,'--candidate',CAND,'--output',str(m),str(a),str(b));assert q.returncode==0,q.stdout
        d=json.loads(m.read_text());assert next(x for x in d['checks'] if x['id']=='rust-linux')['status']=='fail'
        # A pass without artifact digest or workflow/evidence provenance must not satisfy certification.
        r=next(x for x in d['checks'] if x['id']=='desktop-build');r.update(status='pass',recorded_at='2026-08-19T00:00:00+00:00',evidence=None,run_url=None,artifact_sha256=None);m.write_text(json.dumps(d))
        q=run('evaluate','--file',str(m));assert q.returncode==0,q.stdout
        ev=json.loads(q.stdout);assert 'desktop-build' in ev['invalid_provenance'];assert ev['satisfied']==0
        # Local PASS requires both a result artifact digest and a runner attestation binding.
        local=td/'local.json';q=run('init','--version',VERSION,'--candidate',CAND,'--output',str(local));assert q.returncode==0
        art=td/'result.log';art.write_text('ok\n');att=td/'runner.json';att.write_text('{"runner":"test"}\n')
        q=run('record','--file',str(local),'--id','rust-linux','--status','pass','--platform','linux','--artifact',str(art),'--run-url','local://test','--commit-sha','local','--evidence','local test')
        assert q.returncode==0,q.stdout
        q=run('evaluate','--file',str(local));ev=json.loads(q.stdout);assert 'rust-linux' in ev['invalid_provenance']
        q=run('record','--file',str(local),'--id','rust-linux','--status','pass','--platform','linux','--artifact',str(art),'--runner-attestation',str(att),'--runner-attestation-ref','runner.json','--run-url','local://test','--commit-sha','local','--evidence','local test')
        assert q.returncode==0,q.stdout
        q=run('evaluate','--file',str(local));ev=json.loads(q.stdout);assert 'rust-linux' not in ev['invalid_provenance'];assert ev['satisfied']==1
        c=td/'c.json';q=run('init','--version','9.9.9','--candidate',CAND,'--output',str(c));assert q.returncode==0
        # version mismatch must fail closed
        q=run('merge','--version',VERSION,'--candidate',CAND,'--output',str(m),str(a),str(c));assert q.returncode!=0
    print('release-evidence regression tests: PASS')
if __name__=='__main__':main()
