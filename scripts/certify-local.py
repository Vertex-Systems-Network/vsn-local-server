#!/usr/bin/env python3
"""Run only evidence controls this host can genuinely exercise. Missing prerequisites become BLOCKED, never PASS."""
from __future__ import annotations
import argparse,json,os,platform,shutil,subprocess,sys,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]; VERSION=(ROOT/'VERSION').read_text().strip(); EVID=ROOT/'scripts/release-evidence.py'; CAND=ROOT/'scripts/release-candidate.py'; ATTEST=ROOT/'scripts/p30-runner-attest.py'; RUNNER_ATTESTATION=None
def run(cmd,cwd=ROOT,timeout=1800):
    p=subprocess.run(cmd,cwd=cwd,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=timeout,check=False);return p.returncode,p.stdout[-12000:]
def rec(path,cid,status,platform_name,notes,evidence=None,artifact=None):
    cmd=[sys.executable,str(EVID),'record','--file',str(path),'--id',cid,'--status',status,'--platform',platform_name,'--notes',notes,'--run-url','local://certify-local','--commit-sha','local','--evidence',evidence or 'certify-local']
    if artifact:cmd+=['--artifact',str(artifact)]
    if RUNNER_ATTESTATION and Path(RUNNER_ATTESTATION).is_file():cmd+=['--runner-attestation',str(RUNNER_ATTESTATION),'--runner-attestation-ref','runner-attestation.json']
    subprocess.run(cmd,check=True)
