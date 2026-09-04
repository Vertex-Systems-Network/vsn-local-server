#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
LIVE_BASE = "f3afb66e588d01ff2e8cb37273ad413862a4edaf"
TASK = "03.16"
MANIFEST_PATH = ROOT / ".ai/manifests/pkg03-0316-reinstall-repair.v1.json"
TRACKER_PATH = ROOT / "certification/pkg03-windows-installer-v1.json"
DIAGNOSTIC_PATH = ROOT / "dist-pkg03/03.16/authority-validation.log"
CHANGE_CONTROL_DOC = ".ai/features/pkg03-0316/change-control-001.md"
CHANGE_CONTROL_PRODUCT_PATH = "apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh"

PLANNING = {
    "research": ".ai/features/pkg03-0316/research.md",
    "lifecycle": ".ai/features/pkg03-0316/lifecycle-review.md",
    "development_preflight": ".ai/features/pkg03-0316/development-preflight.md",
    "task_plan": ".ai/plans/pkg03-0316-reinstall-repair-v1.md",
    "lifecycle_contract": "docs/PKG03-INSTALLER-REINSTALL-REPAIR-LIFECYCLE-V1.md",
}
IMPLEMENTATION = {
    "scripts/ci/validate-pkg03-0316.py",
    "scripts/ci/pkg03-0316-reinstall-repair.ps1",
    ".github/workflows/pkg03-0316-reinstall-repair.yml",
}
STATE = {
    "certification/pkg03-windows-installer-v1.json",
    "docs/MASTER-EXECUTION-STATUS.json",
    "README.md",
    ".ai/README.md",
    "docs/MASTER-EXECUTION-PLAN.md",
}
CHANGE_CONTROL = {CHANGE_CONTROL_DOC, CHANGE_CONTROL_PRODUCT_PATH}
ALLOWED = set(PLANNING.values()) | {MANIFEST_PATH.relative_to(ROOT).as_posix()} | IMPLEMENTATION | STATE | CHANGE_CONTROL
PROTECTED_PRODUCT_INPUTS = {
    "apps/desktop/src-tauri/tauri.conf.json",
    "apps/desktop/src-tauri/tauri.windows.conf.json",
    "apps/desktop/src-tauri/tauri.per-machine.conf.json",
    "apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh",
    "apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs",
    "installer/windows/owned-payload.v1.json",
    "scripts/ci/pkg03-0310-stage-windows-payload.ps1",
    "crates/vsn-security/src/lib.rs",
    "apps/desktop/package.json",
    "apps/desktop/package-lock.json",
    "Cargo.lock",
}
ACCEPTED_EVIDENCE = {
    "source_commit": "76e74d27a5364aca95b22391a43dc043afb70ef0",
    "workflow_run": 33438918354,
    "job": 99642061436,
    "artifact": 9776080488,
    "artifact_digest": "sha256:42e06e1fe8a505b9ca86b48c3ba46a64646e9b96ad5370a5cc43ce8c0be3e24e",
    "evidence_sha256": "1ec1d4757ece8f23644df5e99f25f628837895d08bd398afb98b5d10d54ed4cc",
    "current_user_setup_sha256": "34b3385f569d85333e76927f3a2e5c7a24585c7105a8a83603f9ce403423d561",
    "per_machine_setup_sha256": "3c91a9f6d34cf668a98746c43b7c43abba2c7f94e6787767074ca48880306e2b",
    "msi_sha256": "2c71e1c9bb0f6bb4c6105d79b83dedc2016ee0bcc6b2f44ca7032a7ba0f781b7",
    "product_code": "{58DD5B3A-6071-45DA-ABFA-B954B4CE43FF}",
    "wix_initial_log_sha256": "382327a32db4f62be7899c9143cc497aa7587404a9407f47feed25634257b531",
    "wix_reinstall_healthy_1_log_sha256": "bb09bf12fe362ab92c9d208de7d75e16646c8a4be3aebedbefa1834d2def8031",
    "wix_repair_missing_log_sha256": "2be962f21f1074953565c2bbb90f8b1f57212ae854f5b4332163b97d17c36aef",
    "wix_repair_tamper_log_sha256": "b96963fca73b78595d0b5ecc071797a124010b7eddff1dc18b6d7095566a16d4",
    "wix_reinstall_healthy_2_log_sha256": "877fceaf8bc85e3d222f96d22fefdd8a4ebb6656d38eef0f07c35c706720659c",
}


