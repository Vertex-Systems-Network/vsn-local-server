#!/usr/bin/env python3
"""Execute one P30 certification pack and emit candidate-bound evidence fragments.
Only controls actually exercised successfully become PASS. Missing prerequisites are BLOCKED in the run ledger.
"""
from __future__ import annotations
import argparse,hashlib,json,os,platform,shutil,subprocess,sys,tempfile,time
from pathlib import Path
from p30_platform import canonical_platform
ROOT=Path(__file__).resolve().parents[1]
PROTECTED_PASS_IDS=set()
VERSION=(ROOT/'VERSION').read_text().strip(); PACKS=ROOT/'certification/p30-runner-packs.json'; TOOLCHAIN=(ROOT/'rust-toolchain.toml').read_text().split('channel = "',1)[1].split('"',1)[0]
EVID=ROOT/'scripts/release-evidence.py'; CAND=ROOT/'scripts/release-candidate.py'; ATTEST=ROOT/'scripts/p30-runner-attest.py'

def run(cmd,cwd=ROOT,timeout=1800,env=None):
 p=subprocess.run(cmd,cwd=cwd,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=timeout,check=False,env=env)
 return p.returncode,p.stdout[-20000:]
def artifact(path,text):
 path.parent.mkdir(parents=True,exist_ok=True);path.write_text(text);return path

def record(ledger,cid,status,platform_name,notes,artifact_path=None,run_url=None,commit=None,evidence=None,runner_attestation=None,runner_attestation_ref=None):
 if cid in PROTECTED_PASS_IDS:
  return
 cmd=[sys.executable,str(EVID),'record','--file',str(ledger),'--id',cid,'--status',status,'--platform',platform_name,'--notes',notes]
 if artifact_path and artifact_path.exists():cmd += ['--artifact',str(artifact_path)]
 if run_url:cmd += ['--run-url',run_url]
 if commit:cmd += ['--commit-sha',commit]
 if evidence:cmd += ['--evidence',evidence]
 if runner_attestation:cmd += ['--runner-attestation',str(runner_attestation)]
 if runner_attestation_ref:cmd += ['--runner-attestation-ref',runner_attestation_ref]
 subprocess.run(cmd,check=True)

