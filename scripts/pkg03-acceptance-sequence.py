#!/usr/bin/env python3
from __future__ import annotations
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
TRACKER=ROOT/'certification/pkg03-windows-installer-v1.json'
STATUS=ROOT/'docs/MASTER-EXECUTION-STATUS.json'
EXPECTED=[f'03.{i:02d}' for i in range(1,26)]
ALLOWED={'BLOCKED','READY','IN_PROGRESS','DONE'}

def fail(msg):
    print('PKG-03 ACCEPTANCE SEQUENCE: FAIL')
    print(f'- {msg}')
    raise SystemExit(1)

def main():
    t=json.loads(TRACKER.read_text())
    s=json.loads(STATUS.read_text())
    tasks=t.get('tasks',[])
    ids=[x.get('id') for x in tasks]
    if t.get('package_id')!='PKG-03': fail('wrong package_id')
    if t.get('required')!=25 or ids!=EXPECTED: fail('task denominator/order must be exact 03.01..03.25')
    if len(set(ids))!=25: fail('duplicate task id')
    by_id={x['id']:x for x in tasks}
    for x in tasks:
        if x.get('status') not in ALLOWED: fail(f"invalid status for {x['id']}")
        deps=x.get('depends_on',[])
        if len(deps)!=len(set(deps)): fail(f"duplicate dependency for {x['id']}")
        for d in deps:
            if d not in by_id: fail(f"unknown dependency {d} for {x['id']}")
            if EXPECTED.index(d)>=EXPECTED.index(x['id']): fail(f"non-forward DAG dependency {d}->{x['id']}")
    done_ids=[x['id'] for x in tasks if x['status']=='DONE']
    if len(done_ids)!=t.get('done'): fail('DONE count mismatch')
    pct=round(100.0*len(done_ids)/25,2)
    if float(t.get('percent'))!=pct: fail(f'percent mismatch expected {pct}')
    active=[x['id'] for x in tasks if x['status']=='IN_PROGRESS']
    ready=[x['id'] for x in tasks if x['status']=='READY']
    if sorted(t.get('active_tasks',[]))!=sorted(active): fail('active_tasks mismatch')
    if sorted(t.get('ready_tasks',[]))!=sorted(ready): fail('ready_tasks mismatch')
    if len(active)>int(t.get('max_parallel_tasks',5)): fail('parallel active-task ceiling exceeded')
    for x in tasks:
        deps_done=all(by_id[d]['status']=='DONE' for d in x.get('depends_on',[]))
        if x['status'] in {'READY','IN_PROGRESS','DONE'} and not deps_done:
            fail(f"{x['id']} advanced before dependency completion")
    actionable=sorted(active+ready)
    expected_cursor=actionable[0] if actionable else None
    if t.get('active_task')!=expected_cursor: fail('active_task is not deterministic lowest actionable cursor')
    dormant=(s.get('active_package')=='PKG-02' and s.get('active_task') is None and t.get('done')==0 and not actionable)
    active_pkg=(s.get('active_package')=='PKG-03')
    if not (dormant or active_pkg): fail('master status is neither valid dormant transition nor active PKG-03')
    if active_pkg:
        pkg=next((p for p in s.get('packages',[]) if p.get('id')=='PKG-03'),None)
        if not pkg: fail('PKG-03 missing from master status')
        if pkg.get('done')!=t.get('done') or pkg.get('required')!=25: fail('master/tracker count mismatch')
        if s.get('active_task')!=t.get('active_task'): fail('master/tracker active_task mismatch')
    if t.get('complete')!=(len(done_ids)==25): fail('complete flag mismatch')
    print('PKG-03 ACCEPTANCE SEQUENCE: PASS')
    print(f'done={len(done_ids)}/25 active={active} ready={ready} cursor={expected_cursor}')

if __name__=='__main__':
    main()
