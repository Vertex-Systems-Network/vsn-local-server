#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / '.ai/manifests/pkg03-0310-cli-agent-payload.v1.json'
TRACKER = ROOT / 'certification/pkg03-windows-installer-v1.json'
STATUS = ROOT / 'docs/MASTER-EXECUTION-STATUS.json'
BASE_TAURI = ROOT / 'apps/desktop/src-tauri/tauri.conf.json'
WINDOWS_TAURI = ROOT / 'apps/desktop/src-tauri/tauri.windows.conf.json'
OWNERSHIP = ROOT / 'installer/windows/owned-payload.v1.json'
STAGE = ROOT / 'scripts/ci/pkg03-0310-stage-windows-payload.ps1'
HARNESS = ROOT / 'scripts/ci/pkg03-0310-cli-agent-payload.ps1'
WORKFLOW = ROOT / '.github/workflows/pkg03-0310-cli-agent-payload.yml'

CANONICAL_BASE = '4f5e8ab30f030e758c52c4ca4ac08f73f896247a'
PARENT_PLAN_SHA = '9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e'
BASE_TAURI_SHA = '172cf6110e58a15442bcf97e9db6a8bdbeb6cbfd2f631d91a3031603ed474180'
OWNERSHIP_SHA = '5292a1ec1ae0a48d76f80258c9e00ba17b1466dab5c5e0cdda60caf4658dabd1'

ALLOWED = {
    '.ai/features/pkg03-0310/research.md',
    '.ai/features/pkg03-0310/lifecycle-review.md',
    '.ai/features/pkg03-0310/development-preflight.md',
    '.ai/plans/pkg03-0310-cli-agent-payload-v1.md',
    '.ai/manifests/pkg03-0310-cli-agent-payload.v1.json',
    'docs/PKG03-CLI-AGENT-PAYLOAD-LIFECYCLE-V1.md',
    'apps/desktop/src-tauri/tauri.windows.conf.json',
    'scripts/ci/pkg03-0310-stage-windows-payload.ps1',
    'scripts/ci/pkg03-0310-cli-agent-payload.ps1',
    'scripts/ci/validate-pkg03-0310.py',
    '.github/workflows/pkg03-0310-cli-agent-payload.yml',
    'certification/pkg03-windows-installer-v1.json',
    'docs/MASTER-EXECUTION-STATUS.json',
}


def fail(message: str) -> None:
    raise SystemExit('PKG-03 03.10 validation failed: ' + message)


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def git_bytes(path: str, ref: str = 'HEAD') -> bytes:
    proc = subprocess.run(
        ['git', 'show', f'{ref}:{path}'], cwd=ROOT,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )
    if proc.returncode:
        fail(f'unable to read {ref}:{path}')
    return proc.stdout


def branch_changed() -> set[str]:
    subprocess.run(['git', 'fetch', 'origin', 'main', '--quiet'], cwd=ROOT, check=False)
    proc = subprocess.run(
        ['git', 'merge-base', 'origin/main', 'HEAD'], cwd=ROOT,
        text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )
    if proc.returncode:
        fail('unable to resolve merge-base with origin/main')
    merge_base = proc.stdout.strip()
    proc = subprocess.run(
        ['git', 'diff', '--name-only', f'{merge_base}..HEAD'], cwd=ROOT,
        text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )
    if proc.returncode:
        fail('unable to inspect branch-local changed paths')
    return {line.strip() for line in proc.stdout.splitlines() if line.strip()}