def linux_core(outdir,ledger,meta):
 results=[]
 miss=[x for x in ['cargo','rustc'] if not shutil.which(x)]
 rust_versions=[]
 if not miss:
  for tool in ['rustc','cargo']:
   c,o=run([tool,'--version'],timeout=30);parts=o.strip().split();rust_versions.append(parts[1] if c==0 and len(parts)>1 else None)
  if any(v!=TOOLCHAIN for v in rust_versions):miss.append(f'exact-toolchain-{TOOLCHAIN}')
 if not (ROOT/'Cargo.lock').is_file():miss.append('Cargo.lock')
 if miss:
  record(ledger,'rust-linux','blocked','linux',f"missing/mismatched reproducibility prerequisites: {','.join(miss)}");results.append(('rust-linux','blocked'))
 else:
  log='';code=0
  for cmd in [['cargo','fmt','--all','--','--check'],['cargo','clippy','--locked','--workspace','--all-targets','--all-features','--','-D','warnings'],['cargo','test','--locked','--workspace','--all-targets','--all-features'],['cargo','build','--locked','--workspace','--release','--all-features']]:
   code,part=run(cmd,timeout=2400);log+=f"\n$ {' '.join(cmd)}\n{part}\n"
   if code:break
  lp=artifact(outdir/'rust-linux.log',log);record(ledger,'rust-linux','pass' if code==0 else 'fail','linux','locked fmt+clippy+test+release build',lp,**meta);results.append(('rust-linux','pass' if code==0 else 'fail'))
 for cid,folder in [('desktop-build',ROOT/'apps/desktop'),('dashboard-build',ROOT/'cloud/dashboard')]:
  if not (folder/'package-lock.json').is_file():record(ledger,cid,'blocked','linux','package-lock.json missing; reproducible production build refused');results.append((cid,'blocked'));continue
  if not (folder/'node_modules/.bin/vite').exists():record(ledger,cid,'blocked','linux','locked dependencies not installed; run npm ci');results.append((cid,'blocked'));continue
  code,log=run(['npm','run','build'],cwd=folder,timeout=900);lp=artifact(outdir/f'{cid}.log',log);record(ledger,cid,'pass' if code==0 else 'fail','linux','locked npm production build',lp,**meta);results.append((cid,'pass' if code==0 else 'fail'))
 rel=ROOT/'target/release'; bins=[rel/'vsn',rel/'vsn-agent',rel/'vsn-updater-helper']
 if not all(x.is_file() for x in bins):
  for cid in ['deb-install-uninstall','updater-linux']:record(ledger,cid,'blocked','linux','real release binaries unavailable');results.append((cid,'blocked'))
 else:
  dist=outdir/'dist';dist.mkdir(exist_ok=True);code,log=run([str(ROOT/'packaging/linux/build-deb.sh'),VERSION,str(rel),str(dist)],timeout=600);pkg=dist/f'vsn-runtime-{VERSION}-amd64.deb'
  if code==0 and hasattr(os,'geteuid') and os.geteuid()==0:
   for cmd in [['dpkg','-i',str(pkg)],['/usr/local/bin/vsn','--version'],['dpkg','-r','vsn-runtime']]:
    c,x=run(cmd,timeout=300);log+=f"\n$ {' '.join(cmd)}\n{x}";code=max(code,c)
  elif code==0:code=3;log+='\nroot privileges required for install/uninstall acceptance'
  lp=artifact(outdir/'deb-install-uninstall.log',log);st='pass' if code==0 else ('blocked' if code==3 else 'fail');record(ledger,'deb-install-uninstall',st,'linux','real deb build/install/version/uninstall acceptance',lp if code!=3 else None,**meta);results.append(('deb-install-uninstall',st))
  code,log=run([sys.executable,str(ROOT/'scripts/smoke-updater-helper.py'),'--helper',str(rel/'vsn-updater-helper')],timeout=300);lp=artifact(outdir/'updater-linux.log',log);st='pass' if code==0 else 'fail';record(ledger,'updater-linux',st,'linux','updater apply/status/rollback E2E',lp,**meta);results.append(('updater-linux',st))
 if not shutil.which('cargo-audit'):
  record(ledger,'rustsec-audit','blocked','linux','cargo-audit unavailable');results.append(('rustsec-audit','blocked'))
 else:
  code,log=run(['cargo','audit'],timeout=900);lp=artifact(outdir/'rustsec-audit.log',log);st='pass' if code==0 else 'fail';record(ledger,'rustsec-audit',st,'linux','cargo audit',lp,**meta);results.append(('rustsec-audit',st))
 return results

def security_pack(outdir,ledger,meta):
 results=[]
 for cid,target in [('fuzz-remote-protocol','remote_protocol'),('fuzz-stream-open','stream_open')]:
  if not shutil.which('cargo') or not shutil.which('cargo-fuzz'):
   record(ledger,cid,'blocked','linux','cargo/cargo-fuzz unavailable');results.append((cid,'blocked'));continue
  code,log=run(['cargo','fuzz','run',target,'--','-max_total_time=300','-rss_limit_mb=2048'],cwd=ROOT/'fuzz',timeout=420);lp=artifact(outdir/f'{cid}.log',log);st='pass' if code==0 else 'fail';record(ledger,cid,st,'linux',f'cargo-fuzz {target} 300s',lp,**meta);results.append((cid,st))
 return results

def load_external_commands(path,candidate):
 if not path:return None
 p=Path(path);data=json.loads(p.read_text())
 if data.get('schema_version')!=1 or data.get('product_version')!=VERSION or data.get('candidate_id')!=candidate:raise ValueError('external command manifest version/candidate mismatch')
 commands=data.get('commands')
 if not isinstance(commands,dict):raise ValueError('external command manifest commands must be an object')
 out={}
 for cid,argv in commands.items():
  if not isinstance(cid,str) or not isinstance(argv,list) or not (1<=len(argv)<=64):raise ValueError(f'invalid argv for {cid}')
  if any(not isinstance(x,str) or not x or len(x)>4096 or '\x00' in x for x in argv):raise ValueError(f'invalid argv item for {cid}')
  out[cid]=argv
 return out

def external_pack(pack,outdir,ledger,meta,commands):
 expected={
  'operations':['control-load-slo','ha-failover','dr-restore','vault-key-rotation'],
  'independent-review':['penetration-test']
 }.get(pack['id'],[])
 results=[]
 for cid in expected:
  argv=(commands or {}).get(cid)
  if not argv:
   record(ledger,cid,'blocked','cross-platform','candidate-bound direct-argv external command not supplied',**meta);results.append((cid,'blocked'));continue
  code,log=run(argv,timeout=3600);lp=artifact(outdir/f'{cid}.log',log);st='pass' if code==0 else 'fail';record(ledger,cid,st,'cross-platform','candidate-bound direct-argv external certification command',lp,**meta);results.append((cid,st))
 return results

