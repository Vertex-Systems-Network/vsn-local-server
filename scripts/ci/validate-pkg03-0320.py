#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

TASK = "03.20"
LINEAR = "ABD-95"
CURRENT_BASE = "73de463594650cb2ebc407957cbb010e8a0e4be8"
MANIFEST_PATH = Path(".ai/manifests/pkg03-0320-reboot-semantics.v1.json")
TRACKER_PATH = "certification/pkg03-windows-installer-v1.json"
PLANNING_PATHS = {
    ".ai/features/pkg03-0320/research.md",
    ".ai/features/pkg03-0320/lifecycle-review.md",
    ".ai/features/pkg03-0320/development-preflight.md",
    ".ai/plans/pkg03-0320-reboot-semantics-v1.md",
    ".ai/manifests/pkg03-0320-reboot-semantics.v1.json",
    "docs/PKG03-INSTALLER-REBOOT-SEMANTICS-V1.md",
}
VALIDATOR_PATH = "scripts/ci/validate-pkg03-0320.py"
HARNESS_PATH = "scripts/ci/pkg03-0320-reboot-semantics.ps1"
WORKFLOW_PATH = ".github/workflows/pkg03-0320-reboot-semantics.yml"
PROJECTION_PATHS = {
    "certification/pkg03-windows-installer-v1.json",
    "docs/MASTER-EXECUTION-STATUS.json",
    "README.md",
    ".ai/README.md",
    "docs/MASTER-EXECUTION-PLAN.md",
}
ALLOWED_PATHS = PLANNING_PATHS | {VALIDATOR_PATH, HARNESS_PATH, WORKFLOW_PATH}
ACCEPTED_EVIDENCE = {
    "source_commit": "02905520e620c953e0edf37dbbda9a6652179e1a",
    "workflow_run": 33767517711,
    "job": 100689057721,
    "artifact": 9899183827,
    "artifact_digest": "sha256:f685764725fcbf1c9a117077f6c8836f446b61abe0bad325aad001057606013c",
    "evidence_sha256": "c64b990e130889ee95a2f3a42795d4e91fa6217980b62c1774f242ce9c906fa4",
    "current_user_setup_sha256": "36703015bd0561670018bb8379f760592a7ff18ff28081de41ff9e1daec32de8",
    "per_machine_setup_sha256": "16c9ee76cacb72377ff41ba27a1d5b7286e42106610a61b636a23a78b6c6afc7",
    "msi_sha256": "d64aa9ab4957979a6dcc51a02db3d8fca83fed8536765f97579d2ca1a06a69a2",
    "product_code": "{F094B870-BD6C-4CCC-8A2B-ED442B6AD6AF}",
    "msi_norestart_install_log_sha256": "e530df2cb8d00b1337654c564d2a13c0281a1e32e4d4463cf8f80f2a3b1fb6dd",
    "msi_norestart_uninstall_log_sha256": "4bce3ace8a71d799b0f5a941023e5106571bad207f092a6f194b59b257ca7574",
    "inherited_0319_evidence_sha256": "f65337a5ba4200486412972a7edd9cc8b09d8016054666417a828ce300b3b6ee",
}


def fail(message: str) -> None:
    raise SystemExit(f"03.20 authority validation failed: {message}")


def git_text(*args: str) -> str:
    return subprocess.check_output(["git", *args], text=True).strip()


def tracked_bytes(path: str, ref: str = "HEAD") -> bytes:
    try:
        return subprocess.check_output(["git", "show", f"{ref}:{path}"])
    except subprocess.CalledProcessError as exc:
        fail(f"cannot read tracked artifact {ref}:{path} ({exc.returncode})")


def tracked_sha256(path: str, ref: str = "HEAD") -> str:
    return hashlib.sha256(tracked_bytes(path, ref)).hexdigest()


def ref_json(path: str, ref: str) -> dict:
    return json.loads(tracked_bytes(path, ref).decode("utf-8"))


def task_map(tracker: dict) -> dict[str, dict]:
    return {item.get("id"): item for item in tracker.get("tasks", [])}


