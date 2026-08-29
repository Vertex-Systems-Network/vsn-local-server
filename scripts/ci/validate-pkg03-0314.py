#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HISTORICAL_BASE = "0eaa4abb7c5e817334f13672952a5901fbbc8fa9"
LIVE_BASE = "b4fe7d07503b13ba0f3d2fcd1741a40163086de7"
TASK = "03.14"
MANIFEST_PATH = ROOT / ".ai/manifests/pkg03-0314-payload-integrity.v1.json"
TRACKER_PATH = ROOT / "certification/pkg03-windows-installer-v1.json"

PLANNING = {
    "research": ".ai/features/pkg03-0314/research.md",
    "lifecycle": ".ai/features/pkg03-0314/lifecycle-review.md",
    "development_preflight": ".ai/features/pkg03-0314/development-preflight.md",
    "task_plan": ".ai/plans/pkg03-0314-payload-integrity-v1.md",
    "lifecycle_contract": "docs/PKG03-INSTALLED-PAYLOAD-INTEGRITY-V1.md",
}
IMPLEMENTATION = {
    "scripts/ci/validate-pkg03-0314.py",
    "scripts/ci/pkg03-0314-payload-integrity.ps1",
    ".github/workflows/pkg03-0314-payload-integrity.yml",
}
STATE = {
    "certification/pkg03-windows-installer-v1.json",
    "docs/MASTER-EXECUTION-STATUS.json",
    "README.md",
    ".ai/README.md",
    "docs/MASTER-EXECUTION-PLAN.md",
}
ALLOWED = set(PLANNING.values()) | {MANIFEST_PATH.relative_to(ROOT).as_posix()} | IMPLEMENTATION | STATE
PROTECTED_PRODUCT_INPUTS = {
    "apps/desktop/src-tauri/tauri.conf.json",
    "apps/desktop/src-tauri/tauri.windows.conf.json",
    "apps/desktop/src-tauri/tauri.per-machine.conf.json",
    "apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs",
    "installer/windows/owned-payload.v1.json",
    "scripts/ci/pkg03-0310-stage-windows-payload.ps1",
    "apps/desktop/package.json",
    "apps/desktop/package-lock.json",
    "Cargo.lock",
}


def fail(message: str) -> None:
    raise SystemExit("PKG-03 03.14 validation failed: " + message)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def changed_paths() -> list[str]:
    return [line for line in git("diff", "--name-only", f"{LIVE_BASE}...HEAD").splitlines() if line]


def blob(path: str) -> str:
    return git("rev-parse", f"HEAD:{path}")


