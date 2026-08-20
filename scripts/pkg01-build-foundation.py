#!/usr/bin/env python3
from __future__ import annotations
import argparse,json,os,shutil,subprocess,sys
from datetime import datetime,timezone
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];SPEC=json.loads((ROOT/'certification/pkg01-build-foundation-v1.json').read_text());STATUS=ROOT/'docs/PKG-01-BUILD-FOUNDATION-STATUS.json'

def now():return datetime.now(timezone.utc).isoformat()
def env():
 e=os.environ.copy();p=ROOT/'.pkg01-toolchain/rust-1.97.1/bin'
 if p.is_dir():e['PATH']=str(p)+os.pathsep+e.get('PATH','')
 return e
def which(c):return shutil.which(c,path=env().get('PATH'))
def ver(c):
 p=which(c)
 if not p:return None
 r=subprocess.run([p,'--version'],cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,env=env(),check=False);return (r.stdout or '').strip()
def jvalid(p):
 try:json.loads(p.read_text());return True
 except:return False

def probe():
 rows=[]
 def add(i,s,d):
  t=next(x for x in SPEC['tasks'] if x['id']==i);rows.append({**t,'status':s,'detail':d})
 txt=(ROOT/'rust-toolchain.toml').read_text();add('01.01','DONE' if 'channel = "1.97.1"' in txt and 'rustfmt' in txt and 'clippy' in txt else 'BLOCKED','Rust 1.97.1 + rustfmt + clippy pinned')
 rv,cv=ver('rustc'),ver('cargo');ok=rv and cv and '1.97.1' in rv and '1.97.1' in cv and which('rustfmt') and which('cargo-clippy')
 add('01.02','DONE' if ok else 'IN_PROGRESS',f'rustc={rv or "missing"}; cargo={cv or "missing"}; rustfmt={bool(which("rustfmt"))}; clippy={bool(which("cargo-clippy"))}')
 c=ROOT/'Cargo.lock';add('01.03','DONE' if c.is_file() else 'BLOCKED','Cargo.lock present' if c.is_file() else 'requires Rust toolchain + dependency resolution');add('01.04','DONE' if c.is_file() else 'BLOCKED','Cargo.lock present' if c.is_file() else 'Cargo.lock missing')
 for i,name in [('01.05','cargo fetch --locked'),('01.06','cargo fmt --check'),('01.07','cargo clippy --locked'),('01.08','cargo test --locked')]:add(i,'BLOCKED',name+' requires Rust toolchain/Cargo.lock')
 for i,p in [('01.09','target/release/vsn-agent'),('01.10','target/release/vsn'),('01.11','target/release/vsn-updater-helper')]:add(i,'DONE' if (ROOT/p).is_file() else 'BLOCKED',p+(' present' if (ROOT/p).is_file() else ' not built'))
 dl=ROOT/'apps/desktop/package-lock.json';gl=ROOT/'cloud/dashboard/package-lock.json'
 add('01.12','DONE' if dl.is_file() and jvalid(dl) else 'BLOCKED','desktop dependency lock '+('present' if dl.is_file() else 'missing'));add('01.13','DONE' if dl.is_file() and jvalid(dl) else 'BLOCKED','desktop package-lock '+('valid' if dl.is_file() and jvalid(dl) else 'missing/invalid'))
 add('01.14','DONE' if (ROOT/'apps/desktop/node_modules/.bin/vite').is_file() else 'BLOCKED','desktop npm ci not complete');add('01.15','DONE' if (ROOT/'apps/desktop/dist').is_dir() else 'BLOCKED','desktop dist missing')
 add('01.16','DONE' if gl.is_file() and jvalid(gl) else 'BLOCKED','dashboard dependency lock '+('present' if gl.is_file() else 'missing'));add('01.17','DONE' if gl.is_file() and jvalid(gl) else 'BLOCKED','dashboard package-lock '+('valid' if gl.is_file() and jvalid(gl) else 'missing/invalid'))
 add('01.18','DONE' if (ROOT/'cloud/dashboard/node_modules/.bin/vite').is_file() else 'BLOCKED','dashboard npm ci not complete');add('01.19','DONE' if (ROOT/'cloud/dashboard/dist').is_dir() else 'BLOCKED','dashboard dist missing')
 add('01.20','DONE' if (ROOT/'docs/pkg01-build-artifacts.json').is_file() else 'BLOCKED','artifact manifest not generated');add('01.21','DONE' if (ROOT/'docs/pkg01-reproducibility-report.json').is_file() else 'BLOCKED','fresh reproducibility not passed')
 prior=sum(x['status']=='DONE' for x in rows);add('01.22','DONE' if prior==21 else 'BLOCKED',f'{prior}/21 prerequisites DONE')
 done=sum(x['status']=='DONE' for x in rows);out={'schema_version':1,'package_id':'PKG-01','title':SPEC['title'],'updated_at':now(),'done':done,'required':22,'percent':round(done*100/22,2),'complete':done==22,'tasks':rows};STATUS.write_text(json.dumps(out,indent=2,sort_keys=True)+'\n');return out

def main():
 ap=argparse.ArgumentParser();sp=ap.add_subparsers(dest='cmd',required=True);sp.add_parser('status');p=sp.add_parser('run-next');p.add_argument('--allow-network',action='store_true');a=ap.parse_args()
 if a.cmd=='status':print(json.dumps(probe(),indent=2,sort_keys=True));return 0
 s=probe();n=next((x for x in s['tasks'] if x['status']!='DONE'),None)
 if not n:print(json.dumps(s,indent=2));return 0
 if n['id']=='01.02':
  cmd=[sys.executable,str(ROOT/'scripts/pkg01-rust-bootstrap.py')]+(['--allow-network'] if a.allow_network else []);r=subprocess.run(cmd,cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False,timeout=300);s=probe();print(json.dumps({'task':'01.02','bootstrap_rc':r.returncode,'bootstrap_output':(r.stdout or '')[-5000:],'status':s},indent=2,sort_keys=True));return 0 if next(x for x in s['tasks'] if x['id']=='01.02')['status']=='DONE' else 4
 print(json.dumps({'error':f'next task {n["id"]} execution not reached because sequential gate requires 01.02 DONE','status':s},indent=2));return 4
if __name__=='__main__':raise SystemExit(main())