def main() -> None:
    manifest = json.loads(MANIFEST.read_text(encoding='utf-8'))
    tracker = json.loads(TRACKER.read_text(encoding='utf-8'))
    master = json.loads(STATUS.read_text(encoding='utf-8'))
    base_tauri = json.loads(BASE_TAURI.read_text(encoding='utf-8'))
    windows_tauri = json.loads(WINDOWS_TAURI.read_text(encoding='utf-8'))
    ownership = json.loads(OWNERSHIP.read_text(encoding='utf-8'))

    identity = (
        manifest.get('feature_id'), manifest.get('task_id'), manifest.get('linear_issue'),
        manifest.get('version'), manifest.get('status'),
    )
    if identity != ('pkg03-0310-cli-agent-payload', '03.10', 'ABD-85', '1.0.0', 'frozen'):
        fail('task identity/version/status mismatch')
    if manifest.get('canonical_base_sha') != CANONICAL_BASE:
        fail('canonical base drifted')
    if manifest.get('parent_plan', {}).get('sha256') != PARENT_PLAN_SHA:
        fail('parent plan digest declaration drifted')
    if sha(git_bytes('.ai/plans/pkg03-windows-installer-v1.md', CANONICAL_BASE)) != PARENT_PLAN_SHA:
        fail('canonical parent plan digest mismatch')

    for label, obj in (
        ('research', manifest['research']),
        ('lifecycle', manifest['lifecycle']),
        ('preflight', manifest['development_preflight']),
        ('task plan', manifest['task_plan']),
        ('contract', manifest['lifecycle_contract']),
    ):
        path = obj.get('path') or obj.get('artifact')
        if not path or sha(git_bytes(path)) != obj.get('sha256'):
            fail(label + ' digest mismatch')
    if manifest['research'].get('change_required') is not True:
        fail('required Windows packaging delta is not frozen')

    changed = branch_changed()
    extra = changed - ALLOWED
    if extra:
        fail('out-of-scope branch changes: ' + str(sorted(extra)))

    base_tauri_bytes = git_bytes('apps/desktop/src-tauri/tauri.conf.json')
    if base_tauri_bytes != git_bytes('apps/desktop/src-tauri/tauri.conf.json', CANONICAL_BASE):
        fail('accepted base Tauri config was mutated')
    if sha(base_tauri_bytes) != BASE_TAURI_SHA:
        fail('accepted base Tauri config digest drifted')
    ownership_bytes = git_bytes('installer/windows/owned-payload.v1.json')
    if ownership_bytes != git_bytes('installer/windows/owned-payload.v1.json', CANONICAL_BASE):
        fail('owned payload manifest was mutated')
    if sha(ownership_bytes) != OWNERSHIP_SHA:
        fail('owned payload manifest digest drifted')

    if base_tauri.get('productName') != 'VSN Dev Platform' or base_tauri.get('version') != '0.38.1':
        fail('base product identity drifted')
    if base_tauri.get('identifier') != 'dev.vsn.platform':
        fail('base bundle identifier drifted')

    owned = {entry.get('relative_path'): entry for entry in ownership.get('owned_files', [])}
    for path in ('bin/vsn.exe', 'bin/vsn-agent.exe'):
        entry = owned.get(path)
        if not entry or entry.get('placement_owner') != '03.10':
            fail(f'ownership contract missing 03.10 payload: {path}')

    hook = windows_tauri.get('build', {}).get('beforeBundleCommand')
    if not isinstance(hook, str) or 'pkg03-0310-stage-windows-payload.ps1' not in hook:
        fail('Windows beforeBundleCommand is not task-owned staging hook')
    resources = windows_tauri.get('bundle', {}).get('resources')
    expected_resources = {
        '../../../target/pkg03/03.10/vsn.exe': 'bin/vsn.exe',
        '../../../target/pkg03/03.10/vsn-agent.exe': 'bin/vsn-agent.exe',
    }
    if resources != expected_resources:
        fail('Windows bundle resource map does not match frozen destinations')
    if 'externalBin' in windows_tauri.get('bundle', {}):
        fail('03.10 must not use sidecar externalBin semantics')

    stage_text = STAGE.read_text(encoding='utf-8')
    for token in (
        'cargo build --locked --release -p vsn -p vsn-agent',
        'target/release/vsn.exe', 'target/release/vsn-agent.exe',
        'target/pkg03/03.10/vsn.exe', 'target/pkg03/03.10/vsn-agent.exe',
        '[Security.Cryptography.SHA256]::Create()',
    ):
        if token not in stage_text:
            fail('staging script missing frozen token: ' + token)

    if not HARNESS.is_file() or not WORKFLOW.is_file():
        fail('03.10 certification surface incomplete')
    harness = HARNESS.read_text(encoding='utf-8')
    for token in (
        'bin\\vsn.exe', 'bin\\vsn-agent.exe', '--version', '--once',
        'VSN Dev Platform.exe', 'msiexec.exe', 'Get-FileHash',
        'service_registration_claimed', 'path_environment_mutation_claimed',
    ):
        if token not in harness:
            fail('03.10 lifecycle harness missing token: ' + token)
    workflow = WORKFLOW.read_text(encoding='utf-8')
    for token in (
        'windows-2025', '22.12.0', '1.97.1', 'tauri-cli 2.11.4',
        'build --bundles nsis,msi', 'pkg03-0310-cli-agent-payload',
        BASE_TAURI_SHA, OWNERSHIP_SHA,
    ):
        if token not in workflow:
            fail('03.10 workflow missing frozen token: ' + token)

    tasks = {item['id']: item for item in tracker.get('tasks', [])}
    if tasks.get('03.02', {}).get('status') != 'DONE' or tasks.get('03.05', {}).get('status') != 'DONE':
        fail('03.10 dependencies are not DONE')
    state = tasks.get('03.10', {}).get('status')
    if state not in ('READY', 'DONE'):
        fail('03.10 must be READY or DONE')
    if state == 'READY' and tasks['03.10'].get('depends_on') != ['03.02', '03.05']:
        fail('03.10 dependency contract drifted')
    if state == 'DONE':
        evidence = tasks['03.10'].get('evidence')
        if not isinstance(evidence, dict):
            fail('DONE 03.10 is missing evidence')
        for key in ('source_commit', 'workflow_run', 'job', 'artifact', 'artifact_digest', 'evidence_sha256'):
            if not evidence.get(key):
                fail('DONE 03.10 evidence missing ' + key)

    pkg03 = {item['id']: item for item in master.get('packages', [])}.get('PKG-03', {})
    if not pkg03 or tracker.get('required') != 25 or pkg03.get('required') != 25:
        fail('PKG-03 denominator drifted')

    authority = manifest['authority']
    if authority.get('base_tauri_config_mutation_allowed') is not False:
        fail('base Tauri mutation authority widened')
    if authority.get('windows_platform_config_allowed') is not True:
        fail('Windows platform config authority missing')
    for key in (
        'custom_nsis_template_allowed', 'custom_wix_template_allowed',
        'service_registration_allowed', 'path_environment_mutation_allowed',
        'acl_mutation_allowed', 'silent_or_passive_deployment_allowed',
        'signing_secret_access_allowed', 'updater_mutation_allowed',
        'delegated_scope_may_expand',
    ):
        if authority.get(key) is not False:
            fail('authority widened: ' + key)

    print(json.dumps({
        'valid': True,
        'task': '03.10',
        'state': state,
        'branch_changed_paths': sorted(changed),
        'base_tauri_unchanged': True,
        'resource_destinations': sorted(expected_resources.values()),
    }, indent=2))


if __name__ == '__main__':
    main()