def main() -> None:
    if not MANIFEST_PATH.is_file():
        fail("manifest missing")
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    tracker = json.loads(TRACKER_PATH.read_text(encoding="utf-8"))

    identity = (
        manifest.get("feature_id"), manifest.get("task_id"), manifest.get("linear_issue"),
        manifest.get("version"), manifest.get("status"), manifest.get("canonical_base_sha"),
    )
    expected_identity = (
        "pkg03-0314-payload-integrity", TASK, "ABD-89", "1.0.0", "frozen", HISTORICAL_BASE,
    )
    if identity != expected_identity:
        fail("manifest identity/version/status/base mismatch")
    if manifest.get("parent_plan", {}).get("sha256") != "9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e":
        fail("parent plan digest declaration drifted")
    if manifest.get("research", {}).get("change_required") is not False:
        fail("03.14 must remain certification-only")

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

    paths = changed_paths()
    unexpected = sorted(set(paths) - ALLOWED)
    if unexpected:
        fail(f"branch changed unauthorized paths: {unexpected}")
    protected = sorted(set(paths) & PROTECTED_PRODUCT_INPUTS)
    if protected:
        fail(f"03.14 illegally changed accepted product inputs: {protected}")

    locked = manifest.get("locked_inputs", {})
    expected_locked = {
        "node": "22.12.0",
        "rust": "1.97.1",
        "product_version": "0.38.1",
        "tauri_cli": "2.11.4",
        "product_name": "VSN Dev Platform",
    }
    for key, value in expected_locked.items():
        if locked.get(key) != value:
            fail(f"locked input drifted: {key}")
    if locked.get("dependency_tasks") != ["03.06", "03.07", "03.08", "03.10"]:
        fail("dependency task set drifted")
    if locked.get("owned_relative_paths") != ["VSN Dev Platform.exe", "bin/vsn.exe", "bin/vsn-agent.exe"]:
        fail("owned relative path set drifted")

    expected_blobs = {
        "tauri_config": ("apps/desktop/src-tauri/tauri.conf.json", "62215d58a5fbf3a0c3098b4cf5c39bea497d1d7a"),
        "tauri_windows_config": ("apps/desktop/src-tauri/tauri.windows.conf.json", "54883cf5cf510b64785529e13c554d622d01f252"),
        "owned_payload_manifest": ("installer/windows/owned-payload.v1.json", "641bbdc4c106fcb36cc232c8a74549e42df1749c"),
        "payload_staging_script": ("scripts/ci/pkg03-0310-stage-windows-payload.ps1", "2a401c2df76720f92c5003e8d5c26c7e99ec0d6c"),
    }
    declared_blobs = locked.get("git_blobs", {})
    for key, (path, expected_blob) in expected_blobs.items():
        if declared_blobs.get(key) != expected_blob:
            fail(f"locked Git blob declaration drifted: {key}")
        actual_blob = blob(path)
        if actual_blob != expected_blob:
            fail(f"accepted input Git blob drifted: {path}: {actual_blob} != {expected_blob}")

    tasks = {task["id"]: task for task in tracker.get("tasks", [])}
    for dependency in ("03.06", "03.07", "03.08", "03.10"):
        if tasks.get(dependency, {}).get("status") != "DONE":
            fail(f"dependency {dependency} is not canonically DONE")
    state = tasks.get(TASK, {}).get("status")
    if state not in {"READY", "DONE"}:
        fail(f"tracker state is not READY/DONE: {state}")
    if state != "DONE":
        premature_state = sorted(set(paths) & STATE)
        if premature_state:
            fail(f"pre-acceptance branch may not project canonical state: {premature_state}")

    authority = manifest.get("authority", {})
    required_false = (
        "planning_product_mutation_allowed",
        "product_runtime_mutation_allowed",
        "tauri_config_mutation_allowed",
        "installer_template_or_hook_mutation_allowed",
        "service_registration_or_coordination_allowed",
        "acl_mutation_allowed",
        "firewall_hosts_dns_trust_mutation_allowed",
        "path_environment_mutation_allowed",
        "repair_or_reinstall_execution_allowed",
        "signing_secret_access_allowed",
        "updater_mutation_allowed",
        "delegated_scope_may_expand",
    )
    for key in required_false:
        if authority.get(key) is not False:
            fail(f"authority widened: {key}")
    if authority.get("test_fixture_perturbation_and_exact_restore_allowed") is not True:
        fail("bounded fixture perturbation authority missing")

    acceptance = manifest.get("acceptance", {})
    expected_acceptance = {
        "healthy_match_all_owned_all_lifecycles": True,
        "current_user_missing_and_tamper_all_owned": True,
        "machine_missing_and_tamper_desktop_cli_only": True,
        "agent_machine_destructive_probe_allowed": False,
        "repair_execution_claimed": False,
        "tracked_repository_drift_zero": True,
    }
    for key, value in expected_acceptance.items():
        if acceptance.get(key) != value:
            fail(f"acceptance contract drifted: {key}")

    for relative in IMPLEMENTATION:
        if relative in paths and not (ROOT / relative).is_file():
            fail(f"implementation artifact missing: {relative}")

    harness_path = ROOT / "scripts/ci/pkg03-0314-payload-integrity.ps1"
    workflow_path = ROOT / ".github/workflows/pkg03-0314-payload-integrity.yml"
    if harness_path.is_file():
        harness = harness_path.read_text(encoding="utf-8")
        for token in (
            "MATCH", "MISSING", "HASH_MISMATCH", "repair_required",
            "nsis-current-user", "nsis-per-machine", "wix-per-machine",
            "bin\\vsn.exe", "bin\\vsn-agent.exe", "VSN Dev Platform.exe",
        ):
            if token not in harness:
                fail(f"harness missing frozen token: {token}")
        for forbidden in (
            "Start-Service", "Stop-Service", "Restart-Service", "sc.exe",
            "REINSTALLMODE", "MsiReinstallProduct", " /f", "/fa", "/fu", "/fv",
        ):
            if forbidden.lower() in harness.lower():
                fail(f"harness contains forbidden service/repair token: {forbidden}")
    if workflow_path.is_file():
        workflow = workflow_path.read_text(encoding="utf-8")
        for token in (
            "runs-on: windows-2025", "22.12.0", "1.97.1", "tauri-cli 2.11.4",
            "tauri.per-machine.conf.json", "pkg03-0314-payload-integrity.ps1",
            "pkg03-0314-payload-integrity",
        ):
            if token not in workflow:
                fail(f"workflow missing frozen token: {token}")

    print(json.dumps({
        "valid": True,
        "task": TASK,
        "state": state,
        "historical_planning_base": HISTORICAL_BASE,
        "live_execution_base": LIVE_BASE,
        "dependencies": {key: tasks[key]["status"] for key in ("03.06", "03.07", "03.08", "03.10")},
        "branch_changed_paths": paths,
        "accepted_input_blobs_unchanged": True,
        "product_mutation": False,
    }, indent=2))


if __name__ == "__main__":
    main()
