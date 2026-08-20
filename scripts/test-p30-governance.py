#!/usr/bin/env python3
from __future__ import annotations
import json,subprocess,sys,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def run(*a,ok=True):
 p=subprocess.run([sys.executable,*map(str,a)],cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False)
 if ok and p.returncode:raise AssertionError(p.stdout)
 return p
# canonical platform regression
from p30_platform import canonical_platform
assert canonical_platform('Darwin')=='macos' and canonical_platform('win32')=='windows' and canonical_platform('Linux')=='linux'
# neutral journal must reconstruct authoritative ledger
v=json.loads(run(ROOT/'scripts/p30-evidence-governance.py','verify').stdout);assert v['ok'] is True
# checkpoint/restore dry-run must preserve candidate/version identity
with tempfile.TemporaryDirectory() as td:
 cp=Path(td)/'checkpoint.zip';c=json.loads(run(ROOT/'scripts/p30-evidence-governance.py','checkpoint','--path',cp).stdout);assert cp.is_file() and len(c['sha256'])==64
 r=json.loads(run(ROOT/'scripts/p30-evidence-governance.py','restore-checkpoint','--path',cp,'--dry-run').stdout);assert r['ok'] is True
print('p30 governance regression: PASS')
