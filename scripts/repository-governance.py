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

def readme_projection_errors(active_package: str, tracker: dict):
    readme_path=ROOT/'README.md'
    if not readme_path.is_file():
        return ['missing README.md']
    text=readme_path.read_text()
    errors=[]
    name=tracker.get('name')
    done=tracker.get('done')
    required=tracker.get('required')
    percent=tracker.get('percent')
    status=tracker.get('status')
    active_task=tracker.get('active_task')
    ready_tasks=tracker.get('ready_tasks',[])
    if not isinstance(name,str) or not name:
        return [f'{active_package} tracker name is missing']
    if not isinstance(done,int) or not isinstance(required,int) or not isinstance(percent,(int,float)):
        return [f'{active_package} tracker progress fields are invalid']
    if not isinstance(status,str) or not status:
        return [f'{active_package} tracker status is missing']
    if active_task is not None and not isinstance(active_task,str):
        return [f'{active_package} active_task is invalid']
    if not isinstance(ready_tasks,list) or any(not isinstance(x,str) for x in ready_tasks):
        return [f'{active_package} ready_tasks is invalid']

    heading=f'**{active_package} — {name}**'
    progress=f'- Current genuine {active_package} progress: `{done}/{required} = {float(percent):.2f}%`.'
    cursor_value=active_task or 'none'
    ready_visible=', '.join(f'`{task}`' for task in ready_tasks) if ready_tasks else 'none'
    cursor=f'- Deterministic resume cursor: `{cursor_value}`; dependency-ready tasks: {ready_visible}.'
    ready_machine=','.join(ready_tasks) if ready_tasks else 'none'
    machine=(
        f'<!-- Canonical {active_package} machine state: {done}/{required} {status}; '
        f'READY {ready_machine}; deterministic cursor {cursor_value}; '
        'query live main SHA at execution time -->'
    )
    expected=(
        ('current-package heading',heading),
        ('current-package progress',progress),
        ('current-package cursor/READY projection',cursor),
        ('current-package machine projection',machine),
    )
    for label,needle in expected:
        if needle not in text:
            errors.append(f'README {label} differs from active tracker')
    return errors

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
                    errors.extend(readme_projection_errors(active_package,t))
    if errors:
        print('REPOSITORY GOVERNANCE: FAIL')
        for e in errors: print(f'- {e}')
        return 1
    print('REPOSITORY GOVERNANCE: PASS')
    return 0
if __name__=='__main__': raise SystemExit(main())
