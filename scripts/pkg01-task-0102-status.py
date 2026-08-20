#!/usr/bin/env python3
from __future__ import annotations
import json,subprocess,sys
from datetime import datetime,timezone
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
BOOT=ROOT/'scripts/pkg01-rust-bootstrap.py';POL=ROOT/'certification/pkg01-rust-bootstrap-v1.json';OUT=ROOT/'docs/PKG-01-01.02-STATUS.json'

def now():return datetime.now(timezone.utc).isoformat()
def run(args):return subprocess.run(args,cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False)
def main():
 policy=json.loads(POL.read_text())
 checks=[]
 def add(i,title,ok,detail):checks.append({'id':i,'title':title,'status':'DONE' if ok else 'PENDING','detail':detail})
 add('01.02.a','Exact toolchain policy pin',policy.get('rust_version')=='1.97.1' and policy.get('target')=='x86_64-unknown-linux-gnu','Rust 1.97.1 / x86_64-unknown-linux-gnu')
 add('01.02.b','Official archive digest pin',policy.get('sha256')=='88f28fa9af20594179f85d6df67078dfd6fa93e2f6da5e1e9b0ac4997988ca4f','Pinned SHA-256')
 text=BOOT.read_text()
 add('01.02.c','Full archive offline path','VSN_RUST_ARCHIVE' in text and 'candidate_archives' in text,'local/archive environment input supported')
 add('01.02.d','Chunk reconstruction path','reconstruct_chunks' in text and 'VSN_RUST_CHUNK_DIR' in text,'3-part chunk input supported')
 add('01.02.e','Hash trust enforcement',"sha256(archive)!=SHA" in text and "sha256(tmp)!=SHA" in text,'archive and reconstructed chunks fail closed')
 add('01.02.f','Private prefix PATH verification','tool_env' in text and "PREFIX/'bin'" in text,'private toolchain PATH injection supported')
 r=run([sys.executable,'scripts/test-pkg01-rust-bootstrap.py'])
 add('01.02.g','Archive install integration regression',r.returncode==0 and 'PASS' in r.stdout,(r.stdout or '')[-1000:])
 # Real runtime check must execute actual installed tools, not synthetic regression fixtures.
 b=run([sys.executable,'scripts/pkg01-rust-bootstrap.py'])
 real=b.returncode==0
 add('01.02.h','Real Rust 1.97.1 runtime execution',real,(b.stdout or '')[-1200:] if b.stdout else 'real Rust toolchain/archive unavailable')
 done=sum(x['status']=='DONE' for x in checks)
 out={'schema_version':1,'package_id':'PKG-01','task_id':'01.02','title':'Rust runtime components verification','updated_at':now(),'done':done,'required':8,'percent':round(done*100/8,2),'complete':done==8,'checks':checks}
 OUT.write_text(json.dumps(out,indent=2,sort_keys=True)+'\n');print(json.dumps(out,indent=2,sort_keys=True));return 0 if out['complete'] else 4
if __name__=='__main__':raise SystemExit(main())
