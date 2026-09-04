#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TASK = "03.13"
MANIFEST_PATH = ROOT / ".ai/manifests/pkg03-0313-installer-nonmutation.v1.json"
TRACKER_PATH = ROOT / "certification/pkg03-windows-installer-v1.json"
HISTORICAL_BASE = "4f5e8ab30f030e758c52c4ca4ac08f73f896247a"
LIVE_BASE = "b4fe7d07503b13ba0f3d2fcd1741a40163086de7"
ACCEPTED_PROJECTION_HEAD = "66c729381e343f45e34ec42a14ca962d9e2baf19"
RECONCILIATION_PATH = ".ai/changes/PKG03-0313-LIVE-MAIN-RECONCILIATION-2026-08-29.md"

PLANNING = {
    "research": ".ai/features/pkg03-0313/research.md",
    "lifecycle": ".ai/features/pkg03-0313/lifecycle-review.md",
    "development_preflight": ".ai/features/pkg03-0313/development-preflight.md",
    "task_plan": ".ai/plans/pkg03-0313-installer-nonmutation-v1.md",
    "lifecycle_contract": "docs/PKG03-INSTALLER-NONMUTATION-LIFECYCLE-V1.md",
}
IMPLEMENTATION = {
    "scripts/ci/pkg03-0313-snapshot.ps1",
    "scripts/ci/pkg03-0313-installer-nonmutation.ps1",
    "scripts/ci/validate-pkg03-0313.py",
    ".github/workflows/pkg03-0313-installer-nonmutation.yml",
}
STATE = {
    "certification/pkg03-windows-installer-v1.json",
    "docs/MASTER-EXECUTION-STATUS.json",
    "README.md",
    ".ai/README.md",
    "docs/MASTER-EXECUTION-PLAN.md",
}
ALLOWED = (
    set(PLANNING.values())
    | {MANIFEST_PATH.relative_to(ROOT).as_posix(), RECONCILIATION_PATH}
    | IMPLEMENTATION
    | STATE
)
ACCEPTED_EVIDENCE = {
    "source_commit": "c3fcf953f29fe50f8636abb92d794dccf5bbde62",
    "workflow_run": 33259248660,
    "job": 99118236848,
    "artifact": 9717148303,
    "artifact_digest": "sha256:19b820104594ff6ad64981e2219c76fd0b4f1c81abe1b98471dad18272c03b59",
    "evidence_sha256": "4b1b1536082dcd84ca80e1d20d2186a8e1a87351f7711166453f61f08728d4ac",
    "current_user_setup_sha256": "c6cb74d878ac3576767e9fd9d2bbb3f248598ebc5685b38fad08351362b4a26a",
    "per_machine_setup_sha256": "d9220f0e6134d05e7d920b6048816f6c2e6c3d5f3a44ec6f86a8eaabfd01b54b",
    "msi_sha256": "4cf9a049b8743fc9a074ee31d24deeab476813349e5545134cfc569efb5be73f",
    "product_code": "{D92E4DDB-E60D-4F58-9F91-D9972567980E}",
    "snapshot_sha256": "2e5755dc191cb6c1df9fa4e92f59f931688024654b27c40fad155cbc71805f4c",
}


def fail(message: str) -> None:
    raise SystemExit(message)


def sha256(path: Path) -> str:
    # Frozen planning digests were accepted from the Windows checkout bytes.
    # Keep that exact byte contract (including checkout CRLF conversion) rather
    # than hashing the raw LF Git object with `git show`.
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def is_ancestor(ancestor: str, descendant: str = "HEAD") -> bool:
    return subprocess.run(
        ["git", "merge-base", "--is-ancestor", ancestor, descendant],
        cwd=ROOT,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    ).returncode == 0


def assert_ancestor(ancestor: str, descendant: str = "HEAD") -> None:
    if not is_ancestor(ancestor, descendant):
        fail(f"03.13 required ancestor missing: {ancestor} !<= {descendant}")


def changed_paths() -> list[str]:
    return [line for line in git("diff", "--name-only", f"{LIVE_BASE}...HEAD").splitlines() if line]


if not MANIFEST_PATH.is_file():
    fail("03.13 manifest missing")
if not (ROOT / RECONCILIATION_PATH).is_file():
    fail("03.13 live-main reconciliation record missing")

manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
tracker = json.loads(TRACKER_PATH.read_text(encoding="utf-8"))

if manifest.get("task_id") != TASK or manifest.get("linear_issue") != "ABD-88":
    fail("03.13 manifest identity mismatch")
if manifest.get("canonical_base_sha") != HISTORICAL_BASE:
    fail("03.13 historical frozen manifest base mismatch")
if manifest.get("status") != "frozen":
    fail("03.13 manifest must remain frozen")
if manifest.get("parent_plan", {}).get("sha256") != "9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e":
    fail("03.13 parent plan digest mismatch")

assert_ancestor(HISTORICAL_BASE)
assert_ancestor(LIVE_BASE)

digest_errors: list[str] = []
for key, relative in PLANNING.items():
    path = ROOT / relative
    if not path.is_file():
        fail(f"03.13 planning artifact missing: {relative}")
    expected = manifest.get(key, {}).get("sha256")
    actual = sha256(path)
    if expected != actual:
        digest_errors.append(f"{key}: expected={expected} actual={actual}")
if digest_errors:
    fail("03.13 planning digest mismatch(es):\n" + "\n".join(digest_errors))

for relative in IMPLEMENTATION:
    if not (ROOT / relative).is_file():
        fail(f"03.13 implementation artifact missing: {relative}")

