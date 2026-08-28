#!/usr/bin/env python3
from __future__ import annotations
import json, subprocess
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
FORBIDDEN_PARTS={"repo-import","node_modules","target","__pycache__",".pkg01-toolchain",".pkg01-assets"}
FORBIDDEN_SUFFIXES=(".zip",".tar",".tar.gz",".tar.xz",".tgz",".pyc")
LIVE_PROJECTIONS=("README.md",".ai/README.md","docs/MASTER-EXECUTION-PLAN.md")
CHECKPOINT_SEMANTICS="NON_AUTHORITATIVE_CHECKPOINT_REFRESH_LIVE_STATE_BEFORE_ANY_MUTATION"


def tracked_files():
    r=subprocess.run(["git","ls-files"],cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,check=False)
    if r.returncode: raise RuntimeError(r.stdout)
    return [x for x in r.stdout.splitlines() if x]


def load_json(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise RuntimeError(f"invalid JSON {path.relative_to(ROOT)}: {exc}") from exc


def active_tracker(active_package: str):
    matches=[]
    for path in sorted((ROOT/'certification').glob('*.json')):
        try:
            payload=json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue
        if payload.get('package_id')==active_package and isinstance(payload.get('tasks'),list):
            matches.append((path,payload))
    if len(matches)!=1:
        return None, None, f'expected exactly one tracker for {active_package}, found {len(matches)}'
    return matches[0][0], matches[0][1], None


def validate_tracker_shape(active_package: str, tracker: dict):
    errors=[]
    name=tracker.get('name')
    done=tracker.get('done')
    required=tracker.get('required')
    percent=tracker.get('percent')
    status=tracker.get('status')
    active_task=tracker.get('active_task')
    ready_tasks=tracker.get('ready_tasks',[])
    tasks=tracker.get('tasks',[])
    if not isinstance(name,str) or not name:
        errors.append(f'{active_package} tracker name is missing')
    if not isinstance(done,int) or not isinstance(required,int) or required<=0:
        errors.append(f'{active_package} tracker progress counts are invalid')
    if not isinstance(percent,(int,float)):
        errors.append(f'{active_package} tracker percent is invalid')
    if not isinstance(status,str) or not status:
        errors.append(f'{active_package} tracker status is missing')
    if active_task is not None and not isinstance(active_task,str):
        errors.append(f'{active_package} active_task is invalid')
    if not isinstance(ready_tasks,list) or any(not isinstance(x,str) for x in ready_tasks):
        errors.append(f'{active_package} ready_tasks is invalid')
    if not isinstance(tasks,list):
        errors.append(f'{active_package} tasks is invalid')
    if errors:
        return errors
    done_from_tasks=sum(1 for x in tasks if isinstance(x,dict) and x.get('status')=='DONE')
    if done_from_tasks!=done:
        errors.append(f'tracker DONE count mismatch: tasks={done_from_tasks}, declared={done}')
    if len(tasks)!=required:
        errors.append('tracker task count differs from required')
    expected_percent=round((done/required)*100,2)
    if round(float(percent),2)!=expected_percent:
        errors.append(f'tracker percent mismatch: expected={expected_percent:.2f}, declared={float(percent):.2f}')
    return errors


def projection_marker(active_package: str, tracker: dict):
    done=tracker['done']
    required=tracker['required']
    status=tracker['status']
    active_task=tracker.get('active_task') or 'none'
    ready_tasks=tracker.get('ready_tasks',[])
    ready_machine=','.join(ready_tasks) if ready_tasks else 'none'
    return (
        f'<!-- Canonical active-package machine state: {active_package} {done}/{required} {status}; '
        f'READY {ready_machine}; deterministic cursor {active_task}; '
        'query live main SHA at execution time -->'
    )


def live_projection_errors(active_package: str, tracker: dict):
    errors=[]
    marker=projection_marker(active_package,tracker)
    for rel in LIVE_PROJECTIONS:
        path=ROOT/rel
        if not path.is_file():
            errors.append(f'missing designated live projection: {rel}')
            continue
        text=path.read_text(encoding="utf-8")
        if marker not in text:
            errors.append(f'{rel} machine projection differs from active tracker')

    readme=ROOT/'README.md'
    if readme.is_file():
        text=readme.read_text(encoding="utf-8")
        name=tracker['name']
        done=tracker['done']
        required=tracker['required']
        percent=tracker['percent']
        active_task=tracker.get('active_task') or 'none'
        ready_tasks=tracker.get('ready_tasks',[])
        ready_visible=', '.join(f'`{task}`' for task in ready_tasks) if ready_tasks else 'none'
        expected=(
            ('current-package heading',f'**{active_package} — {name}**'),
            ('current-package progress',f'- Current genuine {active_package} progress: `{done}/{required} = {float(percent):.2f}%`.'),
            ('current-package cursor/READY projection',f'- Deterministic resume cursor: `{active_task}`; dependency-ready tasks: {ready_visible}.'),
        )
        for label,needle in expected:
            if needle not in text:
                errors.append(f'README {label} differs from active tracker')
    return errors


def ai_state_errors():
    errors=[]
    path=ROOT/'.ai'/'state.json'
    if not path.is_file():
        return ['missing .ai/state.json']
    try:
        state=load_json(path)
    except RuntimeError as exc:
        return [str(exc)]
    canonical=state.get('canonical_state')
    if not isinstance(canonical,dict):
        return ['.ai/state.json canonical_state is missing']
    sources=canonical.get('sources')
    if not isinstance(sources,list) or any(not isinstance(x,str) for x in sources):
        errors.append('.ai/state.json canonical_state.sources is invalid')
    elif any(x.startswith('certification/') for x in sources):
        errors.append('.ai/state.json canonical sources must not hardcode a package tracker')
    resolution=canonical.get('active_tracker_resolution')
    if not isinstance(resolution,dict):
        errors.append('.ai/state.json active_tracker_resolution is missing')
    else:
        if resolution.get('package_id_source')!='docs/MASTER-EXECUTION-STATUS.json#active_package':
            errors.append('.ai/state.json active tracker package source is not live master status')
        if resolution.get('selector')!='tracker.package_id == active_package':
            errors.append('.ai/state.json active tracker selector is invalid')
        if resolution.get('requires_exactly_one_match') is not True:
            errors.append('.ai/state.json active tracker resolution must require exactly one match')
    projections=canonical.get('designated_live_projections')
    if not isinstance(projections,list) or set(projections)!=set(LIVE_PROJECTIONS):
        errors.append('.ai/state.json designated live projections differ from governance contract')
    if canonical.get('wip_checkpoint')!='.ai/current-work.json' or canonical.get('wip_checkpoint_authoritative') is not False:
        errors.append('.ai/state.json WIP checkpoint authority semantics are invalid')
    planning=state.get('planning_scope',{})
    if planning.get('may_change_frozen_active_package_sequence') is not False:
        errors.append('.ai/state.json must prohibit silent active-package sequence changes')
    if planning.get('adoption_audit_template')!='.ai/templates/adoption-audit.v1.json':
        errors.append('.ai/state.json adoption audit template binding is missing')
    if planning.get('capability_ledger_template')!='.ai/templates/capability-ledger.v1.json':
        errors.append('.ai/state.json capability ledger template binding is missing')
    return errors


def checkpoint_errors():
    errors=[]
    path=ROOT/'.ai'/'current-work.json'
    if not path.is_file():
        return ['missing .ai/current-work.json']
    try:
        checkpoint=load_json(path)
    except RuntimeError as exc:
        return [str(exc)]
    if checkpoint.get('snapshot_semantics')!=CHECKPOINT_SEMANTICS:
        errors.append('.ai/current-work.json must declare non-authoritative refresh semantics')
    refresh=checkpoint.get('live_refresh')
    if not isinstance(refresh,dict):
        errors.append('.ai/current-work.json live_refresh is missing')
    else:
        if refresh.get('required_before_any_mutation') is not True:
            errors.append('.ai/current-work.json must require live refresh before mutation')
        if refresh.get('required_before_resume') is not True:
            errors.append('.ai/current-work.json must require live refresh before resume')
        if refresh.get('checkpoint_conflict_action')!='STOP_AND_RECONCILE':
            errors.append('.ai/current-work.json conflict action must be STOP_AND_RECONCILE')
    semantics=checkpoint.get('state_semantics')
    if not isinstance(semantics,dict):
        errors.append('.ai/current-work.json state_semantics is missing')
    else:
        if semantics.get('repository_evidence_over_checkpoint') is not True:
            errors.append('.ai/current-work.json must prefer repository evidence over checkpoint')
        if semantics.get('conversation_memory_authoritative') is not False:
            errors.append('.ai/current-work.json must mark conversation memory non-authoritative')
    return errors


def adoption_assets_errors():
    errors=[]
    required=(
        '.ai/governance/ADOPTION-RESUME.md',
        '.ai/templates/adoption-audit.v1.json',
        '.ai/templates/capability-ledger.v1.json',
    )
    for rel in required:
        if not (ROOT/rel).is_file():
            errors.append(f'missing adoption/resume asset: {rel}')
    for rel in required[1:]:
        path=ROOT/rel
        if path.is_file():
            try:
                load_json(path)
            except RuntimeError as exc:
                errors.append(str(exc))
    return errors


def main():
    errors=[]
    for name in tracked_files():
        p=Path(name)
        if any(part in FORBIDDEN_PARTS for part in p.parts): errors.append(f"forbidden tracked path: {name}")
        if any(name.endswith(s) for s in FORBIDDEN_SUFFIXES): errors.append(f"forbidden tracked artifact: {name}")

    errors.extend(ai_state_errors())
    errors.extend(checkpoint_errors())
    errors.extend(adoption_assets_errors())

    status_path=ROOT/'docs'/'MASTER-EXECUTION-STATUS.json'
    if not status_path.is_file():
        errors.append('missing docs/MASTER-EXECUTION-STATUS.json')
    if not errors:
        try:
            status=load_json(status_path)
        except RuntimeError as exc:
            errors.append(str(exc))
            status=None
        if status is not None:
            active_package=status.get('active_package')
            if not isinstance(active_package,str) or not active_package:
                errors.append('master status active_package is missing')
            else:
                tracker_path,tracker,tracker_error=active_tracker(active_package)
                if tracker_error:
                    errors.append(tracker_error)
                else:
                    errors.extend(validate_tracker_shape(active_package,tracker))
                    package=next((p for p in status.get('packages',[]) if p.get('id')==active_package),None)
                    if not package:
                        errors.append(f'{active_package} missing from master status')
                    else:
                        if package.get('done')!=tracker.get('done'): errors.append(f'{active_package} done count differs between master status and tracker')
                        if package.get('required')!=tracker.get('required'): errors.append(f'{active_package} required count differs between master status and tracker')
                        if round(float(package.get('percent',-1)),2)!=round(float(tracker.get('percent',-2)),2): errors.append(f'{active_package} percent differs between master status and tracker')
                        if package.get('status')!=tracker.get('status'): errors.append(f'{active_package} status differs between master status and tracker')
                    if status.get('active_task')!=tracker.get('active_task'): errors.append('active task differs between master status and tracker')
                    if tracker_path is None: errors.append('active tracker path resolution failed')
                    errors.extend(live_projection_errors(active_package,tracker))

    if errors:
        print('REPOSITORY GOVERNANCE: FAIL')
        for error in errors: print(f'- {error}')
        return 1
    print('REPOSITORY GOVERNANCE: PASS')
    return 0


if __name__=='__main__':
    raise SystemExit(main())
