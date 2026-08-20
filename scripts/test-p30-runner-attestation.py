#!/usr/bin/env python3
from __future__ import annotations
import json,subprocess,sys,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]; VERSION=(ROOT/'VERSION').read_text().strip()
def main():
 with tempfile.TemporaryDirectory() as td:
  out=Path(td)/'att.json'
  q=subprocess.run([sys.executable,str(ROOT/'scripts/p30-runner-attest.py'),'--root',str(ROOT),'--output',str(out),'--pack','linux-core'],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False)
  assert q.returncode==0,q.stdout;d=json.loads(out.read_text());assert d['product_version']==VERSION;assert len(d['candidate_id'])==64;assert d['pack_id']=='linux-core';assert 'cargo' in d['tools'];assert d['host']['system']
 print('p30 runner attestation regression PASS')
if __name__=='__main__':main()
