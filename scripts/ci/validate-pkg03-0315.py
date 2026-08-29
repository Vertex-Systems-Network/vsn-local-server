#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HISTORICAL_BASE = "4f5e8ab30f030e758c52c4ca4ac08f73f896247a"
LIVE_BASE = "0eaa4abb7c5e817334f13672952a5901fbbc8fa9"
HISTORICAL_HEAD = "5cc0be73873e998ba33b0b8212e152bfcbc19603"
TASK = "03.15"
MANIFEST_PATH = ROOT / ".ai/manifests/pkg03-0315-installer-diagnostics.v1.json"
TRACKER_PATH = ROOT / "certification/pkg03-windows-installer-v1.json"
RECONCILIATION = ".ai/changes/PKG03-0315-LIVE-MAIN-RECONCILIATION-2026-08-29.md"

PLANNING = {
    "research": ".ai/features/pkg03-0315/research.md",
    "lifecycle": ".ai/features/pkg03-0315/lifecycle-review.md",
    "development_preflight": ".ai/features/pkg03-0315/development-preflight.md",
    "task_plan": ".ai/plans/pkg03-0315-installer-diagnostics-v1.md",
    "lifecycle_contract": "docs/PKG03-INSTALLER-DIAGNOSTICS-LIFECYCLE-V1.md",
}
IMPLEMENTATION = {
    "scripts/ci/validate-pkg03-0315.py",
    "scripts/ci/pkg03-0315-installer-diagnostics.ps1",
    ".github/workflows/pkg03-0315-installer-diagnostics.yml",
}
STATE = {
    "certification/pkg03-windows-installer-v1.json",
    "docs/MASTER-EXECUTION-STATUS.json",
}
ALLOWED = set(PLANNING.values()) | {MANIFEST_PATH.relative_to(ROOT).as_posix(), RECONCILIATION} | IMPLEMENTATION | STATE
PROTECTED_PRODUCT_INPUTS = {
    "apps/desktop/src-tauri/tauri.conf.json",
    "apps/desktop/src-tauri/tauri.per-machine.conf.json",
    "installer/windows/owned-payload.v1.json",
    "apps/desktop/package.json",
    "apps/desktop/package-lock.json",
    "Cargo.lock",
}


def fail(message: str) -> None:
    raise SystemExit("PKG-03 03.15 validation failed: " + message)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def changed_paths() -> list[str]:
    return [line for line in git("diff", "--name-only", f"{LIVE_BASE}...HEAD").splitlines() if line]


def require_ancestor(ancestor: str, descendant: str = "HEAD") -> None:
    result = subprocess.run(["git", "merge-base", "--is-ancestor", ancestor, descendant], cwd=ROOT)
    if result.returncode != 0:
        fail(f"required ancestor missing: {ancestor} is not an ancestor of {descendant}")


