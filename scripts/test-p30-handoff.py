#!/usr/bin/env python3
from pathlib import Path
import ast,json
R=Path(__file__).resolve().parents[1]
ast.parse((R/'scripts/p30-handoff.py').read_text());ast.parse((R/'scripts/p30-gh-dispatch.py').read_text());ast.parse((R/'scripts/p30-verify-handoff.py').read_text())
s=json.loads((R/'contracts/p30-handoff-v1.schema.json').read_text());assert s['properties']['candidate_id']['pattern']
t=(R/'scripts/p30-gh-dispatch.py').read_text();
for x in ['gh','workflow','run','gh','run','watch','gh','run','download','release-evidence.py','candidate_id']:
 assert x in t,x
h=(R/'scripts/p30-handoff.py').read_text()
for x in ['SOURCE_SHA256SUMS.txt','source manifest mismatch','P30_HANDOFF.json','bundle_sha256']:
 assert x in h,x
r=(R/'scripts/p30-run-pack.py').read_text()
for x in ['--resume-ledger','PROTECTED_PASS_IDS','protected_passes']:
 assert x in r,x
v=(R/'scripts/p30-verify-handoff.py').read_text()
for x in ['ZIP CRC failure','source manifest digest mismatch','candidate fingerprint mismatch','--execute']:
 assert x in v,x
print('p30 handoff regression: PASS')
