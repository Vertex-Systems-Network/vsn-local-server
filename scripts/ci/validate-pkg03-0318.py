#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

TASK = "03.18"
LINEAR = "ABD-93"
ACTIVATION_BASE = "f3afb66e588d01ff2e8cb37273ad413862a4edaf"
CURRENT_BASE = "8f43f3c09cf749a80d08c623ee8b04f2cfc061ac"
MANIFEST_PATH = Path(".ai/manifests/pkg03-0318-install-rollback.v1.json")
TRACKER_PATH = "certification/pkg03-windows-installer-v1.json"
PLANNING_PATHS = {
    ".ai/features/pkg03-0318/research.md",
    ".ai/features/pkg03-0318/lifecycle-review.md",
    ".ai/features/pkg03-0318/development-preflight.md",
    ".ai/plans/pkg03-0318-install-rollback-v1.md",
    ".ai/manifests/pkg03-0318-install-rollback.v1.json",
    "docs/PKG03-INSTALLER-FAILURE-ROLLBACK-RECOVERY-V1.md",
}
VALIDATOR_PATH = "scripts/ci/validate-pkg03-0318.py"
PROJECTION_PATHS = {
    "certification/pkg03-windows-installer-v1.json",
    "docs/MASTER-EXECUTION-STATUS.json",
    "README.md",
    ".ai/README.md",
    "docs/MASTER-EXECUTION-PLAN.md",
}
ACCEPTED_EVIDENCE = {
    "source_commit": "6a2bef83698e995663541217c74041235f0f5b64",
    "workflow_run": 33535852332,
    "job": 99949856782,
    "artifact": 9812778307,
    "artifact_digest": "sha256:b58d1a2b697ee22a28c5424d4f329b252c3e36f54124a0afe1cd73a1ab3427ac",
    "evidence_sha256": "94444fa288eb52db33c480e98d85e62923e419afeb97904272c4bc6e9a5b3cf2",
}


def fail(message: str) -> None:
    raise SystemExit(f"03.18 authority validation failed: {message}")


def git_text(*args: str) -> str:
    return subprocess.check_output(["git", *args], text=True).strip()


def tracked_sha256(path: str, ref: str = "HEAD") -> str:
    try:
        data = subprocess.check_output(["git", "show", f"{ref}:{path}"])
    except subprocess.CalledProcessError as exc:
        fail(f"cannot read tracked artifact {ref}:{path} ({exc.returncode})")
    return hashlib.sha256(data).hexdigest()


def ref_json(path: str, ref: str) -> dict:
    return json.loads(subprocess.check_output(["git", "show", f"{ref}:{path}"], text=True))


def task_map(tracker: dict) -> dict[str, dict]:
    return {item.get("id"): item for item in tracker.get("tasks", [])}


