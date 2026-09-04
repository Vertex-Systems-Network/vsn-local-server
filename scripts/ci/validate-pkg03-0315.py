#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HISTORICAL_BASE = "4f5e8ab30f030e758c52c4ca4ac08f73f896247a"
LIVE_BASE = "bca82333c05e22c755d30d8c34d7394d80d6e547"
HISTORICAL_HEAD = "5cc0be73873e998ba33b0b8212e152bfcbc19603"
ACCEPTED_PROJECTION_HEAD = "f3afb66e588d01ff2e8cb37273ad413862a4edaf"
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
    "README.md",
    ".ai/README.md",
    "docs/MASTER-EXECUTION-PLAN.md",
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
ACCEPTED_EVIDENCE = {
    "source_commit": "4ce857a7ebed50335a8eb166bf56922fa6e30cb5",
    "workflow_run": 33261721842,
    "job": 99124698264,
    "artifact": 9717639764,
    "artifact_digest": "sha256:e105b34e19217fbe2d39e1d4a9ae4dc7dd33d7c9675d5ccafe2a899fdd7a9f5b",
    "evidence_sha256": "0d8ad32f68984f6ce6c6877a7b59a05e377770b69cf27fdea79b83e6b2060520",
    "current_user_setup_sha256": "c8533bf4b3ea162392fcb3d941d60298a7685e4907c85eb0b91076360fb389cf",
    "per_machine_setup_sha256": "14d310090d4cc1c895ac68c10c200926396d3ce23dad9daeeeb81cac35966e4a",
    "msi_sha256": "6c0e644bb5f31e5a751b1b9cc46180338bc78edfe3dd3aaf9ac827ec91a44351",
    "product_code": "{F388EF13-9261-4CD3-8FE7-F1763E8B2480}",
    "msi_install_log_sha256": "fb2eae3d3c5fed533af970adbcf9345a3579e263144bb382afd04ad10fb1419b",
    "msi_uninstall_log_sha256": "e9b23df798241d9893680e1e9d3079de5b7431822d2a05026d54a9d714f7b5ef",
    "msi_cancel_log_sha256": "84e07511eece4e5d837aa68d02aaab8508d946920f66a3812fe48a2e16a63968",
    "nsis_setup_cancel_exit_code": 1,
    "msi_cancel_exit_code": 1602,
}


def fail(message: str) -> None:
    raise SystemExit("PKG-03 03.15 validation failed: " + message)


def sha256(path: Path) -> str:
    # Frozen planning digests bind the accepted Windows checkout bytes,
    # including Git's checkout line-ending conversion.
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def changed_paths() -> list[str]:
    return [line for line in git("diff", "--name-only", f"{LIVE_BASE}...HEAD").splitlines() if line]


def is_ancestor(ancestor: str, descendant: str = "HEAD") -> bool:
    return subprocess.run(
        ["git", "merge-base", "--is-ancestor", ancestor, descendant],
        cwd=ROOT,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    ).returncode == 0


def require_ancestor(ancestor: str, descendant: str = "HEAD") -> None:
    if not is_ancestor(ancestor, descendant):
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

    tasks = {task["id"]: task for task in tracker.get("tasks", [])}
    deps = ["03.06", "03.07", "03.08"]
    for dependency in deps:
        if tasks.get(dependency, {}).get("status") != "DONE":
            fail(f"dependency {dependency} is not canonically DONE")
    task = tasks.get(TASK, {})
    state = task.get("status")
    if state not in {"READY", "DONE"}:
        fail(f"tracker state is not READY/DONE: {state}")
    if task.get("depends_on") != deps:
        fail("03.15 dependency contract drifted")

    projection_state = (
        tracker.get("package_id") == "PKG-03"
        and tracker.get("done") == 15
        and tracker.get("required") == 25
        and tracker.get("percent") == 60.0
        and tracker.get("active_task") == "03.16"
        and tracker.get("active_tasks") == []
        and tracker.get("ready_tasks") == ["03.16", "03.17", "03.18", "03.19", "03.22"]
        and state == "DONE"
    )
    exact_head = git("rev-parse", "HEAD")
    strict_projection_mode = projection_state and exact_head == ACCEPTED_PROJECTION_HEAD
    descendant_mode = (
        tracker.get("package_id") == "PKG-03"
        and tracker.get("required") == 25
        and isinstance(tracker.get("done"), int)
        and tracker.get("done") >= 15
        and state == "DONE"
        and not strict_projection_mode
        and is_ancestor(ACCEPTED_PROJECTION_HEAD)
    )
    if strict_projection_mode or descendant_mode:
        evidence = task.get("evidence", {})
        for key, expected in ACCEPTED_EVIDENCE.items():
            if evidence.get(key) != expected:
                fail(f"accepted 03.15 evidence drifted: {key}")
        require_ancestor(ACCEPTED_EVIDENCE["source_commit"])
        require_ancestor(ACCEPTED_PROJECTION_HEAD)
    elif state == "DONE":
        fail("03.15 DONE state lacks accepted projection ancestry")

    paths = changed_paths()
    if not descendant_mode:
        unexpected = sorted(set(paths) - ALLOWED)
        if unexpected:
            fail(f"live-main reconciled branch changed unauthorized paths: {unexpected}")
        protected = sorted(set(paths) & PROTECTED_PRODUCT_INPUTS)
        if protected:
            fail(f"03.15 illegally changed accepted product inputs: {protected}")
        if strict_projection_mode and not STATE.issubset(set(paths)):
            fail("accepted 03.15 projection is missing one or more canonical state files")
        if state != "DONE":
            premature_state = sorted(set(paths) & STATE)
            if premature_state:
                fail(f"pre-acceptance branch may not project canonical state: {premature_state}")

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
        "product_inputs_unchanged": True,
        "historical_evidence_canonical": strict_projection_mode or descendant_mode,
    }, indent=2))


if __name__ == "__main__":
    main()
