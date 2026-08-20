#!/usr/bin/env python3
from __future__ import annotations
import argparse,json,subprocess,sys
from pathlib import Path
from datetime import datetime, timezone, timedelta
ROOT=Path(__file__).resolve().parents[1]
VERSION=(ROOT/'VERSION').read_text().strip()
EVID=ROOT/'scripts/release-evidence.py'
CONTROL_META={
'rust-windows':('Windows Rust quality/build','Windows + cargo/rustc','release-gate rust-quality/windows'),
'rust-linux':('Linux Rust quality/build','Linux + cargo/rustc','release-gate rust-quality/linux'),
'rust-macos':('macOS Rust quality/build','macOS + cargo/rustc','release-gate rust-quality/macos'),
'desktop-build':('Desktop production bundle','Node/npm + installed dependencies','release-gate frontends'),
'dashboard-build':('Dashboard production bundle','Node/npm + installed dependencies','release-gate frontends'),
'msi-install-uninstall':('Windows MSI acceptance','Windows + compiled binaries + WiX','release-gate windows-msi'),
'deb-install-uninstall':('Linux deb acceptance','Linux root + compiled binaries + dpkg','release-gate linux-deb'),
'pkg-install-uninstall':('macOS pkg acceptance','macOS + compiled binaries + pkg tools','release-gate macos-pkg'),
'updater-windows':('Updater E2E Windows','Windows + updater-helper binary','release-gate rust-quality'),
'updater-linux':('Updater E2E Linux','Linux + updater-helper binary','release-gate rust-quality'),
'updater-macos':('Updater E2E macOS','macOS + updater-helper binary','release-gate rust-quality'),
'windows-authenticode':('Windows Authenticode','Windows + SignTool + protected certificate','release-signing'),
'macos-notarization':('macOS notarization','macOS + signing identity + Apple notarization credentials','release-signing'),
'rustsec-audit':('RustSec audit','cargo + cargo-audit + dependency index','security/release gate'),
'fuzz-remote-protocol':('Remote protocol fuzz','cargo + cargo-fuzz + fuzz corpus','security-nightly'),
'fuzz-stream-open':('Stream-open fuzz','cargo + cargo-fuzz + fuzz corpus','security-nightly'),
'control-load-slo':('Control Plane load/SLO','live Control Plane target','certify-local / reviewed evidence'),
'ha-failover':('HA failover','multi-node Control Plane + shared PostgreSQL','reviewed external evidence'),
'dr-restore':('Disaster-recovery restore','source + isolated restore PostgreSQL targets','reviewed external evidence'),
'vault-key-rotation':('Vault rotation E2E','real VSN binaries + isolated test state','reviewed/local evidence'),
'penetration-test':('Independent penetration test','independent security assessment report','reviewed external evidence'),
}

def load_eval(evidence:Path):
    p=subprocess.run([sys.executable,str(EVID),'evaluate','--file',str(evidence)],text=True,stdout=subprocess.PIPE,check=True)
    return json.loads(p.stdout)

def load_evidence(path:Path): return json.loads(path.read_text())

def exact(ev):
    required=max(1,int(ev['required'])); satisfied=int(ev['satisfied'])
    cert=100*satisfied/required
    p30=100.0 if ev['certified'] else 66.0+34.0*satisfied/required
    overall=(3000.0+p30)/31.0
    return cert,p30,overall

