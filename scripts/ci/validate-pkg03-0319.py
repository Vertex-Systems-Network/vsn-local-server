#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

TASK = "03.19"
LINEAR = "ABD-94"
BASE = "f3afb66e588d01ff2e8cb37273ad413862a4edaf"
MANIFEST_PATH = Path(".ai/manifests/pkg03-0319-running-processes.v1.json")
TRACKER_PATH = "certification/pkg03-windows-installer-v1.json"
PLANNING_PATHS = {
    ".ai/features/pkg03-0319/research.md",
    ".ai/features/pkg03-0319/lifecycle-review.md",
    ".ai/features/pkg03-0319/development-preflight.md",
    ".ai/plans/pkg03-0319-running-processes-v1.md",
    ".ai/manifests/pkg03-0319-running-processes.v1.json",
    "docs/PKG03-INSTALLER-RUNNING-PROCESS-COORDINATION-V1.md",
}
VALIDATOR_PATH = "scripts/ci/validate-pkg03-0319.py"
PROJECTION_PATHS = {
    "certification/pkg03-windows-installer-v1.json",
    "docs/MASTER-EXECUTION-STATUS.json",
    "README.md",
    ".ai/README.md",
    "docs/MASTER-EXECUTION-PLAN.md",
}


def fail(message: str) -> None:
    raise SystemExit(f"03.19 authority validation failed: {message}")


def git_text(*args: str) -> str:
    return subprocess.check_output(["git", *args], text=True).strip()


def tracked_sha256(path: str) -> str:
    try:
        data = subprocess.check_output(["git", "show", f"HEAD:{path}"])
    except subprocess.CalledProcessError as exc:
        fail(f"cannot read tracked artifact HEAD:{path} ({exc.returncode})")
    return hashlib.sha256(data).hexdigest()


def base_json(path: str) -> dict:
    return json.loads(subprocess.check_output(["git", "show", f"{BASE}:{path}"], text=True))


def main() -> None:
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    if manifest.get("task_id") != TASK or manifest.get("linear_issue") != LINEAR:
        fail("manifest task/Linear identity mismatch")
    if manifest.get("canonical_base_sha") != BASE or manifest.get("status") != "frozen":
        fail("canonical base/frozen status mismatch")
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
    mismatches = []
    for path, expected in bindings:
        actual = tracked_sha256(path)
        if actual != expected:
            mismatches.append({"path": path, "expected": expected, "actual": actual})
    if mismatches:
        fail(f"frozen Git-blob digest mismatches: {json.dumps(mismatches, sort_keys=True)}")

    authority = manifest.get("authority", {})
    for key in (
        "planning_product_mutation_allowed",
        "tauri_config_mutation_allowed",
        "installer_template_or_hook_mutation_allowed",
        "product_runtime_mutation_allowed",
        "service_identity_mutation_allowed",
        "acl_mutation_allowed",
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
        "running_resource_identity_required",
        "installer_coordination_or_safe_block_required",
        "silent_force_kill_forbidden",
        "indefinite_hang_forbidden",
        "partial_package_state_forbidden",
        "msi_restart_manager_evidence_required",
        "service_identity_invariant_required",
    ):
        if acceptance.get(key) is not True:
            fail(f"required acceptance flag missing: {key}")
    if manifest.get("locked_inputs", {}).get("harness_pre_kill_allowed") is not False:
        fail("harness pre-kill must remain forbidden")

    tracker = base_json(TRACKER_PATH)
    if tracker.get("done") != 15 or tracker.get("required") != 25:
        fail("canonical package baseline is not 15/25")
    tasks = {row["id"]: row for row in tracker.get("tasks", [])}
    deps = ["03.11", "03.15"]
    task = tasks.get(TASK)
    if not task or task.get("status") != "READY" or task.get("depends_on") != deps:
        fail("03.19 READY/dependency contract drifted")
    for dep in deps:
        if tasks.get(dep, {}).get("status") != "DONE":
            fail(f"dependency {dep} is not canonically DONE")

    changed = [p for p in git_text("diff", "--name-only", f"{BASE}...HEAD").splitlines() if p]
    unexpected = []
    for path in changed:
        if (
            path in PLANNING_PATHS
            or path == VALIDATOR_PATH
            or path.startswith("scripts/ci/pkg03-0319-")
            or path.startswith(".github/workflows/pkg03-0319-")
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
        "canonical_base": BASE,
        "dependencies": {d: tasks[d]["status"] for d in deps},
        "changed_paths": changed,
        "certification_first": True,
        "harness_pre_kill": False,
    }, indent=2))


if __name__ == "__main__":
    main()
