#!/usr/bin/env python3
from __future__ import annotations
import json,subprocess,sys,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]; VERSION=(ROOT/'VERSION').read_text().strip(); PACKS=ROOT/'certification/p30-runner-packs.json'
EXPECTED={'rust-windows','rust-linux','rust-macos','desktop-build','dashboard-build','msi-install-uninstall','deb-install-uninstall','pkg-install-uninstall','updater-windows','updater-linux','updater-macos','windows-authenticode','macos-notarization','rustsec-audit','fuzz-remote-protocol','fuzz-stream-open','control-load-slo','ha-failover','dr-restore','vault-key-rotation','penetration-test'}
def run(*args):return subprocess.run([sys.executable,*map(str,args)],cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False)
def main():
 doc=json.loads(PACKS.read_text());assert doc['product_version']==VERSION
 controls=[c for p in doc['packs'] for c in p['controls']];assert set(controls)==EXPECTED;assert len(controls)==len(EXPECTED)
 ids={p['id'] for p in doc['packs']};assert ids=={'linux-core','windows-core','macos-core','security-nightly','operations','independent-review'}
 for pid in sorted(ids):
  b=run(ROOT/'scripts/p30-bootstrap-plan.py','--pack',pid);assert b.returncode==0,b.stdout;data=json.loads(b.stdout);assert data['product_version']==VERSION and data['pack_id']==pid
  p=run(ROOT/'scripts/p30-pack-preflight.py','--pack',pid);assert p.returncode in {0,3},p.stdout;data=json.loads(p.stdout);assert data['product_version']==VERSION and data['pack_id']==pid and set(data['controls'])==set(next(x['controls'] for x in doc['packs'] if x['id']==pid))
 src=(ROOT/'scripts/p30-run-pack.py').read_text();
 for token in ['linux_core','windows_core','macos_core','security_pack','external_pack','load_external_commands','runner-attestation.json','candidate_id','release-evidence.py']:assert token in src
 assert '/bin/sh' not in src[src.index('def external_pack'):src.index('def rust_quality')]
 print('p30 runner-pack regression PASS')
if __name__=='__main__':main()