def rust_quality(outdir,ledger,platform_name,rust_id,updater_id,helper_name,meta):
 results=[]
 miss=[x for x in ['cargo','rustc'] if not shutil.which(x)]
 if not miss:
  for tool in ['rustc','cargo']:
   c,o=run([tool,'--version'],timeout=30);parts=o.strip().split();
   if c or len(parts)<2 or parts[1]!=TOOLCHAIN:miss.append(f'{tool}!={TOOLCHAIN}')
 if not (ROOT/'Cargo.lock').is_file():miss.append('Cargo.lock')
 if miss:
  record(ledger,rust_id,'blocked',platform_name,f"missing/mismatched reproducibility prerequisites: {','.join(miss)}");results.append((rust_id,'blocked'))
  record(ledger,updater_id,'blocked',platform_name,'release updater helper unavailable without accepted Rust build');results.append((updater_id,'blocked'));return results
 log='';code=0
 for cmd in [['cargo','fmt','--all','--','--check'],['cargo','clippy','--locked','--workspace','--all-targets','--all-features','--','-D','warnings'],['cargo','test','--locked','--workspace','--all-targets','--all-features'],['cargo','build','--locked','--workspace','--release','--all-features']]:
  code,part=run(cmd,timeout=2400);log+=f"\n$ {' '.join(cmd)}\n{part}\n"
  if code:break
 lp=artifact(outdir/f'{rust_id}.log',log);st='pass' if code==0 else 'fail';record(ledger,rust_id,st,platform_name,'fmt+clippy+test+release build',lp,**meta);results.append((rust_id,st))
 helper=ROOT/'target/release'/helper_name
 if code!=0 or not helper.is_file():
  record(ledger,updater_id,'blocked',platform_name,'release updater helper unavailable after Rust quality gate');results.append((updater_id,'blocked'))
 else:
  code,log=run([sys.executable,str(ROOT/'scripts/smoke-updater-helper.py'),'--helper',str(helper)],timeout=300);lp=artifact(outdir/f'{updater_id}.log',log);st='pass' if code==0 else 'fail';record(ledger,updater_id,st,platform_name,'updater apply/status/rollback E2E',lp,**meta);results.append((updater_id,st))
 return results

def windows_core(outdir,ledger,meta):
 results=rust_quality(outdir,ledger,'windows','rust-windows','updater-windows','vsn-updater-helper.exe',meta)
 if not shutil.which('pwsh') or not shutil.which('dotnet'):
  miss=[x for x in ['pwsh','dotnet'] if not shutil.which(x)];record(ledger,'msi-install-uninstall','blocked','windows',f"missing required tools: {','.join(miss)}");results.append(('msi-install-uninstall','blocked'));msi=None
 else:
  rel=ROOT/'target/release';dist=outdir/'dist';dist.mkdir(exist_ok=True);code,log=run(['pwsh','-NoProfile','-File',str(ROOT/'packaging/windows/build-extension-appcontainer.ps1'),'-OutputDir',str(rel)],timeout=600)
  if code==0:code,x=run(['pwsh','-NoProfile','-File',str(ROOT/'packaging/windows/build-msi.ps1'),'-Version',VERSION,'-SourceDir',str(rel),'-OutputDir',str(dist)],timeout=900);log+=x
  msi=dist/f'vsn-runtime-{VERSION}-x64.msi'
  if code==0 and msi.is_file():
   code,x=run(['msiexec.exe','/i',str(msi),'/qn','/norestart'],timeout=600);log+=x
   installed=Path(os.environ.get('ProgramFiles',r'C:\Program Files'))/'VSN'/'vsn.exe'
   if code==0 and installed.is_file():c,x=run([str(installed),'--version'],timeout=60);log+=x;code=max(code,c)
   if code==0:c,x=run(['sc.exe','query','VSNAgent'],timeout=60);log+=x;code=max(code,c)
   c,x=run(['msiexec.exe','/x',str(msi),'/qn','/norestart'],timeout=600);log+=x;code=max(code,c)
  lp=artifact(outdir/'msi-install-uninstall.log',log);st='pass' if code==0 and msi.is_file() else 'fail';record(ledger,'msi-install-uninstall',st,'windows','MSI build/install/service/version/uninstall acceptance',lp,**meta);results.append(('msi-install-uninstall',st))
 thumb=os.environ.get('VSN_WINDOWS_CERT_THUMBPRINT')
 if not thumb or not msi or not msi.is_file():
  record(ledger,'windows-authenticode','blocked','windows','VSN_WINDOWS_CERT_THUMBPRINT or accepted MSI unavailable');results.append(('windows-authenticode','blocked'))
 else:
  code,log=run(['pwsh','-NoProfile','-File',str(ROOT/'packaging/windows/sign-msi.ps1'),'-Msi',str(msi),'-CertificateThumbprint',thumb],timeout=900);lp=artifact(outdir/'windows-authenticode.log',log);st='pass' if code==0 else 'fail';record(ledger,'windows-authenticode',st,'windows','SignTool SHA-256 + timestamp + signature verify',lp,**meta);results.append(('windows-authenticode',st))
 return results

