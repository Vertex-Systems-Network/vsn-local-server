#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HISTORICAL_BASE = "0eaa4abb7c5e817334f13672952a5901fbbc8fa9"
LIVE_BASE = "66c729381e343f45e34ec42a14ca962d9e2baf19"
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
ACCEPTED_EVIDENCE = {
    "source_commit": "8ef4f984b80678131b7ae6a28362ff33bbc9de08",
    "workflow_run": 33261413560,
    "job": 99123894885,
    "artifact": 9717571333,
    "artifact_digest": "sha256:d267882ca0e01cc98a6a24db24b2a41de587bbf2cea985a069c559290ee7144a",
    "evidence_sha256": "76715eafa04ef029e4eff0c98a58037594bb86cb39812da97aa2d145a9ba7bb4",
    "current_user_setup_sha256": "e24640f6ca79226e29d0a3c6522924d6e9db6c013ef5fceef7cd31e516a0b0",
    "per_machine_setup_sha256": "3e0f6452c5466690b9066d8744c4642f01622b03ffceab31905e549e65341743",
    "msi_sha256": "8fdabd11cd91ae23074e181399195b8ad4faeeb1fed4f42e7045ae774b1a8898",
    "product_code": "{5CB5B2DB-F083-4476-B1D6-278DD02AEC0A}",
    "current_user_desktop_sha256": "2be925416d9955065067a5baceee8d042965cf2aeec8b7802edd26fbfa30ef87",
    "per_machine_desktop_sha256": "407c5fd128f6922537103fbf38447db743c0b43d4c8f226dbe89c346d2e24c23",
    "wix_desktop_sha256": "839da5154a269e2c76f99b04c8bf2f3cae997b03985d0861aff66b6806c033cf",
    "cli_sha256": "2499b8bfc004583015a47985385337696f14206a43a8fc992757c5b357bce4f3",
    "agent_sha256": "dff9de0fb69565cce1b515a0a51441eed8bfadfcab9c3ed3493ca8c086126239",
}


def fail(message: str) -> None:
    raise SystemExit("PKG-03 03.14 validation failed: " + message)


def sha256_tracked(relative: str) -> str:
    try:
        data = subprocess.check_output(["git", "show", f"HEAD:{relative}"], cwd=ROOT)
    except subprocess.CalledProcessError as exc:
        fail(f"cannot read tracked artifact from HEAD: {relative} ({exc.returncode})")
    return hashlib.sha256(data).hexdigest()


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def changed_paths() -> list[str]:
    return [line for line in git("diff", "--name-only", f"{LIVE_BASE}...HEAD").splitlines() if line]


def blob(path: str) -> str:
    return git("rev-parse", f"HEAD:{path}")


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
        actual = sha256_tracked(relative)
        if expected != actual:
            digest_errors.append(f"{key}: expected={expected} actual={actual}")
    if digest_errors:
        fail("planning digest mismatch(es):\n" + "\n".join(digest_errors))

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
    deps = ["03.06", "03.07", "03.08", "03.10"]
    for dependency in deps:
        if tasks.get(dependency, {}).get("status") != "DONE":
            fail(f"dependency {dependency} is not canonically DONE")
    task = tasks.get(TASK, {})
    state = task.get("status")
    if state not in {"READY", "DONE"}:
        fail(f"tracker state is not READY/DONE: {state}")
    if task.get("depends_on") != deps:
        fail("03.14 dependency contract drifted")

    descendant_mode = (
        state == "DONE"
        and tracker.get("required") == 25
        and isinstance(tracker.get("done"), int)
        and tracker.get("done") > 14
    )
    if descendant_mode:
        evidence = task.get("evidence", {})
        for key, expected in ACCEPTED_EVIDENCE.items():
            if evidence.get(key) != expected:
                fail(f"accepted 03.14 evidence drifted: {key}")
        require_ancestor(ACCEPTED_EVIDENCE["source_commit"])

    paths = changed_paths()
    if not descendant_mode:
        unexpected = sorted(set(paths) - ALLOWED)
        if unexpected:
            fail(f"branch changed unauthorized paths: {unexpected}")
        protected = sorted(set(paths) & PROTECTED_PRODUCT_INPUTS)
        if protected:
            fail(f"03.14 illegally changed accepted product inputs: {protected}")
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
        if not (ROOT / relative).is_file():
            fail(f"implementation artifact missing: {relative}")

    harness_path = ROOT / "scripts/ci/pkg03-0314-payload-integrity.ps1"
    workflow_path = ROOT / ".github/workflows/pkg03-0314-payload-integrity.yml"
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
        "mode": "accepted-descendant" if descendant_mode else "implementation-or-projection",
        "historical_planning_base": HISTORICAL_BASE,
        "live_execution_base": LIVE_BASE,
        "dependencies": {key: tasks[key]["status"] for key in deps},
        "branch_changed_paths": paths,
        "accepted_input_blobs_unchanged": True,
        "product_mutation": False,
    }, indent=2))


if __name__ == "__main__":
    main()
