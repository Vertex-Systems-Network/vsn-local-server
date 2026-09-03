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
ALLOWED_PATHS = PLANNING_PATHS | {VALIDATOR_PATH, HARNESS_PATH, WORKFLOW_PATH}


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
    if (
        head_tracker.get("done") != 19
        or head_tracker.get("required") != 25
        or head_tracker.get("percent") != 76.0
        or head_tracker.get("active_task") != TASK
        or head_tracker.get("ready_tasks") != ["03.20", "03.22"]
        or head_tasks.get(TASK, {}).get("status") != "READY"
        or head_tasks.get("03.22", {}).get("status") != "READY"
    ):
        fail("HEAD changed canonical package state before accepted evidence")

    changed = [p for p in git_text("diff", "--name-only", f"{CURRENT_BASE}...HEAD").splitlines() if p]
    unexpected = sorted(set(changed) - ALLOWED_PATHS)
    if unexpected:
        fail(f"unauthorized changed paths: {unexpected}")
    missing = sorted(ALLOWED_PATHS - set(changed))
    if missing:
        fail(f"certification bundle missing required paths: {missing}")

    product_prefixes = (
        "apps/",
        "crates/",
        "package.json",
        "Cargo.toml",
        "Cargo.lock",
    )
    product_changes = [p for p in changed if p.startswith(product_prefixes)]
    if product_changes:
        fail(f"product/config mutation appeared before exact evidence: {product_changes}")

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
        "state": {"done": 19, "required": 25, "active_task": TASK, "ready_tasks": ["03.20", "03.22"]},
        "changed_paths": sorted(changed),
        "certification_first": True,
    }, indent=2))


if __name__ == "__main__":
    main()
