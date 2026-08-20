#!/usr/bin/env python3
"""Verify source-scope closure separately from P30 external certification."""
from __future__ import annotations
import json, subprocess, sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
REQUIRED=[
 'scripts/release-gate.py','scripts/release-evidence.py','scripts/release-preflight.py','scripts/control-plane-dr.py',
 '.github/workflows/ci.yml','.github/workflows/release-gate.yml','.github/workflows/security-nightly.yml','.github/workflows/release-signing.yml',
 'docs/security-test-plan.md','docs/threat-model.md','docs/installer-signing.md','contracts/release-certification-evidence-v1.schema.json'
]
SOURCE_CLOSED={'P0','P1','P2','P3','P4','P5','P6','P7','P8','P9','P10','P11','P12','P13','P14','P15','P16','P17','P18','P19','P20','P21','P22','P23','P24','P25','P26','P27','P28','P29'}
def main():
    errors=[]
    for rel in REQUIRED:
        if not (ROOT/rel).is_file(): errors.append(f'missing {rel}')
    roadmap=json.loads((ROOT/'docs/roadmap-status.json').read_text())
    rows={r['id']:r for r in roadmap['phases']}
    for phase in SOURCE_CLOSED:
        row=rows.get(phase)
        if not row or row.get('completion_percent')!=100 or row.get('status')!='done': errors.append(f'{phase} is not source-closed at 100%')
    if '--run-gate' in sys.argv:
        proc=subprocess.run([sys.executable,str(ROOT/'scripts/release-gate.py')],cwd=ROOT)
        if proc.returncode: errors.append('release-gate.py failed')
    result={'ok':not errors,'product_version':roadmap.get('product_version'),'source_closed_phases':sorted(SOURCE_CLOSED,key=lambda x:int(x[1:])), 'external_certification_phase':'P30','errors':errors}
    print(json.dumps(result,indent=2));raise SystemExit(1 if errors else 0)
if __name__=='__main__':main()
