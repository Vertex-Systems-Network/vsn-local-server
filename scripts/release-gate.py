#!/usr/bin/env python3
from __future__ import annotations
import argparse, hashlib, json, os, re, subprocess, sys, tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
os.environ['PYTHONDONTWRITEBYTECODE']='1';sys.dont_write_bytecode=True
VERSION=(ROOT/'VERSION').read_text().strip()
BANNED_DIRS={'.git','target','node_modules','dist','__pycache__'}
BANNED_TEXT=[r'BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY',r'(?i)aws_secret_access_key\s*[:=]',r'(?i)(?:client_secret|api_token|bearer_token)\s*[:=]\s*["\'][A-Za-z0-9_\-]{20,}']
errors=[]
# version consistency
for p in ROOT.rglob('Cargo.toml'):
    d=tomllib.loads(p.read_text(encoding='utf-8'))
    if 'package' in d:
        pkg=d['package']; meta=pkg.get('metadata') or {}
        is_fuzz=pkg.get('publish') is False and meta.get('cargo-fuzz') is True
        if not is_fuzz and pkg.get('version')!=VERSION: errors.append(f'{p.relative_to(ROOT)} package version != {VERSION}')
roadmap=json.loads((ROOT/'docs/roadmap-status.json').read_text())
if roadmap.get('product_version')!=VERSION: errors.append('roadmap product_version mismatch')
# release candidate/evidence binding
try:
    cand=json.loads((ROOT/'docs/release-candidate-current.json').read_text())
    proc=subprocess.run([sys.executable,str(ROOT/'scripts/release-candidate.py'),'show','--root',str(ROOT)],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False)
    if proc.returncode: errors.append('release candidate computation failed')
    else:
        computed=json.loads(proc.stdout)
        if cand!=computed: errors.append('release candidate fingerprint is stale')
        evidence=json.loads((ROOT/'docs/release-evidence-current.json').read_text())
        if evidence.get('candidate_id')!=cand.get('candidate_id'): errors.append('release evidence candidate mismatch')
        if evidence.get('product_version')!=VERSION: errors.append('release evidence product_version mismatch')
except Exception as e: errors.append(f'release candidate binding: {e}')
# P30 governance/status/policy invariants
try:
    if (ROOT/'docs/p30-evidence-journal.json').is_file():
        q=subprocess.run([sys.executable,str(ROOT/'scripts/p30-evidence-governance.py'),'verify'],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False)
        if q.returncode: errors.append('P30 evidence journal/rebuild mismatch: '+(q.stdout or '').strip()[:1000])
    q=subprocess.run([sys.executable,str(ROOT/'scripts/p30-sync-status.py'),'check'],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False)
    if q.returncode: errors.append('P30 status artifacts are stale: '+(q.stdout or '').strip()[:1000])
    q=subprocess.run([sys.executable,str(ROOT/'scripts/p30-evidence-policy.py'),'--require-no-expired'],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False)
    if q.returncode: errors.append('P30 evidence aging policy failed: '+(q.stdout or '').strip()[:1000])
except Exception as e: errors.append(f'P30 governance checks: {e}')
# generated/banned dirs
for p in ROOT.rglob('*'):
    if p.is_dir() and p.name in BANNED_DIRS and p!=ROOT/'.git': errors.append(f'generated directory present: {p.relative_to(ROOT)}')
# secret-like text scan over source/config only
for p in ROOT.rglob('*'):
    if not p.is_file() or any(part in BANNED_DIRS for part in p.parts): continue
    if p.suffix.lower() not in {'.rs','.ts','.tsx','.js','.mjs','.json','.toml','.yml','.yaml','.md','.ps1','.py','.sh','.html','.css'}: continue
    try: text=p.read_text(encoding='utf-8')
    except Exception: continue
    for pat in BANNED_TEXT:
        if re.search(pat,text): errors.append(f'secret-like material in {p.relative_to(ROOT)}: {pat}')
# deterministic source manifest
files=[]
for p in sorted(ROOT.rglob('*')):
    if not p.is_file() or any(part in BANNED_DIRS for part in p.relative_to(ROOT).parts): continue
    if p.name in {'SOURCE_SHA256SUMS.txt'}: continue
    files.append((p.relative_to(ROOT).as_posix(),hashlib.sha256(p.read_bytes()).hexdigest()))
parser=argparse.ArgumentParser(); parser.add_argument('--manifest'); args=parser.parse_args()
manifest=''.join(f'{sha}  {rel}\n' for rel,sha in files)
if args.manifest: Path(args.manifest).write_text(manifest)
print(json.dumps({'ok':not errors,'version':VERSION,'files':len(files),'errors':errors},indent=2))
sys.exit(1 if errors else 0)
