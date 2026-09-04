#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

TASK = "03.21"
LINEAR = "ABD-96"
CURRENT_BASE = "3edb4e1dcd2c062e7b2e270cde626c90a2c5459f"
ACCEPTED_PROJECTION_HEAD = "97a2fa620b9b438e5d211835bc0567a0c6d2be52"
MANIFEST_PATH = Path(".ai/manifests/pkg03-0321-silent-deployment.v1.json")
TRACKER_PATH = "certification/pkg03-windows-installer-v1.json"
PLANNING_PATHS = {
    ".ai/features/pkg03-0321/development-preflight.md",
    ".ai/features/pkg03-0321/lifecycle-review.md",
    ".ai/features/pkg03-0321/research.md",
    ".ai/manifests/pkg03-0321-silent-deployment.v1.json",
    ".ai/plans/pkg03-0321-silent-deployment-v1.md",
    "docs/PKG03-SILENT-DEPLOYMENT-V1.md",
}
VALIDATOR_PATH = "scripts/ci/validate-pkg03-0321.py"
HARNESS_PATH = "scripts/ci/pkg03-0321-silent-deployment.ps1"
WORKFLOW_PATH = ".github/workflows/pkg03-0321-silent-deployment.yml"
PROJECTION_PATHS = {
    "certification/pkg03-windows-installer-v1.json",
    "docs/MASTER-EXECUTION-STATUS.json",
    "README.md",
    ".ai/README.md",
    "docs/MASTER-EXECUTION-PLAN.md",
}
ALLOWED_PATHS = PLANNING_PATHS | {VALIDATOR_PATH, HARNESS_PATH, WORKFLOW_PATH}
ACCEPTED_EVIDENCE = {
    "source_commit": "fc3a947f3f6fe61d2e9e9be0e358634d9bb05cc5",
    "workflow_run": 33791397695,
    "job": 100768588063,
    "artifact": 9908039920,
    "artifact_digest": "sha256:6252741b0f9ae50ef060074fec5066533dcdce7b880681df9a462f6ee98b736c",
    "evidence_sha256": "dcbd63bf5047c363e64950c7cbdd6f3be6cfd722de6b385016e0ffc7fc427fbf",
    "current_user_setup_sha256": "2618973904dc5f888fa23ebcd7ea6cd0e88f5eb2334ce1e22ebadeb27a701d60",
    "per_machine_setup_sha256": "f7b53fb96e0d0bd4fce42f0e51df2ad4f883f588c2b690055ff01846499d4629",
    "msi_sha256": "33bcf088de7cf60761a46bec2dbde150c0206fefce40bc2cccb8350cdcc4b781",
    "product_code": "{FF103894-F7A5-48C9-BA30-2883B692444E}",
    "msi_silent_install_log_sha256": "619864e0edeaef9a6182bc5b4b7f4c6834f23d2fd430ccc2ff3409adbc1914f0",
    "msi_silent_uninstall_log_sha256": "f77f77b4e84fa158768be81fb6d7a8a43441de1c6753f8eb443ba98e559e2a4c",
}


def fail(message: str) -> None:
    raise SystemExit(f"03.21 authority validation failed: {message}")


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


def is_ancestor(ancestor: str, descendant: str = "HEAD") -> bool:
    return subprocess.run(
        ["git", "merge-base", "--is-ancestor", ancestor, descendant],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    ).returncode == 0


def require_ancestor(ancestor: str, descendant: str = "HEAD") -> None:
    if not is_ancestor(ancestor, descendant):
        fail(f"required ancestor missing: {ancestor} is not an ancestor of {descendant}")


