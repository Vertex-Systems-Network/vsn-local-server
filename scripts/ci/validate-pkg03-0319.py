#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

TASK = "03.19"
LINEAR = "ABD-94"
ACTIVATION_BASE = "f3afb66e588d01ff2e8cb37273ad413862a4edaf"
CURRENT_BASE = "9910223a5c5c154c98846c1e091d51ae0acf4847"
ACCEPTED_PROJECTION_HEAD = "73de463594650cb2ebc407957cbb010e8a0e4be8"
MANIFEST_PATH = Path(".ai/manifests/pkg03-0319-running-processes.v1.json")
TRACKER_PATH = "certification/pkg03-windows-installer-v1.json"
CHANGE_CONTROL_PATH = ".ai/features/pkg03-0319/change-control-2026-08-31.md"
CHANGE_CONTROL_BLOB = "89c29fc4473f0883fb84d5eb645d8371270c48ef"
RECONCILIATION_PATH = ".ai/features/pkg03-0319/reconciliation-2026-09-02.md"
RECONCILIATION_BLOB = "971b5b635f12db05d6f8a813df34bfffe8371f62"
HOOK_PATH = "apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh"
HOOK_BLOB = "231bd72cd8afb36ee32b334d0fbbb20484522f9b"
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
ACCEPTED_EVIDENCE = {
    "source_commit": "9e54133afe885bec869093334b866777e0981b9c",
    "workflow_run": 33691830229,
    "job": 100452026075,
    "artifact": 9870937160,
    "artifact_digest": "sha256:b10158927df0b267147c80e265467860c5635d2e0562ed9b79b2dc0ab6710b6f",
    "evidence_sha256": "dc696a4989faa1c123e6ab0061fbca7b33dba0191811f2aa0e6575aea06cced6",
    "current_user_setup_sha256": "f309f6f95eac9c51a226034ac54ce42982f757b81b885fc1c570acea51922898",
    "per_machine_setup_sha256": "30ec17be2312329238315a9620d2cf2378d0a75b198fd2c93d9c4ad4f2d5bbdb",
    "msi_sha256": "cd6c64723d93e1b6d40e1b7c2c0df628430994848a465120886ad0eca3789161",
    "product_code": "{6ECA62D7-D67A-45FC-8D85-6D38A139BC14}",
    "msi_running_uninstall_log_sha256": "aa22f221d520366adc3de9447fe60f685b2c163487f2216a495869a4899545e8",
    "msi_retry_uninstall_log_sha256": "32bd3bf0a25edbb86857875ea481b1d8855d211183e68990f185611ab51ca356",
}


def fail(message: str) -> None:
    raise SystemExit(f"03.19 authority validation failed: {message}")


def git_text(*args: str) -> str:
    return subprocess.check_output(["git", *args], text=True).strip()


def tracked_bytes(path: str, ref: str = "HEAD") -> bytes:
    try:
        return subprocess.check_output(["git", "show", f"{ref}:{path}"])
    except subprocess.CalledProcessError as exc:
        fail(f"cannot read tracked artifact {ref}:{path} ({exc.returncode})")


def tracked_sha256(path: str, ref: str = "HEAD") -> str:
    return hashlib.sha256(tracked_bytes(path, ref)).hexdigest()