tasks = {task["id"]: task for task in tracker.get("tasks", [])}
deps = ["03.06", "03.07", "03.08"]
for dependency in deps:
    if tasks.get(dependency, {}).get("status") != "DONE":
        fail(f"03.13 dependency {dependency} is not canonically DONE")
task = tasks.get(TASK, {})
state = task.get("status")
if state not in {"READY", "DONE"}:
    fail(f"03.13 tracker state is not READY/DONE: {state}")
if task.get("depends_on") != deps:
    fail("03.13 dependency contract drifted")

projection_state = (
    tracker.get("package_id") == "PKG-03"
    and tracker.get("done") == 13
    and tracker.get("required") == 25
    and tracker.get("percent") == 52.0
    and tracker.get("active_task") == "03.14"
    and tracker.get("active_tasks") == []
    and tracker.get("ready_tasks") == ["03.14", "03.15", "03.17"]
    and state == "DONE"
)
exact_head = git("rev-parse", "HEAD")
strict_projection_mode = projection_state and exact_head == ACCEPTED_PROJECTION_HEAD
descendant_mode = (
    tracker.get("package_id") == "PKG-03"
    and tracker.get("required") == 25
    and isinstance(tracker.get("done"), int)
    and tracker.get("done") >= 13
    and state == "DONE"
    and not strict_projection_mode
    and is_ancestor(ACCEPTED_PROJECTION_HEAD)
)

if strict_projection_mode or descendant_mode:
    evidence = task.get("evidence", {})
    for key, expected in ACCEPTED_EVIDENCE.items():
        if evidence.get(key) != expected:
            fail(f"03.13 accepted evidence drifted: {key}")
    assert_ancestor(ACCEPTED_EVIDENCE["source_commit"])
    assert_ancestor(ACCEPTED_PROJECTION_HEAD)
elif state == "DONE":
    fail("03.13 DONE state lacks accepted projection ancestry")

paths = changed_paths()
if not descendant_mode:
    unexpected = sorted(set(paths) - ALLOWED)
    if unexpected:
        fail(f"03.13 branch changed unauthorized paths from live main: {unexpected}")
    if strict_projection_mode and not STATE.issubset(set(paths)):
        fail("03.13 accepted projection is missing one or more canonical state files")

snapshot = (ROOT / "scripts/ci/pkg03-0313-snapshot.ps1").read_text(encoding="utf-8")
harness = (ROOT / "scripts/ci/pkg03-0313-installer-nonmutation.ps1").read_text(encoding="utf-8")
workflow = (ROOT / ".github/workflows/pkg03-0313-installer-nonmutation.yml").read_text(encoding="utf-8")

for token in (
    "Get-NetFirewallRule",
    "Get-NetFirewallPortFilter",
    "Get-DnsClientServerAddress",
    "Get-DnsClientNrptRule",
    r"Cert:\${location}\${store}",
    "Get-FileHash",
    "Assert-Pkg0313SnapshotEqual",
):
    if token not in snapshot:
        fail(f"03.13 snapshotter missing required token: {token}")

for forbidden in (
    "New-NetFirewallRule",
    "Set-NetFirewallRule",
    "Remove-NetFirewallRule",
    "Set-DnsClientServerAddress",
    "Set-DnsClientGlobalSetting",
    "Add-DnsClientNrptRule",
    "Set-DnsClientNrptRule",
    "Remove-DnsClientNrptRule",
    "Import-Certificate",
    "Remove-Item -LiteralPath Cert:",
):
    if forbidden.lower() in snapshot.lower() or forbidden.lower() in harness.lower():
        fail(f"03.13 read-only boundary contains forbidden mutation command: {forbidden}")

for lifecycle in ("nsis-current-user", "nsis-per-machine", "wix-per-machine"):
    if lifecycle not in harness:
        fail(f"03.13 harness missing lifecycle: {lifecycle}")
for token in ("baseline", "post-install", "post-uninstall", "ensure-safety-checkbox-off", "automatic_repair = $false"):
    if token not in harness:
        fail(f"03.13 harness missing contract token: {token}")

for token in (
    "runs-on: windows-2025",
    "22.12.0",
    "1.97.1",
    "tauri.per-machine.conf.json",
    "--bundles nsis",
    "--bundles msi",
    "pkg03-0313-installer-nonmutation.ps1",
    "pkg03-0313-installer-nonmutation",
):
    if token not in workflow:
        fail(f"03.13 workflow missing required token: {token}")

if not descendant_mode:
    for protected in (
        "apps/desktop/src-tauri/tauri.conf.json",
        "apps/desktop/src-tauri/tauri.per-machine.conf.json",
        "installer/windows/owned-payload.v1.json",
    ):
        if protected in paths:
            fail(f"03.13 illegally changed protected product input: {protected}")

    state_changed = sorted(set(paths) & STATE)
    if state != "DONE" and state_changed:
        fail(f"03.13 pre-acceptance branch changed canonical state files: {state_changed}")

mode = "accepted-descendant" if descendant_mode else ("accepted-projection" if strict_projection_mode else "implementation")
print(json.dumps({
    "valid": True,
    "task": TASK,
    "state": state,
    "mode": mode,
    "historical_planning_base": HISTORICAL_BASE,
    "live_execution_base": LIVE_BASE,
    "accepted_projection_head": ACCEPTED_PROJECTION_HEAD,
    "dependencies": {key: tasks[key]["status"] for key in deps},
    "branch_changed_paths": paths,
    "read_only_system_boundary": True,
}, indent=2))
