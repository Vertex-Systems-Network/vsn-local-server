#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / ".ai/manifests/pkg03-0307-nsis-machine-install.v1.json"
TRACKER = ROOT / "certification/pkg03-windows-installer-v1.json"
STATUS = ROOT / "docs/MASTER-EXECUTION-STATUS.json"
TAURI = ROOT / "apps/desktop/src-tauri/tauri.conf.json"
MACHINE = ROOT / "apps/desktop/src-tauri/tauri.per-machine.conf.json"
OWNERSHIP = ROOT / "installer/windows/owned-payload.v1.json"
HARNESS = ROOT / "scripts/ci/pkg03-0307-interactive-nsis.ps1"
WORKFLOW = ROOT / ".github/workflows/pkg03-0307-nsis-machine-install.yml"

EXPECTED_BASE = "a5c7781767d9bf5870f66085de7f3c247b943b87"
EXPECTED_PARENT_SHA = "9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e"
EXPECTED_TAURI_SHA = "172cf6110e58a15442bcf97e9db6a8bdbeb6cbfd2f631d91a3031603ed474180"
EXPECTED_MACHINE_SHA = "48fd4eb22ffe99a884ce5f4770de83e29ad919650d7c254b5d180fca3add7429"
EXPECTED_OWNERSHIP_SHA = "5292a1ec1ae0a48d76f80258c9e00ba17b1466dab5c5e0cdda60caf4658dabd1"
EXPECTED_PRODUCT = "VSN Dev Platform"
EXPECTED_VERSION = "0.38.1"
EXPECTED_IDENTIFIER = "dev.vsn.platform"
EXPECTED_PUBLISHER = "Vertex Systems Network"
EXPECTED_OWNED_PATHS = ["VSN Dev Platform.exe", "bin/vsn.exe", "bin/vsn-agent.exe"]
EXPECTED_READY_PRE = ["03.07", "03.08", "03.09", "03.10"]
EXPECTED_READY_POST = ["03.08", "03.09", "03.10"]

ALLOWED_CHANGED_PATHS = {
    ".ai/features/pkg03-0307/research.md",
    ".ai/features/pkg03-0307/lifecycle-review.md",
    ".ai/features/pkg03-0307/development-preflight.md",
    ".ai/plans/pkg03-0307-nsis-machine-install-v1.md",
    ".ai/manifests/pkg03-0307-nsis-machine-install.v1.json",
    "docs/PKG03-NSIS-PER-MACHINE-LIFECYCLE-V1.md",
    "scripts/ci/validate-pkg03-0307.py",
    "scripts/ci/pkg03-0307-interactive-nsis.ps1",
    ".github/workflows/pkg03-0307-nsis-machine-install.yml",
    "certification/pkg03-windows-installer-v1.json",
    "docs/MASTER-EXECUTION-STATUS.json",
}


def fail(message: str) -> None:
    raise SystemExit(f"PKG-03 03.07 validation failed: {message}")


def git_bytes(path: str) -> bytes:
    proc = subprocess.run(
        ["git", "show", f"HEAD:{path}"], cwd=ROOT,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
    )
    if proc.returncode:
        fail(f"unable to read git blob {path}: {proc.stderr.decode(errors='replace')}")
    return proc.stdout


def git_bytes_at(ref: str, path: str) -> bytes:
    proc = subprocess.run(
        ["git", "show", f"{ref}:{path}"], cwd=ROOT,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
    )
    if proc.returncode:
        fail(f"unable to read git blob {ref}:{path}: {proc.stderr.decode(errors='replace')}")
    return proc.stdout


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def changed_paths() -> set[str]:
    proc = subprocess.run(
        ["git", "diff", "--name-only", f"{EXPECTED_BASE}..HEAD"], cwd=ROOT,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=False,
    )
    if proc.returncode:
        fail(f"unable to compare canonical base: {proc.stderr.strip()}")
    return {line.strip() for line in proc.stdout.splitlines() if line.strip()}


