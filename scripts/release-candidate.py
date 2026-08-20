#!/usr/bin/env python3
from __future__ import annotations
import argparse, hashlib, json
from pathlib import Path
INCLUDED_PREFIXES=(
    '.github/workflows/','apps/','cloud/','contracts/','crates/','native/','packaging/','providers/','scripts/','certification/'
)
INCLUDED_ROOT={'VERSION','Cargo.toml','Cargo.lock','rust-toolchain.toml'}
def included(rel:str)->bool:
    if rel in INCLUDED_ROOT:return True
    if not any(rel.startswith(x) for x in INCLUDED_PREFIXES):return False
    if '/__pycache__/' in '/'+rel or rel.endswith('.pyc'):return False
    return True
def digest_file(p:Path):
    h=hashlib.sha256()
    with p.open('rb') as f:
        for b in iter(lambda:f.read(1024*1024),b''):h.update(b)
    return h.hexdigest()
def compute(root:Path):
    root=root.resolve(); version=(root/'VERSION').read_text().strip(); rows=[]
    for p in sorted(x for x in root.rglob('*') if x.is_file()):
        rel=p.relative_to(root).as_posix()
        if included(rel):rows.append((rel,digest_file(p)))
    h=hashlib.sha256()
    for rel,d in rows:h.update(rel.encode()+b'\0'+d.encode()+b'\n')
    source=h.hexdigest(); c=hashlib.sha256(f'vsn-release-candidate-v1\0{version}\0{source}'.encode()).hexdigest()
    return {'schema_version':1,'product_version':version,'candidate_id':c,'source_fingerprint_sha256':source,'file_count':len(rows),'profile':'release-inputs-v1'}
def main():
    ap=argparse.ArgumentParser(); ap.add_argument('command',choices=['show','id','write','verify']); ap.add_argument('--root',default='.'); ap.add_argument('--output',default='docs/release-candidate-current.json'); ap.add_argument('--file'); a=ap.parse_args(); root=Path(a.root); d=compute(root)
    if a.command=='id':print(d['candidate_id']);return
    if a.command=='show':print(json.dumps(d,indent=2,sort_keys=True));return
    target=Path(a.file or a.output)
    if not target.is_absolute():target=root/target
    if a.command=='write':target.write_text(json.dumps(d,indent=2,sort_keys=True)+'\n');print(target);return
    old=json.loads(target.read_text());
    if old!=d:raise SystemExit('release candidate fingerprint mismatch')
    print('release candidate fingerprint PASS')
if __name__=='__main__':main()