def tracked_blob(path: str, ref: str = "HEAD") -> str:
    try:
        return git_text("rev-parse", f"{ref}:{path}")
    except subprocess.CalledProcessError as exc:
        fail(f"cannot resolve tracked blob {ref}:{path} ({exc.returncode})")


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
    if manifest.get("canonical_base_sha") != ACTIVATION_BASE or manifest.get("status") != "frozen":
        fail("immutable activation/frozen status mismatch")
    if manifest.get("research", {}).get("change_required") is not False:
        fail("original planning authority was not certification-first")

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
            fail(f"frozen authority widened: {key}")

    if tracked_blob(CHANGE_CONTROL_PATH) != CHANGE_CONTROL_BLOB:
        fail("03.19 historical change-control artifact blob drifted")
    if tracked_blob(RECONCILIATION_PATH) != RECONCILIATION_BLOB:
        fail("03.19 current-main reconciliation artifact blob drifted")
    if tracked_blob(HOOK_PATH) != HOOK_BLOB:
        fail("03.19 authorized current-main installer-hook blob drifted")

    change_control = tracked_bytes(CHANGE_CONTROL_PATH).decode("utf-8")
    for token in (
        "Triggering Windows run: `33333493252`",
        "Failure artifact: `9738737126`",
        "tauri-bundler 2.9.4",
        HOOK_PATH,
        "Both checks MUST occur before any `VSN-Agent` stop/uninstall command",
        "No custom process kill",
        "service-identity relaxation",
    ):
        if token not in change_control:
            fail(f"change-control artifact missing binding token: {token}")

    reconciliation = tracked_bytes(RECONCILIATION_PATH).decode("utf-8")
    for token in (
        CURRENT_BASE,
        "18/25 = 72%",
        "Superseded historical PR: `#149`",
        "VSN Agent service stop failed with exit code 1",
        "service already stopped: `1062`",
        "service already absent: `1060`",
        "service already marked for deletion: `1072`",
        "03.19 remains `READY`, not `DONE`",
    ):
        if token not in reconciliation:
            fail(f"reconciliation artifact missing binding token: {token}")

    hook = tracked_bytes(HOOK_PATH).decode("utf-8")
    main_guard = '!insertmacro CheckIfAppIsRunning "${MAINBINARYNAME}.exe" "${PRODUCTNAME}"'
    cli_guard = '!insertmacro CheckIfAppIsRunning "vsn.exe" "VSN CLI"'
    stop_marker = 'DetailPrint "Stopping VSN Agent Windows service through SCM"'
    remove_marker = 'DetailPrint "Removing VSN Agent Windows service through SCM"'
    if hook.count(main_guard) != 1 or hook.count(cli_guard) != 1:
        fail("authorized hook must contain exactly one Desktop and one CLI Tauri process guard")
    if hook.index(main_guard) >= hook.index(stop_marker) or hook.index(cli_guard) >= hook.index(stop_marker):
        fail("running-resource guard occurs after Agent service stop")
    if hook.index(main_guard) >= hook.index(remove_marker) or hook.index(cli_guard) >= hook.index(remove_marker):
        fail("running-resource guard occurs after Agent service removal")
    for token in (
        'DetailPrint "Checking VSN Agent Windows service registration"',
        'SetAutoClose true',
        '"$SYSDIR\\sc.exe" stop VSN-Agent',
        'StrCmp $0 "1062" pkg0311_service_stop_ok',
        '"$SYSDIR\\sc.exe" delete VSN-Agent',
        'StrCmp $0 "1060" pkg0311_service_remove_ok',
        'StrCmp $0 "1072" pkg0311_service_remove_ok',
        'DeleteRegValue HKCU "Software\\${MANUFACTURER}\\${PRODUCTNAME}" ""',
    ):
        if token not in hook:
            fail(f"accepted current-main hook semantic missing: {token}")
    for forbidden in (
        '"$INSTDIR\\bin\\vsn-agent.exe" service stop',
        '"$INSTDIR\\bin\\vsn-agent.exe" service uninstall',
        "taskkill",
        "killprocess",
    ):
        if forbidden.lower() in hook.lower():
            fail(f"stale/custom process-service handling reappeared: {forbidden}")

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

    deps = ["03.11", "03.15"]
    activation_tracker = ref_json(TRACKER_PATH, ACTIVATION_BASE)
    if activation_tracker.get("done") != 15 or activation_tracker.get("required") != 25:
        fail("activation package baseline is not 15/25")
    activation_tasks = task_map(activation_tracker)
    activation_task = activation_tasks.get(TASK)
    if not activation_task or activation_task.get("status") != "READY" or activation_task.get("depends_on") != deps:
        fail("03.19 activation READY/dependency contract drifted")
    for dep in deps:
        if activation_tasks.get(dep, {}).get("status") != "DONE":
            fail(f"activation dependency {dep} is not DONE")

    current_tracker = ref_json(TRACKER_PATH, CURRENT_BASE)
    if (
        current_tracker.get("package_id") != "PKG-03"
        or current_tracker.get("done") != 18
        or current_tracker.get("required") != 25
        or current_tracker.get("percent") != 72.0
        or current_tracker.get("active_task") != TASK
        or current_tracker.get("ready_tasks") != ["03.19", "03.22"]
    ):
        fail("current canonical package baseline is not the accepted 18/25 cursor state")
    current_tasks = task_map(current_tracker)
    current_task = current_tasks.get(TASK)
    if not current_task or current_task.get("status") != "READY" or current_task.get("depends_on") != deps:
        fail("03.19 current READY/dependency contract drifted")
    for dep in deps:
        if current_tasks.get(dep, {}).get("status") != "DONE":
            fail(f"current dependency {dep} is not canonically DONE")

    head_tracker = ref_json(TRACKER_PATH, "HEAD")
    head_tasks = task_map(head_tracker)
    head_task = head_tasks.get(TASK, {})
    projection_state = (
        head_tracker.get("package_id") == "PKG-03"
        and head_tracker.get("done") == 19
        and head_tracker.get("required") == 25
        and head_tracker.get("percent") == 76.0
        and head_tracker.get("active_task") == "03.20"
        and head_tracker.get("active_tasks") == []
        and head_tracker.get("ready_tasks") == ["03.20", "03.22"]
        and head_task.get("status") == "DONE"
    )
    exact_head = git_text("rev-parse", "HEAD")
    strict_projection_mode = projection_state and exact_head == ACCEPTED_PROJECTION_HEAD
    descendant_mode = (
        head_tracker.get("package_id") == "PKG-03"
        and head_tracker.get("required") == 25
        and isinstance(head_tracker.get("done"), int)
        and head_tracker.get("done") >= 19
        and head_task.get("status") == "DONE"
        and not strict_projection_mode
        and is_ancestor(ACCEPTED_PROJECTION_HEAD)
    )

    if strict_projection_mode or descendant_mode:
        evidence = head_task.get("evidence", {})
        for key, expected in ACCEPTED_EVIDENCE.items():
            if evidence.get(key) != expected:
                fail(f"accepted 03.19 evidence mismatch: {key}")
        require_ancestor(ACCEPTED_EVIDENCE["source_commit"])
        require_ancestor(ACCEPTED_PROJECTION_HEAD)
        for dep in deps:
            if head_tasks.get(dep, {}).get("status") != "DONE":
                fail(f"accepted descendant dependency {dep} is not DONE")

    if strict_projection_mode:
        if head_tasks.get("03.20", {}).get("status") != "READY":
            fail("03.20 was not unblocked by accepted 03.19")
        if head_tasks.get("03.22", {}).get("status") != "READY":
            fail("03.22 readiness drifted in 03.19 projection")
        for task_id in ("03.21", "03.23", "03.24", "03.25"):
            if head_tasks.get(task_id, {}).get("status") != "BLOCKED":
                fail(f"projection prematurely unblocked {task_id}")
    elif not descendant_mode:
        if (
            head_tracker.get("done") != 18
            or head_tracker.get("required") != 25
            or head_tracker.get("percent") != 72.0
            or head_tracker.get("active_task") != TASK
            or head_tracker.get("ready_tasks") != ["03.19", "03.22"]
            or head_task.get("status") != "READY"
        ):
            fail("HEAD is neither 03.19 implementation state nor accepted projection/descendant")

    changed = [p for p in git_text("diff", "--name-only", f"{CURRENT_BASE}...HEAD").splitlines() if p]
    if not descendant_mode:
        unexpected = []
        for path in changed:
            if (
                path in PLANNING_PATHS
                or path in {VALIDATOR_PATH, CHANGE_CONTROL_PATH, RECONCILIATION_PATH, HOOK_PATH}
                or path.startswith("scripts/ci/pkg03-0319-")
                or path.startswith(".github/workflows/pkg03-0319-")
                or (strict_projection_mode and path in PROJECTION_PATHS)
            ):
                continue
            unexpected.append(path)
        if unexpected:
            fail(f"unauthorized changed paths: {unexpected}")
        if (not strict_projection_mode) and any(p in PROJECTION_PATHS for p in changed):
            fail("canonical projection appeared before accepted evidence")
        if strict_projection_mode and not PROJECTION_PATHS.issubset(set(changed)):
            fail("accepted projection is missing one or more canonical state files")

        product_paths = [p for p in changed if p.startswith(("apps/", "crates/", "installer/"))]
        if product_paths != [HOOK_PATH]:
            fail(f"evidence-bound product exception widened: {product_paths}")

    mode = "accepted-descendant" if descendant_mode else ("accepted-projection" if strict_projection_mode else "implementation")
    print(json.dumps({
        "valid": True,
        "task": TASK,
        "linear": LINEAR,
        "state": head_task.get("status"),
        "mode": mode,
        "activation_base": ACTIVATION_BASE,
        "current_base": CURRENT_BASE,
        "accepted_projection_head": ACCEPTED_PROJECTION_HEAD,
        "activation_done": activation_tracker["done"],
        "current_done": current_tracker["done"],
        "head_progress": {"done": head_tracker.get("done"), "required": head_tracker.get("required"), "percent": head_tracker.get("percent")},
        "dependencies": {d: head_tasks[d]["status"] for d in deps},
        "changed_paths": changed,
        "certification_first_original_plan": True,
        "evidence_bound_change_control": True,
        "current_main_reconciled": True,
        "authorized_installer_hook": HOOK_PATH,
        "preserved_scm_idempotence": [1062, 1060, 1072],
        "harness_pre_kill": False,
    }, indent=2))


if __name__ == "__main__":
    main()