def bar(v,width=20):
    n=max(0,min(width,round(float(v)*width/100)))
    return '█'*n+'░'*(width-n)

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('--evidence',default=str(ROOT/'docs/release-evidence-current.json'))
    ap.add_argument('--local-evidence')
    ap.add_argument('--json-output')
    ap.add_argument('--markdown-output')
    a=ap.parse_args(); ep=Path(a.evidence)
    ev=load_eval(ep); ledger=load_evidence(ep); cert,p30,overall=exact(ev)
    rows={r['id']:r for r in ledger.get('checks',[])}
    local_rows={}
    if a.local_evidence:
        lp=Path(a.local_evidence)
        if lp.exists(): local_rows={r['id']:r for r in load_evidence(lp).get('checks',[])}
    invalid=set(ev.get('invalid_provenance',[]));expired=set(ev.get('expired',[]))
    controls=[]
    per_p30=34.0/ev['required']; per_overall=per_p30/31.0
    for cid in CONTROL_META:
        row=rows.get(cid,{})
        raw=row.get('status','missing')
        effective=raw
        if cid in invalid: effective='invalid-pass'
        if cid in expired: effective='expired'
        title,requirement,path=CONTROL_META[cid]
        controls.append({
            'id':cid,'title':title,'raw_status':raw,'effective_status':effective,
            'requirement':requirement,'certification_path':path,
            'p30_gain_if_next_valid_pass':round(per_p30,4),
            'overall_gain_if_next_valid_pass':round(per_overall,4),
            'notes':row.get('notes') or '', 'recorded_at':row.get('recorded_at'),
            'current_runner_status':local_rows.get(cid,{}).get('status') if local_rows else None,
            'current_runner_notes':local_rows.get(cid,{}).get('notes') if local_rows else None
        })
    status_counts={}
    for c in controls: status_counts[c['effective_status']]=status_counts.get(c['effective_status'],0)+1
    out={
      'schema_version':2,'product_version':VERSION,'release_candidate_id':ledger.get('candidate_id'),
      'source_completion_exact_percent':100.0,
      'p30_source_ready_floor_percent':66.0,
      'certification_evidence_exact_percent':round(cert,4),
      'certification_satisfied':ev['satisfied'],'certification_required':ev['required'],
      'p30_completion_exact_percent':round(p30,4),
      'overall_completion_exact_percent':round(overall,4),
      'headline_completion_percent':round(overall),
      'stable_release_certified':bool(ev['certified']),
      'p30_points_per_valid_control':round(per_p30,4),
      'overall_points_per_valid_control':round(per_overall,4),
      'status_counts':status_counts,'controls':controls,
      'milestones':[
        {'valid_passes':n,'certification_percent':round(100*n/ev['required'],2),
         'p30_exact_percent':round(100.0 if n==ev['required'] else 66.0+34.0*n/ev['required'],2),
         'overall_exact_percent':round((3000.0+(100.0 if n==ev['required'] else 66.0+34.0*n/ev['required']))/31.0,2)}
        for n in [0,1,2,5,10,15,20,21]
      ]
    }
    text=json.dumps(out,indent=2,sort_keys=True);print(text)
    if a.json_output:Path(a.json_output).write_text(text+'\n')
    if a.markdown_output:
      md=[f'# P30 Certification Scoreboard — {VERSION}','',f'**Release candidate:** `{ledger.get("candidate_id")}`','',
          f'**Source completion:** 100.00%  `{bar(100)}`',
          f'**P30 exact:** {p30:.2f}%  `{bar(p30)}`',
          f'**Certification evidence:** {ev["satisfied"]}/{ev["required"]} = {cert:.2f}%  `{bar(cert)}`',
          f'**Overall exact:** {overall:.2f}%  `{bar(overall)}`','',
          f'Each valid certification control contributes **{per_p30:.4f} P30 points** and **{per_overall:.4f} overall percentage-points**.','',
          '| Control | Authoritative | Current runner | Requirement | P30 gain | Overall gain |','|---|---|---|---|---:|---:|']
      for c in controls:
        st=c['effective_status'];icon={'pass':'✅','blocked':'⛔','pending':'🟡','fail':'❌','expired':'⌛','invalid-pass':'⚠️','waived':'⚪'}.get(st,'•')
        local=c.get('current_runner_status') or 'n/a'
        md.append(f"| `{c['id']}` | {icon} {st} | {local} | {c['requirement']} | +{per_p30:.4f} | +{per_overall:.4f} |")
      Path(a.markdown_output).write_text('\n'.join(md)+'\n')
if __name__=='__main__':raise SystemExit(main())
