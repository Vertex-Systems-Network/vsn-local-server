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
}
# The frozen manifest retains the historical planning base. Resumed execution
# is authorized from the separately recorded live-main reconciliation base.
ALLOWED = (
    set(PLANNING.values())
    | {MANIFEST_PATH.relative_to(ROOT).as_posix(), RECONCILIATION_PATH}
    | IMPLEMENTATION
    | STATE
)


def fail(message: str) -> None:
    raise SystemExit(message)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def assert_ancestor(ancestor: str, descendant: str = "HEAD") -> None:
    result = subprocess.run(
        ["git", "merge-base", "--is-ancestor", ancestor, descendant],
        cwd=ROOT,
        check=False,
    )
    if result.returncode != 0:
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

paths = changed_paths()
unexpected = sorted(set(paths) - ALLOWED)
if unexpected:
    fail(f"03.13 branch changed unauthorized paths from live main: {unexpected}")

# Canonical dependency eligibility must come from the refreshed live-main
# tracker, not from historical/concurrent branch projections.
tasks = {task["id"]: task for task in tracker.get("tasks", [])}
for dependency in ("03.06", "03.07", "03.08"):
    if tasks.get(dependency, {}).get("status") != "DONE":
        fail(f"03.13 dependency {dependency} is not canonically DONE")
if tasks.get(TASK, {}).get("status") not in {"READY", "DONE"}:
    fail(f"03.13 tracker state is not READY/DONE: {tasks.get(TASK, {}).get('status')}")

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

# This certification task is not allowed to solve failures by changing product
# configuration. The live-base allowed-path gate above enforces this mechanically.
for protected in (
    "apps/desktop/src-tauri/tauri.conf.json",
    "apps/desktop/src-tauri/tauri.per-machine.conf.json",
    "installer/windows/owned-payload.v1.json",
):
    if protected in paths:
        fail(f"03.13 illegally changed protected product input: {protected}")

# State files are projection-only. They may appear only once 03.13 is DONE.
state_changed = sorted(set(paths) & STATE)
if tasks.get(TASK, {}).get("status") != "DONE" and state_changed:
    fail(f"03.13 pre-acceptance branch changed canonical state files: {state_changed}")

print(json.dumps({
    "valid": True,
    "task": TASK,
    "state": tasks[TASK]["status"],
    "historical_planning_base": HISTORICAL_BASE,
    "live_execution_base": LIVE_BASE,
    "dependencies": {key: tasks[key]["status"] for key in ("03.06", "03.07", "03.08")},
    "branch_changed_paths": paths,
    "read_only_system_boundary": True,
}, indent=2))