def main() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    if manifest.get("task_id") != TASK or manifest.get("linear_issue") != LINEAR:
        fail("manifest task/Linear identity mismatch")
    if manifest.get("canonical_base_sha") != CURRENT_BASE or manifest.get("status") != "frozen":
        fail("canonical base/frozen status mismatch")
    if manifest.get("dependencies") != ["03.15", "03.19"] or manifest.get("lane") != "reboot":
        fail("dependency/lane contract mismatch")
    if manifest.get("research", {}).get("change_required") is not False:
        fail("03.20 must remain certification-first before exact failure evidence")

    bindings = [
        (manifest["research"]["artifact"], manifest["research"]["sha256"]),
        (manifest["lifecycle"]["artifact"], manifest["lifecycle"]["sha256"]),
        (manifest["development_preflight"]["artifact"], manifest["development_preflight"]["sha256"]),
        (manifest["task_plan"]["path"], manifest["task_plan"]["sha256"]),
        (manifest["lifecycle_contract"]["artifact"], manifest["lifecycle_contract"]["sha256"]),
        (manifest["parent_plan"]["path"], manifest["parent_plan"]["sha256"]),
    ]
    mismatches = []
    for path, expected in bindings:
        actual = tracked_sha256(path)
        if actual != expected:
            mismatches.append({"path": path, "expected": expected, "actual": actual})
    if mismatches:
        fail(f"frozen artifact digest mismatches: {json.dumps(mismatches, sort_keys=True)}")

    authority = manifest.get("authority", {})
    for key in (
        "planning_product_mutation_allowed",
        "tauri_config_mutation_allowed",
        "installer_template_or_hook_mutation_allowed",
        "product_runtime_mutation_allowed",
        "service_identity_mutation_allowed",
        "acl_mutation_allowed",
        "silent_deployment_acceptance_allowed",
        "signing_secret_access_allowed",
        "provenance_or_pkg05_implementation_allowed",
        "updater_mutation_allowed",
        "delegated_scope_may_expand",
    ):
        if authority.get(key) is not False:
            fail(f"authority widened: {key}")

    acceptance = manifest.get("acceptance", {})
    for key in (
        "exact_three_package_builds_required",
        "boot_identity_must_not_change",
        "pending_file_rename_probe_required",
        "pending_probe_exact_restore_required",
        "msi_norestart_required",
        "msi_really_suppress_evidence_required",
        "msi_system_reboot_pending_evidence_required",
        "reuse_0319_running_process_lifecycle_required",
        "tracked_repository_drift_zero_required",
    ):
        if acceptance.get(key) is not True:
            fail(f"acceptance flag missing: {key}")
    if acceptance.get("msi_success_codes") != [0, 3010]:
        fail("MSI success code set drifted")
    if acceptance.get("msi_reboot_initiated_code_forbidden") != 1641:
        fail("MSI reboot-initiated forbidden code drifted")

    base_tracker = ref_json(TRACKER_PATH, CURRENT_BASE)
    base_tasks = task_map(base_tracker)
    if (
        base_tracker.get("package_id") != "PKG-03"
        or base_tracker.get("done") != 19
        or base_tracker.get("required") != 25
        or base_tracker.get("percent") != 76.0
        or base_tracker.get("active_task") != TASK
        or base_tracker.get("ready_tasks") != ["03.20", "03.22"]
    ):
        fail("canonical base is not accepted 19/25 cursor 03.20")
    if base_tasks.get(TASK, {}).get("status") != "READY" or base_tasks.get(TASK, {}).get("depends_on") != ["03.15", "03.19"]:
        fail("03.20 canonical READY/dependency contract drifted")
    for dep in ("03.15", "03.19"):
        if base_tasks.get(dep, {}).get("status") != "DONE":
            fail(f"canonical dependency {dep} is not DONE")
    if base_tasks.get("03.22", {}).get("status") != "READY":
        fail("independent 03.22 readiness drifted")

    head_tracker = ref_json(TRACKER_PATH, "HEAD")
    head_tasks = task_map(head_tracker)
    head_task = head_tasks.get(TASK, {})
    projection_mode = (
        head_tracker.get("package_id") == "PKG-03"
        and head_tracker.get("done") == 20
        and head_tracker.get("required") == 25
        and head_tracker.get("percent") == 80.0
        and head_tracker.get("active_task") == "03.21"
        and head_tracker.get("ready_tasks") == ["03.21", "03.22"]
        and head_task.get("status") == "DONE"
    )
    if projection_mode:
        evidence = head_task.get("evidence", {})
        for key, expected in ACCEPTED_EVIDENCE.items():
            if evidence.get(key) != expected:
                fail(f"accepted projection evidence mismatch: {key}")
        if head_tasks.get("03.21", {}).get("status") != "READY":
            fail("03.21 was not unblocked by accepted 03.20")
        if head_tasks.get("03.22", {}).get("status") != "READY":
            fail("03.22 readiness drifted in 03.20 projection")
        for task_id in ("03.23", "03.24", "03.25"):
            if head_tasks.get(task_id, {}).get("status") != "BLOCKED":
                fail(f"projection prematurely unblocked {task_id}")
    else:
        if (
            head_tracker.get("done") != 19
            or head_tracker.get("required") != 25
            or head_tracker.get("percent") != 76.0
            or head_tracker.get("active_task") != TASK
            or head_tracker.get("ready_tasks") != ["03.20", "03.22"]
            or head_task.get("status") != "READY"
            or head_tasks.get("03.22", {}).get("status") != "READY"
        ):
            fail("HEAD is neither 03.20 implementation state nor accepted projection")

    changed = [p for p in git_text("diff", "--name-only", f"{CURRENT_BASE}...HEAD").splitlines() if p]
    unexpected = []
    for path in changed:
        if path in ALLOWED_PATHS or (projection_mode and path in PROJECTION_PATHS):
            continue
        unexpected.append(path)
    if unexpected:
        fail(f"unauthorized changed paths: {unexpected}")
    if (not projection_mode) and any(p in PROJECTION_PATHS for p in changed):
        fail("canonical projection appeared before accepted evidence")
    if projection_mode and not PROJECTION_PATHS.issubset(set(changed)):
        fail("accepted projection is missing one or more canonical state files")

    product_prefixes = (
        "apps/",
        "crates/",
        "package.json",
        "Cargo.toml",
        "Cargo.lock",
    )
    product_changes = [p for p in changed if p.startswith(product_prefixes)]
    if product_changes:
        fail(f"product/config mutation appeared in 03.20 scope: {product_changes}")

    harness = tracked_bytes(HARNESS_PATH).decode("utf-8")
    required_tokens = (
        "PendingFileRenameOperations",
        "MsiSystemRebootPending",
        "ReallySuppress",
        "/norestart",
        "3010",
        "1641",
        "pkg03-0319-running-processes.ps1",
        "LastBootUpTime",
    )
    for token in required_tokens:
        if token not in harness:
            fail(f"03.20 harness missing frozen token: {token}")
    for forbidden in ("Restart-Computer", "shutdown.exe", "InitiateSystemShutdown", "taskkill"):
        if forbidden.lower() in harness.lower():
            fail(f"03.20 harness contains forbidden reboot/process-kill primitive: {forbidden}")

    workflow = tracked_bytes(WORKFLOW_PATH).decode("utf-8")
    for token in (
        "windows-2025",
        "Validate frozen 03.20 authority",
        "Build exact-head current-user NSIS",
        "Build exact-head per-machine NSIS",
        "Build exact-head MSI/WiX",
        "Exercise exact-head reboot semantics",
        "Verify exact 03.20 evidence",
        "pkg03-0320-reboot-semantics",
    ):
        if token not in workflow:
            fail(f"03.20 workflow missing frozen token: {token}")

    print(json.dumps({
        "valid": True,
        "task": TASK,
        "linear": LINEAR,
        "canonical_base": CURRENT_BASE,
        "mode": "accepted-projection" if projection_mode else "implementation",
        "state": head_task.get("status"),
        "head_progress": {
            "done": head_tracker.get("done"),
            "required": head_tracker.get("required"),
            "percent": head_tracker.get("percent"),
            "active_task": head_tracker.get("active_task"),
            "ready_tasks": head_tracker.get("ready_tasks"),
        },
        "changed_paths": sorted(changed),
        "certification_first": True,
    }, indent=2))


if __name__ == "__main__":
    main()
