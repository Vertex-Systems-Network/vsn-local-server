#!/usr/bin/env python3
from __future__ import annotations
import argparse,hashlib,json,os,platform,shutil,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
# Make rustup-installed and offline standalone toolchains discoverable without requiring shell profile reload.
_path_parts=[]
for _p in [Path(os.environ.get('CARGO_HOME',str(Path.home()/'.cargo')))/'bin',Path(os.environ.get('VSN_PKG01_RUST_PREFIX','/opt/vsn-rust-1.97.1'))/'bin']:
 if _p.is_dir():_path_parts.append(str(_p))
if _path_parts:os.environ['PATH']=os.pathsep.join(_path_parts+[os.environ.get('PATH','')])
SPEC=ROOT/'certification/pkg01-linux-core-v1.json'
STATUS=ROOT/'docs/PKG-01-STATUS.json'
EVID=ROOT/'scripts/release-evidence.py'
CAND=ROOT/'scripts/release-candidate.py'

def run(args,check=True,env=None):
 p=subprocess.run(args,cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False,env=env)
 if check and p.returncode: raise RuntimeError(p.stdout[-10000:])
 return p

def version_of(cmd):
 if not shutil.which(cmd): return None
 p=run([cmd,'--version'],check=False);return (p.stdout or '').strip().splitlines()[0] if p.stdout else None

def candidate(): return run([sys.executable,str(CAND),'id','--root',str(ROOT)]).stdout.strip()

def eval_current():
 p=run([sys.executable,str(EVID),'evaluate','--file',str(ROOT/'docs/release-evidence-current.json')]);return json.loads(p.stdout)

def valid_passes(spec):
 d=json.loads((ROOT/'docs/release-evidence-current.json').read_text());r=eval_current();bad=set(sum((r.get(k,[]) for k in ['pending','blocked','failed','expired','waived','invalid_provenance','missing']),[]))
 rows={x.get('id'):x for x in d.get('checks',[]) if isinstance(x,dict)}
 return [cid for cid in spec['controls'] if cid in rows and rows[cid].get('status')=='pass' and cid not in bad]

def status(write=False,last_bundle=None,last_sha=None):
 spec=json.loads(SPEC.read_text());toolchain=spec['required_rust_toolchain'];blockers=[]
 tools={t:{'path':shutil.which(t),'version':version_of(t)} for t in spec['required_tools']}
 for t,x in tools.items():
  if not x['path']:blockers.append(f'missing tool: {t}')
 rustc_v=(tools.get('rustc',{}).get('version') or '').split()
 cargo_v=(tools.get('cargo',{}).get('version') or '').split()
 if rustc_v and len(rustc_v)>1 and rustc_v[1]!=toolchain:blockers.append(f'rustc {rustc_v[1]} != {toolchain}')
 if cargo_v and len(cargo_v)>1 and cargo_v[1]!=toolchain:blockers.append(f'cargo {cargo_v[1]} != {toolchain}')
 locks={rel:(ROOT/rel).is_file() for rel in spec['required_source_files']}
 for rel,ok in locks.items():
  if not ok:blockers.append(f'missing reproducibility lock: {rel}')
 deps={'desktop_vite':(ROOT/'apps/desktop/node_modules/.bin/vite').is_file(),'dashboard_vite':(ROOT/'cloud/dashboard/node_modules/.bin/vite').is_file()}
 for name,ok in deps.items():
  if not ok:blockers.append(f'locked dependencies not installed: {name}')
 if hasattr(os,'geteuid') and os.geteuid()!=0 and not shutil.which('sudo'):blockers.append('root or sudo required for real deb install/uninstall acceptance')
 passes=valid_passes(spec);complete=len(passes)==len(spec['controls'])
 out={'schema_version':1,'package_id':'PKG-01','product_version':(ROOT/'VERSION').read_text().strip(),'candidate_id':candidate(),'host':platform.system().lower(),'controls':spec['controls'],'prerequisites':{'required_rust_toolchain':toolchain,'tools':tools,'lockfiles':locks,'dependencies':deps,'root':(os.geteuid()==0 if hasattr(os,'geteuid') else None)},'ready':not blockers,'complete':complete,'valid_passes':len(passes),'required_passes':len(spec['controls']),'blockers':blockers,'last_result_bundle':last_bundle,'last_result_sha256':last_sha}
 if write: STATUS.write_text(json.dumps(out,indent=2,sort_keys=True)+'\n')
 return out

def cleanup_generated():
 for p in [ROOT/'target',ROOT/'apps/desktop/node_modules',ROOT/'cloud/dashboard/node_modules',ROOT/'apps/desktop/dist',ROOT/'cloud/dashboard/dist']:
  if p.exists():shutil.rmtree(p,ignore_errors=True)