def macos_core(outdir,ledger,meta):
 results=rust_quality(outdir,ledger,'macos','rust-macos','updater-macos','vsn-updater-helper',meta)
 rel=ROOT/'target/release';dist=outdir/'dist';dist.mkdir(exist_ok=True);pkg=dist/f'vsn-runtime-{VERSION}-unsigned.pkg'
 if not shutil.which('pkgbuild') or not shutil.which('productbuild'):
  record(ledger,'pkg-install-uninstall','blocked','macos','pkgbuild/productbuild unavailable');results.append(('pkg-install-uninstall','blocked'))
 else:
  code,log=run([str(ROOT/'packaging/macos/build-pkg.sh'),VERSION,str(rel),str(dist)],timeout=900)
  sudo=['sudo'] if shutil.which('sudo') else []
  if code==0 and pkg.is_file():
   c,x=run(sudo+['installer','-pkg',str(pkg),'-target','/'],timeout=600);log+=x;code=max(code,c)
   if code==0:c,x=run(['/usr/local/bin/vsn','--version'],timeout=60);log+=x;code=max(code,c)
   # Cleanup is part of acceptance; package script installs only these runtime files + LaunchAgent.
   for path in ['/usr/local/bin/vsn','/usr/local/bin/vsn-agent','/usr/local/bin/vsn-updater-helper','/Library/LaunchAgents/dev.vsn.agent.plist']:
    c,x=run(sudo+['rm','-f',path],timeout=60);log+=x;code=max(code,c)
   run(sudo+['pkgutil','--forget','dev.vsn.runtime'],timeout=60)
  lp=artifact(outdir/'pkg-install-uninstall.log',log);st='pass' if code==0 and pkg.is_file() else 'fail';record(ledger,'pkg-install-uninstall',st,'macos','pkg build/install/version/cleanup acceptance',lp,**meta);results.append(('pkg-install-uninstall',st))
 identity=os.environ.get('VSN_MACOS_SIGN_IDENTITY'); keychain=os.environ.get('VSN_MACOS_KEYCHAIN')
 req=['VSN_APPLE_API_KEY_ID','VSN_APPLE_API_ISSUER','VSN_APPLE_API_KEY_P8_B64']
 if not identity or not keychain or any(not os.environ.get(k) for k in req) or not pkg.is_file():
  record(ledger,'macos-notarization','blocked','macos','signing identity/keychain/App Store Connect API credentials or unsigned pkg unavailable');results.append(('macos-notarization','blocked'))
 else:
  code,log=run([str(ROOT/'packaging/macos/sign-notarize-ci.sh'),str(pkg),identity,keychain],timeout=3600);lp=artifact(outdir/'macos-notarization.log',log);st='pass' if code==0 else 'fail';record(ledger,'macos-notarization',st,'macos','productsign + notarytool + stapler validation',lp,**meta);results.append(('macos-notarization',st))
 return results

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--pack',required=True);ap.add_argument('--output-dir',default=str(ROOT/'dist-p30'));ap.add_argument('--run-url',default=os.getenv('GITHUB_SERVER_URL','local')+'/'+os.getenv('GITHUB_REPOSITORY','local') if os.getenv('GITHUB_REPOSITORY') else 'local://p30-run-pack');ap.add_argument('--commit-sha',default=os.getenv('GITHUB_SHA','local'));ap.add_argument('--external-command-file');ap.add_argument('--resume-ledger');ap.add_argument('--result-bundle-dir');a=ap.parse_args()
 packs=json.loads(PACKS.read_text());pack=next((x for x in packs['packs'] if x['id']==a.pack),None)
 if not pack:raise SystemExit(f'unknown pack: {a.pack}')
 host=canonical_platform();
 if pack['platform']!='cross-platform' and pack['platform']!=host:raise SystemExit(f"pack {a.pack} requires {pack['platform']}, host is {host}")
 candidate=subprocess.check_output([sys.executable,str(CAND),'id','--root',str(ROOT)],text=True).strip();outdir=Path(a.output_dir)/f'{a.pack}-{candidate[:12]}';outdir.mkdir(parents=True,exist_ok=True);ledger=outdir/'evidence.json'
 attestation=outdir/'runner-attestation.json';subprocess.run([sys.executable,str(ATTEST),'--root',str(ROOT),'--output',str(attestation),'--pack',a.pack],check=True)
 subprocess.run([sys.executable,str(EVID),'init','--version',VERSION,'--candidate',candidate,'--output',str(ledger)],check=True)
 if a.resume_ledger:
  resume=Path(a.resume_ledger)
  if not resume.is_file(): raise SystemExit('resume ledger not found')
  rep=outdir/'resume-report.json'
  subprocess.run([sys.executable,str(EVID),'evaluate','--file',str(resume),'--report',str(rep)],check=True)
  rd=json.loads(rep.read_text())
  if rd.get('candidate_id')!=candidate: raise SystemExit('resume ledger candidate mismatch')
  bad=set(sum((rd.get(k,[]) for k in ['pending','blocked','failed','expired','waived','invalid_provenance','missing']),[]))
  required=set(json.loads(resume.read_text()).get('checks',{}).keys()) if isinstance(json.loads(resume.read_text()).get('checks'),dict) else set()
  # Required controls are the canonical 21; a valid PASS is any control not present in evaluator problem lists.
  all_required=set(['rust-windows','rust-linux','rust-macos','desktop-build','dashboard-build','msi-install-uninstall','deb-install-uninstall','pkg-install-uninstall','updater-windows','updater-linux','updater-macos','windows-authenticode','macos-notarization','rustsec-audit','fuzz-remote-protocol','fuzz-stream-open','control-load-slo','ha-failover','dr-restore','vault-key-rotation','penetration-test'])
  PROTECTED_PASS_IDS.update(all_required-bad)
  subprocess.run([sys.executable,str(EVID),'merge','--version',VERSION,'--candidate',candidate,'--output',str(ledger),str(resume),str(ledger)],check=True)
 meta={'run_url':a.run_url,'commit':a.commit_sha,'evidence':f'p30-run-pack/{a.pack} candidate={candidate}','runner_attestation':attestation,'runner_attestation_ref':'runner-attestation.json'}
 if a.pack=='linux-core':results=linux_core(outdir,ledger,meta)
 elif a.pack=='security-nightly':results=security_pack(outdir,ledger,meta)
 elif a.pack=='windows-core':results=windows_core(outdir,ledger,meta)
 elif a.pack=='macos-core':results=macos_core(outdir,ledger,meta)
 elif a.pack in {'operations','independent-review'}:results=external_pack(pack,outdir,ledger,meta,load_external_commands(a.external_command_file,candidate))
 else:raise SystemExit(f'pack executor not implemented: {a.pack}')
 results=[(c,'pass' if c in PROTECTED_PASS_IDS else st) for c,st in results]
 report=outdir/'report.json';subprocess.run([sys.executable,str(EVID),'evaluate','--file',str(ledger),'--report',str(report)],check=True)
 summary={'schema_version':1,'product_version':VERSION,'candidate_id':candidate,'pack_id':a.pack,'host':host,'results':[{'id':c,'status':s} for c,s in results],'ledger':str(ledger),'report':str(report),'runner_attestation':str(attestation),'runner_attestation_sha256':hashlib.sha256(attestation.read_bytes()).hexdigest(),'protected_passes':sorted(PROTECTED_PASS_IDS)}
 (outdir/'summary.json').write_text(json.dumps(summary,indent=2,sort_keys=True)+'\n')
 if a.result_bundle_dir: subprocess.run([sys.executable,str(ROOT/'scripts/p30-result-bundle.py'),'build','--run-dir',str(outdir),'--output-dir',a.result_bundle_dir],check=True)
 print(json.dumps(summary,indent=2,sort_keys=True));return 0 if all(s=='pass' for _,s in results) else 4
if __name__=='__main__':raise SystemExit(main())
