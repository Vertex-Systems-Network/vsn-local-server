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

def active_tracker(active_package: str):
    matches=[]
    for path in sorted((ROOT/'certification').glob('*.json')):
        try:
            payload=json.loads(path.read_text())
        except (OSError, json.JSONDecodeError):
            continue
        if payload.get('package_id')==active_package and isinstance(payload.get('tasks'),list):
            matches.append((path,payload))
    if len(matches)!=1:
        return None, None, f'expected exactly one tracker for {active_package}, found {len(matches)}'
    return matches[0][0], matches[0][1], None

def main():
    errors=[]
    for name in tracked_files():
        p=Path(name)
        if any(part in FORBIDDEN_PARTS for part in p.parts): errors.append(f"forbidden tracked path: {name}")
        if any(name.endswith(s) for s in FORBIDDEN_SUFFIXES): errors.append(f"forbidden tracked artifact: {name}")
    status_path=ROOT/'docs'/'MASTER-EXECUTION-STATUS.json'
    if not status_path.is_file(): errors.append('missing docs/MASTER-EXECUTION-STATUS.json')
    if not errors:
        s=json.loads(status_path.read_text())
        active_package=s.get('active_package')
        if not isinstance(active_package,str) or not active_package:
            errors.append('master status active_package is missing')
        else:
            tracker_path,t,tracker_error=active_tracker(active_package)
            if tracker_error:
                errors.append(tracker_error)
            else:
                package=next((p for p in s.get('packages',[]) if p.get('id')==active_package),None)
                if not package: errors.append(f'{active_package} missing from master status')
                else:
                    if package.get('done')!=t.get('done'): errors.append(f'{active_package} done count differs between master status and tracker')
                    if package.get('required')!=t.get('required'): errors.append(f'{active_package} required count differs between master status and tracker')
                    if s.get('active_task')!=t.get('active_task'): errors.append('active task differs between master status and tracker')
                    done=sum(1 for x in t.get('tasks',[]) if x.get('status')=='DONE')
                    if done!=t.get('done'): errors.append(f'tracker DONE count mismatch: tasks={done}, declared={t.get("done")}')
                    if len(t.get('tasks',[]))!=t.get('required'): errors.append('tracker task count differs from required')
                    if tracker_path is None: errors.append('active tracker path resolution failed')
    if errors:
        print('REPOSITORY GOVERNANCE: FAIL')
        for e in errors: print(f'- {e}')
        return 1
    print('REPOSITORY GOVERNANCE: PASS')
    return 0
if __name__=='__main__': raise SystemExit(main())