def missing(*tools):return [x for x in tools if not shutil.which(x)]
def main():
    ap=argparse.ArgumentParser();ap.add_argument('--output',default=str(ROOT/f'docs/release-evidence-{VERSION}-local.json'));ap.add_argument('--control-url');ap.add_argument('--skip-destructive',action='store_true');a=ap.parse_args();out=Path(a.output)
    global RUNNER_ATTESTATION
    candidate=subprocess.check_output([sys.executable,str(CAND),'id','--root',str(ROOT)],text=True).strip(); subprocess.run([sys.executable,str(EVID),'init','--version',VERSION,'--candidate',candidate,'--output',str(out)],check=True)
    RUNNER_ATTESTATION=ROOT/f'docs/release-runner-attestation-{VERSION}-local.json';subprocess.run([sys.executable,str(ATTEST),'--root',str(ROOT),'--output',str(RUNNER_ATTESTATION),'--pack','local-certifier'],check=True)
    host=platform.system().lower()
    # Rust Linux quality gate
    if host=='linux':
        miss=missing('cargo','rustc')
        if miss:rec(out,'rust-linux','blocked','linux',f"missing required tools: {','.join(miss)}")
        else:
            code,log=run(['cargo','fmt','--all','--','--check']);
            if code==0:code,log2=run(['cargo','clippy','--workspace','--all-targets','--','-D','warnings']);log+=log2
            if code==0:code,log2=run(['cargo','test','--workspace']);log+=log2
            if code==0:code,log2=run(['cargo','build','--workspace','--release']);log+=log2
            logp=ROOT/'docs/cert-rust-linux.log';logp.write_text(log);rec(out,'rust-linux','pass' if code==0 else 'fail','linux','fmt+clippy+test+release build',artifact=logp)
    # Frontends: only real local dependency installs count
    for cid,folder in [('desktop-build',ROOT/'apps/desktop'),('dashboard-build',ROOT/'cloud/dashboard')]:
        if not (folder/'node_modules/.bin/vite').exists():rec(out,cid,'blocked','linux','node_modules/vite not installed; source typing is not a production bundle')
        else:
            code,log=run(['npm','run','build'],cwd=folder,timeout=600);lp=ROOT/f'docs/cert-{cid}.log';lp.write_text(log);rec(out,cid,'pass' if code==0 else 'fail','linux','npm production build',artifact=lp)
    # Linux package/updater only after real Rust binaries exist
    rel=ROOT/'target/release'; needed=[rel/'vsn',rel/'vsn-agent',rel/'vsn-updater-helper']
    if host=='linux':
        if not all(p.is_file() for p in needed):
            rec(out,'deb-install-uninstall','blocked','linux','real release binaries are unavailable')
            rec(out,'updater-linux','blocked','linux','vsn-updater-helper release binary unavailable')
        else:
            dist=ROOT/'dist-cert';shutil.rmtree(dist,ignore_errors=True);dist.mkdir()
            code,log=run([str(ROOT/'packaging/linux/build-deb.sh'),VERSION,str(rel),str(dist)],timeout=300);pkg=dist/f'vsn-runtime-{VERSION}-amd64.deb'
            if code==0 and os.geteuid()==0:
                c2,l2=run(['dpkg','-i',str(pkg)],timeout=180);log+=l2
                if c2==0:c2,l2=run(['/usr/local/bin/vsn','--version']);log+=l2
                c3,l3=run(['dpkg','-r','vsn-runtime'],timeout=180);log+=l3;code=max(code,c2,c3)
            elif code==0: code=2;log+='\nnot root: install/uninstall acceptance not executed'
            lp=ROOT/'docs/cert-deb.log';lp.write_text(log);rec(out,'deb-install-uninstall','pass' if code==0 else ('blocked' if code==2 else 'fail'),'linux','real deb install/version/uninstall acceptance',artifact=lp if lp.exists() else None)
            code,log=run([sys.executable,str(ROOT/'scripts/smoke-updater-helper.py'),'--helper',str(rel/'vsn-updater-helper')],timeout=180);lp=ROOT/'docs/cert-updater-linux.log';lp.write_text(log);rec(out,'updater-linux','pass' if code==0 else 'fail','linux','apply/status/rollback E2E',artifact=lp)
    # RustSec/fuzz require their real tools
    if shutil.which('cargo-audit'):
        code,log=run(['cargo','audit'],timeout=600);lp=ROOT/'docs/cert-rustsec.log';lp.write_text(log);rec(out,'rustsec-audit','pass' if code==0 else 'fail','linux','cargo audit',artifact=lp)
    else:rec(out,'rustsec-audit','blocked','linux','cargo-audit unavailable')
    for cid,target in [('fuzz-remote-protocol','remote_protocol'),('fuzz-stream-open','stream_open')]:
        if shutil.which('cargo-fuzz') and shutil.which('cargo'):
            code,log=run(['cargo','fuzz','run',target,'--','-max_total_time=90','-rss_limit_mb=2048'],cwd=ROOT/'fuzz',timeout=180);lp=ROOT/f'docs/cert-{cid}.log';lp.write_text(log);rec(out,cid,'pass' if code==0 else 'fail','linux',f'cargo-fuzz {target}',artifact=lp)
        else:rec(out,cid,'blocked','linux','cargo-fuzz/cargo unavailable')
    if a.control_url:
        code,log=run([sys.executable,str(ROOT/'scripts/load-control-plane.py'),'--url',a.control_url,'--requests','1000','--concurrency','32','--max-error-rate','0.01','--max-p95-ms','1000'],timeout=300);lp=ROOT/'docs/cert-control-load.log';lp.write_text(log);rec(out,'control-load-slo','pass' if code==0 else 'fail','linux','live Control Plane load/SLO probe',artifact=lp)
    else:rec(out,'control-load-slo','blocked','linux','--control-url not supplied; toy server evidence is not accepted')
    # Controls that require external environments are explicitly blocked here.
    for cid,note in [('ha-failover','multi-node PostgreSQL/Control Plane target not supplied'),('dr-restore','source/restore PostgreSQL targets not supplied'),('vault-key-rotation','real VSN binary/toolchain unavailable for isolated Vault E2E'),('penetration-test','independent penetration-test report required')]:
        row=json.loads(out.read_text());cur=next(x for x in row['checks'] if x['id']==cid)
        if cur['status']=='pending':rec(out,cid,'blocked','cross-platform',note)
    report=ROOT/f'docs/release-evidence-{VERSION}-local-report.json';subprocess.run([sys.executable,str(EVID),'evaluate','--file',str(out),'--report',str(report)],check=True)
    print(report.read_text());return 0
if __name__=='__main__':raise SystemExit(main())