def main() -> None:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    tracker = json.loads(TRACKER.read_text(encoding="utf-8"))
    status = json.loads(STATUS.read_text(encoding="utf-8"))
    tauri = json.loads(TAURI.read_text(encoding="utf-8"))
    machine = json.loads(MACHINE.read_text(encoding="utf-8"))
    ownership = json.loads(OWNERSHIP.read_text(encoding="utf-8"))

    if manifest.get("feature_id") != "pkg03-0307-nsis-machine-install":
        fail("feature identity mismatch")
    if manifest.get("task_id") != "03.07" or manifest.get("linear_issue") != "ABD-82":
        fail("task identity mismatch")
    if manifest.get("canonical_base_sha") != EXPECTED_BASE:
        fail("canonical base mismatch")
    if manifest.get("parent_plan", {}).get("sha256") != EXPECTED_PARENT_SHA:
        fail("parent plan digest mismatch")
    if sha256(git_bytes(".ai/plans/pkg03-windows-installer-v1.md")) != EXPECTED_PARENT_SHA:
        fail("parent plan bytes drifted")

    for label, obj in [
        ("research", manifest["research"]),
        ("lifecycle", manifest["lifecycle"]),
        ("development preflight", manifest["development_preflight"]),
        ("task plan", manifest["task_plan"]),
        ("lifecycle contract", manifest["lifecycle_contract"]),
    ]:
        path = obj.get("path") or obj.get("artifact")
        if not path or sha256(git_bytes(path)) != obj.get("sha256"):
            fail(f"{label} digest mismatch")
    if manifest.get("research", {}).get("change_required") is not False:
        fail("material research delta unresolved")

    unexpected = changed_paths() - ALLOWED_CHANGED_PATHS
    if unexpected:
        fail(f"out-of-scope changed paths: {sorted(unexpected)}")

    tauri_bytes = git_bytes("apps/desktop/src-tauri/tauri.conf.json")
    machine_bytes = git_bytes("apps/desktop/src-tauri/tauri.per-machine.conf.json")
    ownership_bytes = git_bytes("installer/windows/owned-payload.v1.json")
    if tauri_bytes != git_bytes_at(EXPECTED_BASE, "apps/desktop/src-tauri/tauri.conf.json"):
        fail("03.07 mutated accepted Tauri config")
    if machine_bytes != git_bytes_at(EXPECTED_BASE, "apps/desktop/src-tauri/tauri.per-machine.conf.json"):
        fail("03.07 mutated accepted per-machine overlay")
    if ownership_bytes != git_bytes_at(EXPECTED_BASE, "installer/windows/owned-payload.v1.json"):
        fail("03.07 mutated accepted owned-payload manifest")
    if sha256(tauri_bytes) != EXPECTED_TAURI_SHA:
        fail("accepted Tauri config digest drifted")
    if sha256(machine_bytes) != EXPECTED_MACHINE_SHA:
        fail("accepted per-machine overlay digest drifted")
    if sha256(ownership_bytes) != EXPECTED_OWNERSHIP_SHA:
        fail("accepted owned-payload digest drifted")

    accepted = manifest.get("accepted_source_sha256", {})
    if accepted != {
        "tauri_config": EXPECTED_TAURI_SHA,
        "per_machine_overlay": EXPECTED_MACHINE_SHA,
        "owned_payload_manifest": EXPECTED_OWNERSHIP_SHA,
    }:
        fail("accepted source digest declaration drifted")

    bundle = tauri.get("bundle", {})
    windows = bundle.get("windows", {})
    if tauri.get("productName") != EXPECTED_PRODUCT or tauri.get("mainBinaryName") != EXPECTED_PRODUCT:
        fail("product/main binary name drifted")
    if tauri.get("version") != EXPECTED_VERSION or tauri.get("identifier") != EXPECTED_IDENTIFIER:
        fail("version/identifier drifted")
    if bundle.get("publisher") != EXPECTED_PUBLISHER or bundle.get("icon") != ["icons/icon.ico"]:
        fail("publisher/icon drifted")
    if windows.get("nsis", {}).get("installMode") != "currentUser":
        fail("default NSIS currentUser mode drifted")
    if machine.get("bundle", {}).get("windows", {}).get("nsis", {}).get("installMode") != "perMachine":
        fail("per-machine overlay drifted")
    if "externalBin" in bundle or "resources" in bundle:
        fail("03.07 must preserve 03.10 CLI/Agent placement authority")

    paths = [item.get("relative_path") for item in ownership.get("owned_files", [])]
    if paths != EXPECTED_OWNED_PATHS:
        fail("owned payload path set drifted")
    by_id = {item.get("id"): item for item in ownership.get("owned_files", [])}
    for item_id in ("cli", "agent"):
        item = by_id.get(item_id, {})
        if item.get("placement_owner") != "03.10" or item.get("placement_status") != "declared-not-yet-packaged":
            fail(f"{item_id} placement authority drifted")

    locked = manifest.get("locked_inputs", {})
    if locked != {
        "node": "22.12.0",
        "rust": "1.97.1",
        "product_version": "0.38.1",
        "default_tauri_nsis_install_mode": "currentUser",
        "per_machine_tauri_nsis_install_mode": "perMachine",
        "main_binary_name": "VSN Dev Platform",
        "windows_icon": "icons/icon.ico",
        "owned_payload_manifest": "installer/windows/owned-payload.v1.json",
    }:
        fail("locked inputs changed")

    authority = manifest.get("authority", {})
    if authority.get("post_planning_allowed_files") != [
        "scripts/ci/validate-pkg03-0307.py",
        "scripts/ci/pkg03-0307-interactive-nsis.ps1",
        ".github/workflows/pkg03-0307-nsis-machine-install.yml",
    ]:
        fail("post-planning allowed file set changed")
    if authority.get("per_machine_elevated_execution_allowed") is not True:
        fail("per-machine execution authority missing")
    for key in [
        "planning_product_mutation_allowed",
        "explicit_runas_allowed",
        "uac_prompt_claim_allowed",
        "custom_nsis_template_allowed",
        "tauri_config_mutation_allowed",
        "msi_execution_allowed",
        "cli_agent_real_placement_allowed",
        "service_registration_allowed",
        "acl_mutation_allowed",
        "signing_secret_access_allowed",
        "updater_mutation_allowed",
        "delegated_scope_may_expand",
    ]:
        if authority.get(key) is not False:
            fail(f"authority widened: {key}")

    acceptance = manifest.get("acceptance", {})
    if acceptance.get("runner") != "windows-2025" or acceptance.get("evidence_artifact") != "pkg03-0307-nsis-machine-install":
        fail("acceptance runner/artifact drifted")
    interactive = acceptance.get("interactive_contract", {})
    if interactive.get("installer_arguments") != [] or interactive.get("uninstaller_arguments") != []:
        fail("installer/uninstaller arguments must remain empty")
    if interactive.get("forbidden_arguments") != ["/S", "/P", "/UPDATE"]:
        fail("forbidden argument contract drifted")
    if interactive.get("explicit_elevation_verb_allowed") is not False:
        fail("explicit elevation verb became allowed")
    for key in [
        "inherited_elevated_runner_token_required",
        "visible_installer_window_required",
        "visible_uninstaller_window_required",
        "uac_disabled_runner_environment_expected",
    ]:
        if interactive.get(key) is not True:
            fail(f"interactive requirement weakened: {key}")
    if interactive.get("uac_prompt_observation_required") is not False or interactive.get("uac_prompt_certified") is not False:
        fail("hosted-runner UAC nonclaim drifted")

    installed = acceptance.get("installed_state", {})
    if installed.get("install_root") != "%ProgramFiles%\\VSN Dev Platform":
        fail("per-machine install root contract drifted")
    if installed.get("registry_scope") != "HKLM" or installed.get("hkcu_registration_forbidden") is not True:
        fail("registry scope contract drifted")
    if installed.get("current_user_install_root_forbidden") is not True:
        fail("current-user root exclusion weakened")
    if installed.get("required_files") != ["VSN Dev Platform.exe", "uninstall.exe"]:
        fail("installed required file set drifted")
    if installed.get("forbidden_until_03_10") != ["bin/vsn.exe", "bin/vsn-agent.exe"]:
        fail("03.10 placement boundary drifted")

    privilege = acceptance.get("process_privilege", {})
    for key in [
        "runner_administrator_required",
        "runner_elevated_required",
        "installer_elevated_required",
        "uninstaller_elevated_required",
        "high_integrity_required",
    ]:
        if privilege.get(key) is not True:
            fail(f"privilege requirement weakened: {key}")

    if HARNESS.exists() or WORKFLOW.exists():
        if not HARNESS.is_file() or not WORKFLOW.is_file():
            fail("partial certification surface")
        harness = HARNESS.read_text(encoding="utf-8")
        workflow = WORKFLOW.read_text(encoding="utf-8")
        required_harness_tokens = [
            "Start-Process -FilePath $SetupPath -PassThru",
            "Start-Process -FilePath $uninstaller -PassThru",
            "UIAutomationClient",
            "Get-ProcessPrivilegeSnapshot",
            "high_integrity",
            "uac_prompt_certified",
            "ProgramFiles",
            "HKLM",
            "HKCU",
            "VSN Dev Platform.exe",
            "bin/vsn.exe",
            "bin/vsn-agent.exe",
        ]
        for token in required_harness_tokens:
            if token not in harness:
                fail(f"interactive harness missing token: {token}")
        for pattern in [
            r"Start-Process\s+-FilePath\s+\$SetupPath[^\r\n]*-ArgumentList",
            r"Start-Process\s+-FilePath\s+\$uninstaller[^\r\n]*-ArgumentList",
            r"Start-Process\s+-FilePath\s+\$SetupPath[^\r\n]*-Verb\s+RunAs",
            r"Start-Process\s+-FilePath\s+\$uninstaller[^\r\n]*-Verb\s+RunAs",
        ]:
            if re.search(pattern, harness, flags=re.IGNORECASE):
                fail(f"forbidden process launch shape: {pattern}")
        for token in [
            "windows-2025",
            "22.12.0",
            "1.97.1",
            "build --bundles nsis --config src-tauri/tauri.per-machine.conf.json",
            "pkg03-0307-nsis-machine-install",
        ]:
            if token not in workflow:
                fail(f"workflow missing frozen token: {token}")
        for digest in [EXPECTED_TAURI_SHA, EXPECTED_MACHINE_SHA, EXPECTED_OWNERSHIP_SHA]:
            if digest not in workflow:
                fail(f"workflow not bound to accepted digest: {digest}")

    tasks = {item["id"]: item for item in tracker.get("tasks", [])}
    expected_ids = [f"03.{i:02d}" for i in range(1, 26)]
    if list(tasks) != expected_ids or tracker.get("required") != 25:
        fail("PKG-03 denominator/order drifted")
    for task_id in ["03.01", "03.02", "03.03", "03.04", "03.05", "03.06"]:
        if tasks.get(task_id, {}).get("status") != "DONE":
            fail(f"canonical completed task regressed: {task_id}")
    if tasks["03.07"].get("depends_on") != ["03.02", "03.03", "03.04", "03.05"]:
        fail("03.07 dependencies drifted")

    state = tasks["03.07"].get("status")
    if state == "READY":
        expected_done = 6
        expected_ready = EXPECTED_READY_PRE
        expected_cursor = "03.07"
        phase = "pre_evidence"
        if tasks["03.07"].get("evidence") is not None:
            fail("pre-evidence task must not carry evidence")
    elif state == "DONE":
        expected_done = 7
        expected_ready = EXPECTED_READY_POST
        expected_cursor = "03.08"
        phase = "accepted"
        evidence = tasks["03.07"].get("evidence")
        if not isinstance(evidence, dict):
            fail("accepted 03.07 missing evidence block")
        for key in ["source_commit", "workflow_run", "job", "artifact", "artifact_digest", "evidence_sha256", "setup_sha256"]:
            if not evidence.get(key):
                fail(f"accepted 03.07 evidence missing {key}")
    else:
        fail(f"unexpected 03.07 status: {state}")

    if tracker.get("done") != expected_done or float(tracker.get("percent", -1)) != expected_done * 4.0:
        fail("tracker progress mismatch")
    if tracker.get("active_task") != expected_cursor or tracker.get("ready_tasks") != expected_ready:
        fail("tracker cursor/READY set mismatch")
    for task_id in EXPECTED_READY_POST:
        if tasks[task_id].get("status") != "READY":
            fail(f"parallel Wave 2 task lost READY state: {task_id}")

    packages = {item["id"]: item for item in status.get("packages", [])}
    pkg03 = packages.get("PKG-03", {})
    if status.get("active_package") != "PKG-03" or status.get("active_task") != expected_cursor:
        fail("master active package/task mismatch")
    if pkg03.get("done") != expected_done or pkg03.get("required") != 25 or float(pkg03.get("percent", -1)) != expected_done * 4.0:
        fail("master PKG-03 progress mismatch")
    if pkg03.get("status") != "IN_PROGRESS":
        fail("master PKG-03 status mismatch")

    print(json.dumps({
        "valid": True,
        "task_id": "03.07",
        "phase": phase,
        "canonical_base": EXPECTED_BASE,
        "done": expected_done,
        "cursor": expected_cursor,
        "ready_tasks": expected_ready,
        "changed_paths": sorted(changed_paths()),
    }, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