def main() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    if manifest.get("task_id") != TASK or manifest.get("linear_issue") != LINEAR:
        fail("manifest task/Linear identity mismatch")
    if manifest.get("canonical_base_sha") != ACTIVATION_BASE or manifest.get("status") != "frozen":
        fail("immutable activation/frozen status mismatch")
    if manifest.get("research", {}).get("change_required") is not False:
        fail("planning is not certification-first")

    bindings = [
        (manifest["research"]["artifact"], manifest["research"]["sha256"]),
        (manifest["lifecycle"]["artifact"], manifest["lifecycle"]["sha256"]),
        (manifest["development_preflight"]["artifact"], manifest["development_preflight"]["sha256"]),
        (manifest["task_plan"]["path"], manifest["task_plan"]["sha256"]),
        (manifest["lifecycle_contract"]["artifact"], manifest["lifecycle_contract"]["sha256"]),
        (manifest["parent_plan"]["path"], manifest["parent_plan"]["sha256"]),
    ]
    for path, expected in bindings:
        actual = tracked_sha256(path)
        if actual != expected:
            fail(f"frozen Git-blob digest mismatch for {path}: expected={expected} actual={actual}")

    authority = manifest.get("authority", {})
    for key in (
        "planning_product_mutation_allowed",
        "tauri_config_mutation_allowed",
        "installer_template_or_hook_mutation_allowed",
        "package_identity_mutation_allowed",
        "product_payload_source_mutation_allowed",
        "service_identity_mutation_allowed",
        "acl_mutation_allowed",
        "running_process_coordination_allowed",
        "reboot_semantics_claim_allowed",
        "silent_or_passive_deployment_allowed",
        "signing_secret_access_allowed",
        "updater_mutation_allowed",
        "delegated_scope_may_expand",
    ):
        if authority.get(key) is not False:
            fail(f"authority widened: {key}")

    acceptance = manifest.get("acceptance", {})
    for key in (
        "forced_failure_required",
        "partial_owned_state_forbidden",
        "interrupted_install_required",
        "exact_candidate_rerun_recovery_required",
        "duplicate_identity_forbidden",
        "protected_state_nonmutation_required",
    ):
        if acceptance.get(key) is not True:
            fail(f"required acceptance flag missing: {key}")

    # Preserve the immutable activation witness separately from the live canonical
    # scope baseline. 03.18 was legitimately activated at 15/25; accepted 03.16
    # and 03.17 later advanced main to 17/25 without changing 03.18's frozen
    # dependencies or acceptance. Current diff authorization starts at CURRENT_BASE
    # so already-integrated 03.16/03.17 changes are never attributed to 03.18.
    activation_tracker = ref_json(TRACKER_PATH, ACTIVATION_BASE)
    if activation_tracker.get("done") != 15 or activation_tracker.get("required") != 25:
        fail("activation package baseline is not 15/25")
    activation_tasks = task_map(activation_tracker)
    activation_task = activation_tasks.get(TASK)
    deps = ["03.11", "03.12", "03.14", "03.15"]
    if not activation_task or activation_task.get("status") != "READY" or activation_task.get("depends_on") != deps:
        fail("03.18 activation READY/dependency contract drifted")
    for dep in deps:
        if activation_tasks.get(dep, {}).get("status") != "DONE":
            fail(f"activation dependency {dep} is not DONE")

    current_tracker = ref_json(TRACKER_PATH, CURRENT_BASE)
    if current_tracker.get("done") != 17 or current_tracker.get("required") != 25:
        fail("current canonical package baseline is not 17/25")
    current_tasks = task_map(current_tracker)
    current_task = current_tasks.get(TASK)
    if not current_task or current_task.get("status") != "READY" or current_task.get("depends_on") != deps:
        fail("03.18 current READY/dependency contract drifted")
    for dep in deps:
        if current_tasks.get(dep, {}).get("status") != "DONE":
            fail(f"current dependency {dep} is not canonically DONE")

    head_tracker = ref_json(TRACKER_PATH, "HEAD")
    head_tasks = task_map(head_tracker)
    head_task = head_tasks.get(TASK, {})
    projection_mode = (
        head_tracker.get("package_id") == "PKG-03"
        and head_tracker.get("done") == 18
        and head_tracker.get("required") == 25
        and head_tracker.get("percent") == 72.0
        and head_tracker.get("active_task") == "03.19"
        and head_tracker.get("ready_tasks") == ["03.19", "03.22"]
        and head_task.get("status") == "DONE"
    )
    if projection_mode:
        evidence = head_task.get("evidence", {})
        for key, expected in ACCEPTED_EVIDENCE.items():
            if evidence.get(key) != expected:
                fail(f"accepted projection evidence mismatch: {key}")
        for task_id in ("03.19", "03.22"):
            if head_tasks.get(task_id, {}).get("status") != "READY":
                fail(f"projection READY set drifted at {task_id}")
        for task_id in ("03.20", "03.21", "03.23", "03.24", "03.25"):
            if head_tasks.get(task_id, {}).get("status") != "BLOCKED":
                fail(f"projection prematurely unblocked {task_id}")
    else:
        if head_tracker.get("done") != 17 or head_task.get("status") != "READY":
            fail("HEAD is neither implementation state nor accepted 03.18 projection")

    changed = [p for p in git_text("diff", "--name-only", f"{CURRENT_BASE}...HEAD").splitlines() if p]
    unexpected = []
    for path in changed:
        if (
            path in PLANNING_PATHS
            or path == VALIDATOR_PATH
            or path.startswith("scripts/ci/pkg03-0318-")
            or path.startswith(".github/workflows/pkg03-0318-")
            or (projection_mode and path in PROJECTION_PATHS)
        ):
            continue
        unexpected.append(path)
    if unexpected:
        fail(f"unauthorized changed paths: {unexpected}")
    if (not projection_mode) and any(p in PROJECTION_PATHS for p in changed):
        fail("canonical projection appeared before accepted evidence")
    if projection_mode and not PROJECTION_PATHS.issubset(set(changed)):
        fail("accepted projection is missing one or more canonical state files")
    if any(p.startswith(("apps/", "crates/", "installer/")) for p in changed):
        fail("product/installer mutation appeared before change control")

    print(json.dumps({
        "valid": True,
        "task": TASK,
        "linear": LINEAR,
        "state": head_task.get("status"),
        "mode": "accepted-projection" if projection_mode else "implementation",
        "activation_base": ACTIVATION_BASE,
        "current_base": CURRENT_BASE,
        "activation_done": activation_tracker["done"],
        "current_done": current_tracker["done"],
        "head_progress": {"done": head_tracker.get("done"), "required": head_tracker.get("required"), "percent": head_tracker.get("percent")},
        "dependencies": {dep: current_tasks[dep]["status"] for dep in deps},
        "changed_paths": changed,
        "certification_first": True,
        "product_mutation": False,
    }, indent=2))


if __name__ == "__main__":
    main()
