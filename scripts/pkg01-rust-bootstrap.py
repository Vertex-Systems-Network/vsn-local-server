#!/usr/bin/env python3
from __future__ import annotations
import argparse, hashlib, os, shutil, subprocess, sys, tarfile, tempfile, urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
VERSION='1.97.1';TARGET='x86_64-unknown-linux-gnu';ARCHIVE=f'rust-{VERSION}-{TARGET}.tar.xz'
SHA='88f28fa9af20594179f85d6df67078dfd6fa93e2f6da5e1e9b0ac4997988ca4f'
PREFIX=ROOT/'.pkg01-toolchain'/f'rust-{VERSION}'
URLS=[f'https://static.rust-lang.org/dist/2026-07-16/{ARCHIVE}',f'https://mirror1.hs-esslingen.de/Mirrors/gentoo/distfiles/6b/{ARCHIVE}',f'https://ftp.gwdg.de/pub/linux/gentoo/distfiles/6b/{ARCHIVE}']
CHUNK_NAMES=[f'rust-{VERSION}.tar.xz.part{i:03d}' for i in range(3)]
CHUNK_BASE='https://github.com/kelexine/rust-toolchain/raw/refs/heads/main/toolchains/1.97.1'

def sha256(p:Path):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for b in iter(lambda:f.read(1024*1024),b''):h.update(b)
 return h.hexdigest()

def run(args,cwd=None,timeout=900,env=None):return subprocess.run(args,cwd=cwd,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False,timeout=timeout,env=env)

def tool_env():
 env=os.environ.copy();env['PATH']=str(PREFIX/'bin')+os.pathsep+env.get('PATH','');return env

def valid_install():
 env=tool_env()
 for c in ['rustc','cargo','rustfmt','cargo-clippy']:
  if not shutil.which(c,path=env['PATH']):return False
 for c in ['rustc','cargo']:
  p=run([c,'--version'],cwd=ROOT,timeout=30,env=env);parts=(p.stdout or '').split()
  if p.returncode or len(parts)<2 or parts[1]!=VERSION:return False
 return True

def candidate_archives(explicit=None):
 seen=set()
 for x in [explicit,os.environ.get('VSN_RUST_ARCHIVE'),str(ROOT/'.pkg01-assets'/ARCHIVE),f'/mnt/data/{ARCHIVE}']:
  if not x:continue
  p=Path(x).expanduser().resolve()
  if p not in seen:seen.add(p);yield p

def candidate_chunk_dirs(explicit=None):
 seen=set()
 for x in [explicit,os.environ.get('VSN_RUST_CHUNK_DIR'),str(ROOT/'.pkg01-assets'/'rust-1.97.1-chunks'),'/mnt/data/rust-1.97.1-chunks']:
  if not x:continue
  p=Path(x).expanduser().resolve()
  if p not in seen:seen.add(p);yield p

def reconstruct_chunks(d:Path,dst:Path):
 parts=[d/n for n in CHUNK_NAMES]
 if not all(p.is_file() for p in parts):return False
 tmp=dst.with_suffix(dst.suffix+'.part');tmp.parent.mkdir(parents=True,exist_ok=True)
 with tmp.open('wb') as out:
  for p in parts:
   with p.open('rb') as src:shutil.copyfileobj(src,out,1024*1024)
 if sha256(tmp)!=SHA:tmp.unlink(missing_ok=True);raise RuntimeError('Rust chunk reconstruction SHA mismatch')
 os.replace(tmp,dst);return True

def fetch(url,dst):
 tmp=dst.with_suffix(dst.suffix+'.download');tmp.unlink(missing_ok=True)
 try:
  req=urllib.request.Request(url,headers={'User-Agent':'VSN-PKG01/1.0'})
  with urllib.request.urlopen(req,timeout=30) as src,tmp.open('wb') as out:
   while True:
    b=src.read(1024*1024)
    if not b:break
    out.write(b)
  os.replace(tmp,dst);return True
 except Exception as e:
  print(f'fetch failed: {url}: {e}',file=sys.stderr);tmp.unlink(missing_ok=True);return False

def fetch_chunks(dst):
 with tempfile.TemporaryDirectory(prefix='vsn-rust-chunks-') as td:
  td=Path(td)
  for n in CHUNK_NAMES:
   if not fetch(f'{CHUNK_BASE}/{n}',td/n):return False
  return reconstruct_chunks(td,dst)

def install(archive:Path):
 if sha256(archive)!=SHA:raise RuntimeError('Rust archive SHA mismatch')
 with tempfile.TemporaryDirectory(prefix='vsn-rust-install-') as td:
  td=Path(td)
  with tarfile.open(archive,'r:xz') as tf:
   for m in tf.getmembers():
    n=Path(m.name)
    if n.is_absolute() or '..' in n.parts:raise RuntimeError('unsafe Rust archive path')
   tf.extractall(td,filter='data')
  roots=[p for p in td.iterdir() if p.is_dir() and (p/'install.sh').is_file()]
  if len(roots)!=1:raise RuntimeError('Rust archive install root not found')
  shutil.rmtree(PREFIX,ignore_errors=True);PREFIX.parent.mkdir(parents=True,exist_ok=True)
  r=run(['bash','install.sh',f'--prefix={PREFIX}','--without=rust-docs','--disable-ldconfig'],cwd=roots[0],timeout=1200)
  if r.returncode:raise RuntimeError('Rust install failed:\n'+(r.stdout or '')[-6000:])
 if not valid_install():raise RuntimeError('installed Rust toolchain failed exact 1.97.1 verification')

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--archive');ap.add_argument('--chunk-dir');ap.add_argument('--allow-network',action='store_true');a=ap.parse_args()
 if valid_install():print(PREFIX);return 0
 for p in candidate_archives(a.archive):
  if p.is_file():install(p);print(PREFIX);return 0
 cache=ROOT/'.pkg01-assets';cache.mkdir(parents=True,exist_ok=True);dst=cache/ARCHIVE
 for d in candidate_chunk_dirs(a.chunk_dir):
  if reconstruct_chunks(d,dst):install(dst);print(PREFIX);return 0
 if a.allow_network:
  for url in URLS:
   print('trying',url,file=sys.stderr)
   if fetch(url,dst):
    if sha256(dst)!=SHA:dst.unlink(missing_ok=True);continue
    install(dst);print(PREFIX);return 0
  if fetch_chunks(dst):install(dst);print(PREFIX);return 0
 print(f'Rust {VERSION} unavailable; expected SHA256 {SHA}',file=sys.stderr);return 4
if __name__=='__main__':raise SystemExit(main())
