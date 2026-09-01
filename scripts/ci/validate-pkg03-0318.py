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
    activation_tasks = {row["id"]: row for row in activation_tracker.get("tasks", [])}
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
    current_tasks = {row["id"]: row for row in current_tracker.get("tasks", [])}
    current_task = current_tasks.get(TASK)
    if not current_task or current_task.get("status") != "READY" or current_task.get("depends_on") != deps:
        fail("03.18 current READY/dependency contract drifted")
    for dep in deps:
        if current_tasks.get(dep, {}).get("status") != "DONE":
            fail(f"current dependency {dep} is not canonically DONE")

    changed = [p for p in git_text("diff", "--name-only", f"{CURRENT_BASE}...HEAD").splitlines() if p]
    unexpected = []
    for path in changed:
        if (
            path in PLANNING_PATHS
            or path == VALIDATOR_PATH
            or path.startswith("scripts/ci/pkg03-0318-")
            or path.startswith(".github/workflows/pkg03-0318-")
        ):
            continue
        unexpected.append(path)
    if unexpected:
        fail(f"unauthorized changed paths: {unexpected}")
    if any(p in PROJECTION_PATHS for p in changed):
        fail("canonical projection appeared before accepted evidence")
    if any(p.startswith(("apps/", "crates/", "installer/")) for p in changed):
        fail("product/installer mutation appeared before change control")

    print(json.dumps({
        "valid": True,
        "task": TASK,
        "linear": LINEAR,
        "activation_base": ACTIVATION_BASE,
        "current_base": CURRENT_BASE,
        "activation_done": activation_tracker["done"],
        "current_done": current_tracker["done"],
        "dependencies": {dep: current_tasks[dep]["status"] for dep in deps},
        "changed_paths": changed,
        "certification_first": True,
        "product_mutation": False,
    }, indent=2))


if __name__ == "__main__":
    main()
