#!/usr/bin/env python3
"""Cross-platform end-to-end smoke for the out-of-process updater helper."""
from __future__ import annotations
import argparse, hashlib, json, os, pathlib, shutil, subprocess, tempfile

def invoke(helper:pathlib.Path,payload:dict)->dict:
    p=subprocess.run([str(helper)],input=json.dumps(payload),text=True,capture_output=True,timeout=30)
    if p.returncode!=0: raise RuntimeError(f'helper failed rc={p.returncode}: {p.stderr.strip()}')
    out=json.loads(p.stdout.strip())
    if not out.get('ok'): raise RuntimeError(f'helper rejected request: {out}')
    return out

def main()->int:
    ap=argparse.ArgumentParser();ap.add_argument('--helper',required=True);args=ap.parse_args();helper=pathlib.Path(args.helper).resolve()
    if not helper.is_file(): raise SystemExit(f'helper missing: {helper}')
    with tempfile.TemporaryDirectory(prefix='vsn-updater-smoke-') as td:
        root=pathlib.Path(td)/'install'; staged=pathlib.Path(td)/'staged.bin'; target=root/'bin'/'probe.bin'
        target.parent.mkdir(parents=True);target.write_bytes(b'old-version\n');staged.write_bytes(b'new-version\n')
        digest=hashlib.sha256(staged.read_bytes()).hexdigest()
        apply={"operation":"apply","request":{"install_root":str(root),"target_relative":"bin/probe.bin","staged_artifact":str(staged),"expected_sha256":digest,"release":"0.18.0-smoke","confirm_apply":True}}
        invoke(helper,apply)
        if target.read_bytes()!=b'new-version\n': raise RuntimeError('apply content mismatch')
        status=invoke(helper,{"operation":"status","install_root":str(root)})['result']
        if status.get('current_release')!='0.18.0-smoke' or not status.get('rollback_available'): raise RuntimeError(f'unexpected update status: {status}')
        invoke(helper,{"operation":"rollback","install_root":str(root),"confirm_rollback":True})
        if target.read_bytes()!=b'old-version\n': raise RuntimeError('rollback content mismatch')
        final=invoke(helper,{"operation":"status","install_root":str(root)})['result']
        if final.get('rollback_available'): raise RuntimeError(f'rollback still reported available: {final}')
        print('UPDATER_HELPER_E2E=PASS')
    return 0
if __name__=='__main__': raise SystemExit(main())