def freeze_after_source_changes():
 report=eval_current()
 if int(report.get('satisfied') or 0)>0: raise RuntimeError('refusing candidate refreeze after source changes while certification PASS evidence exists')
 run([sys.executable,str(CAND),'write','--root',str(ROOT),'--output','docs/release-candidate-current.json'])
 cand=candidate();ver=(ROOT/'VERSION').read_text().strip()
 run([sys.executable,str(EVID),'init','--version',ver,'--candidate',cand,'--output',str(ROOT/'docs/release-evidence-current.json')])
 journal=ROOT/'docs/p30-evidence-journal.json'
 if journal.exists():
  j=json.loads(journal.read_text());j['product_version']=ver;j['candidate_id']=cand;j['updated_at']='1970-01-01T00:00:00+00:00';j['entries']=[];j['operations']=[];j['checkpoints']=[];journal.write_text(json.dumps(j,indent=2,sort_keys=True)+'\n')
 run([sys.executable,str(ROOT/'scripts/p30-sync-status.py'),'write'])
 return cand

def prepare(allow_network):
 before=candidate();env=os.environ.copy();env['VSN_PKG01_ALLOW_NETWORK']='1' if allow_network else '0'
 p=run([str(ROOT/'scripts/pkg01-bootstrap-linux.sh')],check=False,env=env)
 if p.returncode: raise RuntimeError(p.stdout[-10000:])
 cargo_bin=Path(os.environ.get('CARGO_HOME',str(Path.home()/'.cargo')))/'bin'
 if cargo_bin.is_dir():os.environ['PATH']=str(cargo_bin)+os.pathsep+os.environ.get('PATH','')
 prefix_bin=Path(os.environ.get('VSN_PKG01_RUST_PREFIX',f'/opt/vsn-rust-{json.loads(SPEC.read_text())["required_rust_toolchain"]}'))/'bin'
 if prefix_bin.is_dir():os.environ['PATH']=str(prefix_bin)+os.pathsep+os.environ.get('PATH','')
 after=candidate()
 if after!=before: freeze_after_source_changes()
 return status(write=True)

def execute(do_import=True):
 st=status(write=True)
 if not st['ready']: raise RuntimeError('PKG-01 prerequisites are not ready:\n- '+'\n- '.join(st['blockers']))
 runs=ROOT/'dist-pkg01/runs';results=ROOT/'dist-pkg01/results';shutil.rmtree(ROOT/'dist-pkg01',ignore_errors=True);runs.mkdir(parents=True);results.mkdir(parents=True)
 p=run([sys.executable,str(ROOT/'scripts/p30-run-pack.py'),'--pack','linux-core','--output-dir',str(runs),'--result-bundle-dir',str(results)],check=False)
 zips=sorted(results.glob('*.zip'))
 if not zips: raise RuntimeError('PKG-01 produced no result bundle:\n'+p.stdout[-5000:])
 bundle=zips[-1];sha_file=bundle.with_suffix('.zip.sha256')
 run([sys.executable,str(ROOT/'scripts/p30-result-bundle.py'),'verify',str(bundle),'--sha256',str(sha_file)])
 run_dirs=sorted(x for x in runs.iterdir() if x.is_dir());summary=json.loads((run_dirs[-1]/'summary.json').read_text()) if run_dirs else {}
 rows={x['id']:x['status'] for x in summary.get('results',[])};spec=json.loads(SPEC.read_text())
 if any(rows.get(cid)!='pass' for cid in spec['controls']):
  status(write=True,last_bundle=str(bundle),last_sha=hashlib.sha256(bundle.read_bytes()).hexdigest())
  raise RuntimeError('PKG-01 not complete; controls are '+json.dumps(rows,sort_keys=True))
 if do_import:
  run([sys.executable,str(ROOT/'scripts/p30-evidence-governance.py'),'import','--bundle',str(bundle),'--sha256',str(sha_file)])
  run([sys.executable,str(ROOT/'scripts/p30-evidence-governance.py'),'verify'])
 cleanup_generated()
 if do_import:run([sys.executable,str(ROOT/'scripts/release-gate.py')])
 final=status(write=True,last_bundle=str(bundle),last_sha=hashlib.sha256(bundle.read_bytes()).hexdigest())
 if not final['complete']: raise RuntimeError(f'PKG-01 evidence import incomplete: {final["valid_passes"]}/6')
 return final

def main():
 ap=argparse.ArgumentParser();sp=ap.add_subparsers(dest='cmd',required=True)
 sp.add_parser('status');p=sp.add_parser('prepare');p.add_argument('--allow-network',action='store_true');p=sp.add_parser('execute');p.add_argument('--no-import',action='store_true');p=sp.add_parser('all');p.add_argument('--allow-network',action='store_true');p.add_argument('--no-import',action='store_true')
 a=ap.parse_args()
 try:
  if a.cmd=='status':out=status(write=True)
  elif a.cmd=='prepare':out=prepare(a.allow_network)
  elif a.cmd=='execute':out=execute(not a.no_import)
  else:
   prepare(a.allow_network);out=execute(not a.no_import)
  print(json.dumps(out,indent=2,sort_keys=True));return 0
 except Exception as e:
  st=status(write=True);print(json.dumps({'ok':False,'error':str(e),'status':st},indent=2,sort_keys=True));return 4
if __name__=='__main__':raise SystemExit(main())