def main() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    if manifest.get("task_id") != TASK or manifest.get("linear_issue") != LINEAR:
        fail("manifest task/Linear identity mismatch")
    if manifest.get("status") != "frozen" or manifest.get("canonical_base_sha") != CURRENT_BASE:
        fail("canonical base/frozen status mismatch")
    if manifest.get("dependencies") != ["03.16", "03.17", "03.20"] or manifest.get("lane") != "automation":
        fail("dependency/lane contract mismatch")
    if manifest.get("research", {}).get("change_required") is not False:
        fail("03.21 must remain certification-first before exact failure evidence")

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

    acceptance = manifest.get("acceptance", {})
    for key in (
        "exact_three_package_builds_required",
        "nsis_uppercase_s_required",
        "msi_quiet_required",
        "msi_norestart_required",
        "zero_ui_input_required",
        "no_visible_installer_family_titled_window_required",
        "bounded_completion_required",
        "current_user_service_must_remain_absent",
        "machine_service_contract_required",
        "msi_really_suppress_evidence_required",
        "tracked_repository_drift_zero_required",
    ):
        if acceptance.get(key) is not True:
            fail(f"acceptance flag missing: {key}")
    if acceptance.get("nsis_success_codes") != [0]:
        fail("NSIS success code set drifted")
    if acceptance.get("msi_success_codes") != [0, 3010]:
        fail("MSI success code set drifted")
    if acceptance.get("msi_reboot_initiated_code_forbidden") != 1641:
        fail("MSI reboot-initiated forbidden code drifted")

    authority = manifest.get("authority", {})
    for key, value in authority.items():
        if value is not False:
            fail(f"initial authority widened: {key}={value!r}")

    base_tracker = ref_json(TRACKER_PATH, CURRENT_BASE)
    base_tasks = task_map(base_tracker)
    if (
        base_tracker.get("package_id") != "PKG-03"
        or base_tracker.get("done") != 20
        or base_tracker.get("required") != 25
        or base_tracker.get("percent") != 80.0
        or base_tracker.get("active_task") != TASK
        or base_tracker.get("active_tasks") != []
        or base_tracker.get("ready_tasks") != ["03.21", "03.22"]
    ):
        fail("canonical base is not accepted 20/25 cursor 03.21")
    if base_tasks.get(TASK, {}).get("status") != "READY" or base_tasks.get(TASK, {}).get("depends_on") != ["03.16", "03.17", "03.20"]:
        fail("03.21 canonical READY/dependency contract drifted")
    for dep in ("03.16", "03.17", "03.20"):
        if base_tasks.get(dep, {}).get("status") != "DONE":
            fail(f"canonical dependency {dep} is not DONE")
    if base_tasks.get("03.22", {}).get("status") != "READY":
        fail("independent 03.22 readiness drifted")

    head_tracker = ref_json(TRACKER_PATH, "HEAD")
    head_tasks = task_map(head_tracker)
    head_task = head_tasks.get(TASK, {})
    projection_state = (
        head_tracker.get("package_id") == "PKG-03"
        and head_tracker.get("done") == 21
        and head_tracker.get("required") == 25
        and head_tracker.get("percent") == 84.0
        and head_tracker.get("active_task") == "03.22"
        and head_tracker.get("active_tasks") == []
        and head_tracker.get("ready_tasks") == ["03.22"]
        and head_task.get("status") == "DONE"
    )
    exact_head = git_text("rev-parse", "HEAD")
    strict_projection_mode = projection_state and exact_head == ACCEPTED_PROJECTION_HEAD
    descendant_mode = (
        head_tracker.get("package_id") == "PKG-03"
        and head_tracker.get("required") == 25
        and isinstance(head_tracker.get("done"), int)
        and head_tracker.get("done") >= 21
        and head_task.get("status") == "DONE"
        and not strict_projection_mode
        and is_ancestor(ACCEPTED_PROJECTION_HEAD)
    )

    if strict_projection_mode or descendant_mode:
        evidence = head_task.get("evidence", {})
        for key, expected in ACCEPTED_EVIDENCE.items():
            if evidence.get(key) != expected:
                fail(f"accepted 03.21 evidence mismatch: {key}")
        require_ancestor(ACCEPTED_EVIDENCE["source_commit"])
        require_ancestor(ACCEPTED_PROJECTION_HEAD)
        for dep in ("03.16", "03.17", "03.20"):
            if head_tasks.get(dep, {}).get("status") != "DONE":
                fail(f"accepted descendant dependency {dep} is not DONE")

    if strict_projection_mode:
        if head_tasks.get("03.22", {}).get("status") != "READY":
            fail("03.22 was not preserved READY by accepted 03.21")
        for task_id in ("03.23", "03.24", "03.25"):
            if head_tasks.get(task_id, {}).get("status") != "BLOCKED":
                fail(f"projection prematurely unblocked {task_id}")
    elif not descendant_mode:
        if (
            head_tracker.get("done") != 20
            or head_tracker.get("required") != 25
            or head_tracker.get("percent") != 80.0
            or head_tracker.get("active_task") != TASK
            or head_tracker.get("active_tasks") != []
            or head_tracker.get("ready_tasks") != ["03.21", "03.22"]
            or head_task.get("status") != "READY"
            or head_tasks.get("03.22", {}).get("status") != "READY"
        ):
            fail("HEAD is neither 03.21 implementation state nor accepted projection/descendant")

    changed = [p for p in git_text("diff", "--name-only", f"{CURRENT_BASE}...HEAD").splitlines() if p]
    if not descendant_mode:
        unexpected = []
        for path in changed:
            if path in ALLOWED_PATHS or (strict_projection_mode and path in PROJECTION_PATHS):
                continue
            unexpected.append(path)
        if unexpected:
            fail(f"unauthorized changed paths: {unexpected}")
        if (not strict_projection_mode) and any(path in PROJECTION_PATHS for path in changed):
            fail("canonical projection appeared before accepted evidence")
        if strict_projection_mode and not PROJECTION_PATHS.issubset(set(changed)):
            fail("accepted projection is missing one or more canonical state files")

        product_prefixes = ("apps/", "crates/", "packaging/", "package.json", "Cargo.toml", "Cargo.lock")
        product_changes = [path for path in changed if path.startswith(product_prefixes)]
        if product_changes:
            fail(f"product/runtime/installer mutation appeared in 03.21 scope: {product_changes}")

    plan_text = tracked_bytes(manifest["task_plan"]["path"]).decode("utf-8")
    for token in ("/S", "/quiet", "/norestart", "3010", "1641", "03.19", "03.20"):
        if token not in plan_text:
            fail(f"task plan missing acceptance token: {token}")
    if "/passive" not in plan_text or "not strict silent" not in plan_text:
        fail("passive-mode nonclaim missing")

    harness = tracked_bytes(HARNESS_PATH).decode("utf-8")
    for token in ("/S", "/quiet", "/norestart", "ReallySuppress", "3010", "1641", "visible_titled_windows", "zero_ui_or_input_events_sent"):
        if token not in harness:
            fail(f"harness missing frozen token: {token}")

    workflow = tracked_bytes(WORKFLOW_PATH).decode("utf-8")
    for token in (
        "windows-2025",
        "Validate frozen 03.21 authority",
        "Build exact-head current-user NSIS",
        "Build exact-head per-machine NSIS",
        "Build exact-head MSI/WiX",
        "Exercise exact-head silent deployment",
        "Verify exact 03.21 evidence",
        "pkg03-0321-silent-deployment",
    ):
        if token not in workflow:
            fail(f"workflow missing frozen token: {token}")

    mode = "accepted-descendant" if descendant_mode else ("accepted-projection" if strict_projection_mode else "implementation")
    print(json.dumps({
        "valid": True,
        "task": TASK,
        "linear": LINEAR,
        "canonical_base": CURRENT_BASE,
        "accepted_projection_head": ACCEPTED_PROJECTION_HEAD,
        "mode": mode,
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
