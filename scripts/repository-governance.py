#!/usr/bin/env python3
from __future__ import annotations
import json, subprocess, sys
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
FORBIDDEN_PARTS={"repo-import","node_modules","target","__pycache__",".pkg01-toolchain",".pkg01-assets"}
FORBIDDEN_SUFFIXES=(".zip",".tar",".tar.gz",".tar.xz",".tgz",".pyc")

def tracked_files():
    r=subprocess.run(["git","ls-files"],cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False)
    if r.returncode: raise RuntimeError(r.stdout)
    return [x for x in r.stdout.splitlines() if x]

def main():
    errors=[]
    for name in tracked_files():
        p=Path(name)
        if any(part in FORBIDDEN_PARTS for part in p.parts): errors.append(f"forbidden tracked path: {name}")
        if any(name.endswith(s) for s in FORBIDDEN_SUFFIXES): errors.append(f"forbidden tracked artifact: {name}")
    status_path=ROOT/'docs'/'MASTER-EXECUTION-STATUS.json'
    tracker_path=ROOT/'certification'/'pkg01-build-foundation-v1.json'
    if not status_path.is_file(): errors.append('missing docs/MASTER-EXECUTION-STATUS.json')
    if not tracker_path.is_file(): errors.append('missing certification/pkg01-build-foundation-v1.json')
    if not errors:
        s=json.loads(status_path.read_text());t=json.loads(tracker_path.read_text())
        pkg1=next((p for p in s.get('packages',[]) if p.get('id')=='PKG-01'),None)
        if not pkg1: errors.append('PKG-01 missing from master status')
        else:
            if pkg1.get('done')!=t.get('done'): errors.append('PKG-01 done count differs between master status and tracker')
            if pkg1.get('required')!=t.get('required'): errors.append('PKG-01 required count differs between master status and tracker')
            if s.get('active_task')!=t.get('active_task'): errors.append('active task differs between master status and tracker')
            done=sum(1 for x in t.get('tasks',[]) if x.get('status')=='DONE')
            if done!=t.get('done'): errors.append(f'tracker DONE count mismatch: tasks={done}, declared={t.get("done")}')
            if len(t.get('tasks',[]))!=t.get('required'): errors.append('tracker task count differs from required')
    if errors:
        print('REPOSITORY GOVERNANCE: FAIL')
        for e in errors: print(f'- {e}')
        return 1
    print('REPOSITORY GOVERNANCE: PASS')
    return 0
if __name__=='__main__': raise SystemExit(main())