def main() -> None:
    if not MANIFEST_PATH.is_file():
        fail("manifest missing")
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    tracker = json.loads(TRACKER_PATH.read_text(encoding="utf-8"))

    identity = (
        manifest.get("feature_id"), manifest.get("task_id"),
        manifest.get("linear_issue"), manifest.get("version"), manifest.get("status"),
    )
    if identity != ("pkg03-0315-installer-diagnostics", TASK, "ABD-90", "1.0.0", "frozen"):
        fail("manifest identity/version/status mismatch")
    if manifest.get("canonical_base_sha") != HISTORICAL_BASE:
        fail("frozen historical planning base mismatch")
    if manifest.get("parent_plan", {}).get("sha256") != "9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e":
        fail("parent plan digest declaration drifted")
    if manifest.get("research", {}).get("change_required") is not False:
        fail("03.15 must remain certification-only")

    require_ancestor(HISTORICAL_HEAD)
    require_ancestor(LIVE_BASE)
    reconciliation_path = ROOT / RECONCILIATION
    if not reconciliation_path.is_file():
        fail("live-main reconciliation record missing")
    reconciliation = reconciliation_path.read_text(encoding="utf-8")
    for token in (HISTORICAL_BASE, HISTORICAL_HEAD, LIVE_BASE, "evidence-only live-main reconciliation"):
        if token not in reconciliation:
            fail(f"reconciliation record missing token: {token}")

    digest_errors: list[str] = []
    for key, relative in PLANNING.items():
        path = ROOT / relative
        if not path.is_file():
            fail(f"planning artifact missing: {relative}")
        expected = manifest.get(key, {}).get("sha256")
        actual = sha256(path)
        if expected != actual:
            digest_errors.append(f"{key}: expected={expected} actual={actual}")
    if digest_errors:
        fail("planning digest mismatch(es):\n" + "\n".join(digest_errors))

    for relative in IMPLEMENTATION:
        if not (ROOT / relative).is_file():
            fail(f"implementation artifact missing: {relative}")

    paths = changed_paths()
    unexpected = sorted(set(paths) - ALLOWED)
    if unexpected:
        fail(f"live-main reconciled branch changed unauthorized paths: {unexpected}")
    protected = sorted(set(paths) & PROTECTED_PRODUCT_INPUTS)
    if protected:
        fail(f"03.15 illegally changed accepted product inputs: {protected}")

    tasks = {task["id"]: task for task in tracker.get("tasks", [])}
    for dependency in ("03.06", "03.07", "03.08"):
        if tasks.get(dependency, {}).get("status") != "DONE":
            fail(f"dependency {dependency} is not canonically DONE")
    state = tasks.get(TASK, {}).get("status")
    if state not in {"READY", "DONE"}:
        fail(f"tracker state is not READY/DONE: {state}")

    locked = manifest.get("locked_inputs", {})
    expected_locked = {
        "node": "22.12.0",
        "rust": "1.97.1",
        "product_version": "0.38.1",
        "tauri_cli": "2.11.4",
        "product_name": "VSN Dev Platform",
        "msi_success_exit_code": 0,
        "msi_user_cancel_exit_code": 1602,
        "nsis_success_exit_code": 0,
        "nsis_setup_user_cancel_exit_code": 1,
    }
    for key, value in expected_locked.items():
        if locked.get(key) != value:
            fail(f"locked input drifted: {key}")

    acceptance = manifest.get("acceptance", {})
    if acceptance.get("msi_verbose_logging") is not True:
        fail("MSI verbose logging must be required")
    if acceptance.get("nsis_native_persistent_log_claimed") is not False:
        fail("stock NSIS persistent logging must remain a nonclaim")
    if acceptance.get("nsis_uninstaller_cancel_exit_code_claimed") is not False:
        fail("NSIS uninstaller cancellation exit code must remain a nonclaim")

    authority = manifest.get("authority", {})
    for key in (
        "planning_product_mutation_allowed",
        "tauri_config_mutation_allowed",
        "installer_template_or_hook_mutation_allowed",
        "nsis_toolchain_mutation_allowed",
        "service_registration_allowed",
        "acl_mutation_allowed",
        "firewall_hosts_dns_trust_mutation_allowed",
        "silent_or_passive_deployment_allowed",
        "reboot_semantics_claim_allowed",
        "rollback_or_recovery_claim_allowed",
        "signing_secret_access_allowed",
        "updater_mutation_allowed",
        "delegated_scope_may_expand",
    ):
        if authority.get(key) is not False:
            fail(f"authority widened: {key}")

    harness = (ROOT / "scripts/ci/pkg03-0315-installer-diagnostics.ps1").read_text(encoding="utf-8")
    workflow = (ROOT / ".github/workflows/pkg03-0315-installer-diagnostics.yml").read_text(encoding="utf-8")
    for token in (
        "nsis-current-user-success", "nsis-per-machine-success", "nsis-setup-cancel",
        "msi-install-success", "msi-uninstall-success", "msi-install-cancel",
        "1602", "expected_exit_code = 1", "/L*V", "ui-observations.json", "ui-actions.json",
    ):
        if token not in harness:
            fail(f"lifecycle harness missing frozen token: {token}")
    for forbidden in ("/quiet", "/passive", "/qn", "/qb", "LogSet", "NSIS_CONFIG_LOG"):
        if forbidden.lower() in harness.lower():
            fail(f"lifecycle harness contains forbidden/unclaimed token: {forbidden}")
    for token in (
        "runs-on: windows-2025", "22.12.0", "1.97.1", "tauri-cli 2.11.4",
        "tauri.per-machine.conf.json", "--bundles nsis", "--bundles msi",
        "pkg03-0315-installer-diagnostics.ps1", "pkg03-0315-installer-diagnostics",
    ):
        if token not in workflow:
            fail(f"workflow missing frozen token: {token}")

    print(json.dumps({
        "valid": True,
        "task": TASK,
        "state": state,
        "historical_planning_base": HISTORICAL_BASE,
        "live_execution_base": LIVE_BASE,
        "dependencies": {key: tasks[key]["status"] for key in ("03.06", "03.07", "03.08")},
        "branch_changed_paths": paths,
        "product_inputs_unchanged": True,
        "historical_evidence_canonical": False,
    }, indent=2))


if __name__ == "__main__":
    main()
