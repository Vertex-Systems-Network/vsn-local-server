#!/usr/bin/env python3
from __future__ import annotations
import json, subprocess, sys, tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]; FRAG=ROOT/'scripts/p30-fragment.py'; COLLECT=ROOT/'scripts/p30-collect.py'
with tempfile.TemporaryDirectory() as td:
    td=Path(td)
    common=['--run-url','https://example.invalid/actions/runs/1','--commit-sha','a'*40]
    subprocess.run([sys.executable,str(FRAG),'--output',str(td/'linux.json'),'--platform','linux','--controls','rust-linux','updater-linux',*common,'--evidence','test/linux'],check=True,stdout=subprocess.DEVNULL)
    subprocess.run([sys.executable,str(FRAG),'--output',str(td/'front.json'),'--platform','linux','--controls','desktop-build','dashboard-build',*common,'--evidence','test/frontends'],check=True,stdout=subprocess.DEVNULL)
    p=subprocess.run([sys.executable,str(COLLECT),str(td),'--output',str(td/'merged.json'),'--report',str(td/'report.json')],check=True,text=True,stdout=subprocess.PIPE)
    d=json.loads((td/'report.json').read_text())
    assert d['satisfied']==4, d
print('p30 evidence fragments PASS')