def fail(message: str) -> None:
    rendered = "PKG-03 03.16 validation failed: " + message
    DIAGNOSTIC_PATH.parent.mkdir(parents=True, exist_ok=True)
    DIAGNOSTIC_PATH.write_text(rendered + "\n", encoding="utf-8")
    raise SystemExit(rendered)


def sha256_tracked(relative: str) -> str:
    """Hash canonical Git blob bytes so Windows checkout EOL conversion cannot fake drift."""
    try:
        blob = subprocess.check_output(["git", "show", f"HEAD:{relative}"], cwd=ROOT)
    except subprocess.CalledProcessError as exc:
        fail(f"cannot read tracked planning artifact from HEAD: {relative} ({exc.returncode})")
    return hashlib.sha256(blob).hexdigest()


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def changed_paths() -> list[str]:
    return [line for line in git("diff", "--name-only", f"{LIVE_BASE}...HEAD").splitlines() if line]


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
        manifest.get("feature_id"), manifest.get("task_id"),
        manifest.get("linear_issue"), manifest.get("version"), manifest.get("status"),
    )
    if identity != ("pkg03-0316-reinstall-repair", TASK, "ABD-91", "1.0.0", "frozen"):
        fail("manifest identity/version/status mismatch")
    if manifest.get("canonical_base_sha") != LIVE_BASE:
        fail("canonical activation base mismatch")
    if manifest.get("parent_plan", {}).get("sha256") != "9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e":
        fail("parent plan digest declaration drifted")
    if manifest.get("research", {}).get("change_required") is not False:
        fail("frozen planning research must remain certification-first; evidence-triggered mutation belongs to bounded change control")

    change = manifest.get("change_control", {})
    if change.get("id") != "CC-0316-001" or change.get("status") != "active":
        fail("bounded change-control identity/status missing")
    if change.get("trigger_run_id") != 33281884610 or change.get("trigger_job_id") != 99178308962:
        fail("bounded change-control trigger evidence drifted")
    if change.get("allowed_product_paths") != [CHANGE_CONTROL_PRODUCT_PATH]:
        fail("bounded change-control product path set widened or drifted")
    if change.get("artifact") != CHANGE_CONTROL_DOC or not (ROOT / CHANGE_CONTROL_DOC).is_file():
        fail("bounded change-control artifact missing")

    require_ancestor(LIVE_BASE)

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

    for relative in IMPLEMENTATION:
        if not (ROOT / relative).is_file():
            fail(f"implementation artifact missing: {relative}")

    tasks = {task["id"]: task for task in tracker.get("tasks", [])}
    for dependency in ("03.11", "03.12", "03.14", "03.15"):
        if tasks.get(dependency, {}).get("status") != "DONE":
            fail(f"dependency {dependency} is not canonically DONE")
    task = tasks.get(TASK, {})
    state = task.get("status")
    if state not in {"READY", "DONE"}:
        fail(f"tracker state is not READY/DONE: {state}")

    descendant_mode = state == "DONE" and isinstance(tracker.get("done"), int) and tracker.get("done") >= 16
    if descendant_mode:
        evidence = task.get("evidence")
        if not isinstance(evidence, dict):
            fail("accepted descendant is missing frozen 03.16 evidence")
        for key, expected in ACCEPTED_EVIDENCE.items():
            if evidence.get(key) != expected:
                fail(f"accepted 03.16 evidence drifted: {key}")
        require_ancestor(ACCEPTED_EVIDENCE["source_commit"])

    paths = changed_paths()
    protected = set(paths) & PROTECTED_PRODUCT_INPUTS
    if not descendant_mode:
        unexpected = sorted(set(paths) - ALLOWED)
        if unexpected:
            fail(f"03.16 branch changed unauthorized paths: {unexpected}")
        unauthorized_protected = sorted(protected - {CHANGE_CONTROL_PRODUCT_PATH})
        if unauthorized_protected:
            fail(f"03.16 illegally changed accepted product inputs: {unauthorized_protected}")

    locked = manifest.get("locked_inputs", {})
    expected_locked = {
        "node": "22.12.0",
        "rust": "1.97.1",
        "product_version": "0.38.1",
        "tauri_cli": "2.11.4",
        "product_name": "VSN Dev Platform",
        "agent_service_name": "VSN-Agent",
        "agent_service_account": "NT AUTHORITY\\LocalService",
        "integrity_states": ["MATCH", "MISSING", "HASH_MISMATCH"],
        "msi_repair_mode": "force-file-reinstall",
        "running_processes_required": False,
    }
    for key, value in expected_locked.items():
        if locked.get(key) != value:
            fail(f"locked input drifted: {key}")
    if locked.get("dependency_tasks") != ["03.11", "03.12", "03.14", "03.15"]:
        fail("dependency task declaration drifted")

    acceptance = manifest.get("acceptance", {})
    for key in (
        "healthy_idempotent_reinstall_required",
        "missing_file_exact_repair_required",
        "tampered_file_exact_repair_required",
        "second_healthy_pass_required",
        "duplicate_registration_forbidden",
        "current_user_machine_service_forbidden",
        "service_quiescent_during_destructive_probe",
        "exact_sha256_restoration_required",
        "msi_verbose_repair_logs_required",
    ):
        if acceptance.get(key) is not True:
            fail(f"acceptance invariant disabled: {key}")
    if acceptance.get("per_machine_agent_destructive_probe_allowed") is not False:
        fail("machine Agent destructive probe must remain forbidden")

    authority = manifest.get("authority", {})
    for key in (
        "planning_product_mutation_allowed",
        "tauri_config_mutation_allowed",
        "installer_template_or_hook_mutation_allowed",
        "package_identity_mutation_allowed",
        "product_payload_source_mutation_allowed",
        "new_repair_runtime_allowed",
        "service_identity_mutation_allowed",
        "acl_mutation_allowed",
        "firewall_hosts_dns_trust_mutation_allowed",
        "running_process_coordination_allowed",
        "rollback_or_recovery_claim_allowed",
        "dirty_user_data_uninstall_claim_allowed",
        "reboot_semantics_claim_allowed",
        "silent_or_passive_deployment_allowed",
        "signing_secret_access_allowed",
        "updater_mutation_allowed",
        "delegated_scope_may_expand",
    ):
        if authority.get(key) is not False:
            fail(f"global authority widened: {key}")

    hook = (ROOT / CHANGE_CONTROL_PRODUCT_PATH).read_text(encoding="utf-8")
    for token in (
        'sc.exe" query VSN-Agent',
        'vsn-agent.exe" service install',
        'vsn-agent.exe" service start',
        'StrCmp $0 "0" pkg0311_service_install_ok',
    ):
        if token not in hook:
            fail(f"bounded service-hook remediation missing token: {token}")

    harness = (ROOT / "scripts/ci/pkg03-0316-reinstall-repair.ps1").read_text(encoding="utf-8")
    workflow = (ROOT / ".github/workflows/pkg03-0316-reinstall-repair.yml").read_text(encoding="utf-8")
    for token in (
        "MISSING", "HASH_MISMATCH", "MATCH", "VSN-Agent", "Stop-Service",
        "nsis-current-user", "nsis-per-machine", "wix-per-machine", "/fa",
        "reinstall-healthy-1", "repair-missing", "repair-tamper", "reinstall-healthy-2",
        "exact_sha256_restored", "duplicate_registration_forbidden",
    ):
        if token not in harness:
            fail(f"repair harness missing frozen token: {token}")
    for forbidden in ("/quiet", "/passive", "/qn", "/qb"):
        if forbidden.lower() in harness.lower():
            fail(f"repair harness contains forbidden deployment mode: {forbidden}")
    for token in (
        "runs-on: windows-2025", "22.12.0", "1.97.1", "tauri-cli 2.11.4",
        "tauri.per-machine.conf.json", "--bundles nsis", "--bundles msi",
        "pkg03-0316-reinstall-repair.ps1", "pkg03-0316-reinstall-repair",
    ):
        if token not in workflow:
            fail(f"workflow missing frozen token: {token}")

    print(json.dumps({
        "valid": True,
        "task": TASK,
        "state": state,
        "mode": "accepted-descendant" if descendant_mode else "implementation",
        "live_execution_base": LIVE_BASE,
        "dependencies": {key: tasks[key]["status"] for key in ("03.11", "03.12", "03.14", "03.15")},
        "branch_changed_paths": paths,
        "product_inputs_unchanged": len(protected) == 0,
        "certification_first": True,
        "bounded_change_control": {
            "id": change["id"],
            "allowed_product_paths": change["allowed_product_paths"],
            "trigger_run_id": change["trigger_run_id"],
            "trigger_job_id": change["trigger_job_id"],
        },
    }, indent=2))


if __name__ == "__main__":
    main()
