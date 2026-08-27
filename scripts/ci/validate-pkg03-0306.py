#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / ".ai/manifests/pkg03-0306-nsis-user-install.v1.json"
TRACKER = ROOT / "certification/pkg03-windows-installer-v1.json"
STATUS = ROOT / "docs/MASTER-EXECUTION-STATUS.json"
TAURI = ROOT / "apps/desktop/src-tauri/tauri.conf.json"
OWNERSHIP = ROOT / "installer/windows/owned-payload.v1.json"
HARNESS = ROOT / "scripts/ci/pkg03-0306-interactive-nsis.ps1"
WORKFLOW = ROOT / ".github/workflows/pkg03-0306-nsis-user-install.yml"

HISTORICAL_PLANNING_BASE = "bc8d1403e589fa5f4f9833f6975b5cb53e94e01c"
EXPECTED_EXECUTION_BASE = "30d238a6404d656974d2ecc5c13ff2192915565b"
EXPECTED_PARENT_SHA = "9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e"
EXPECTED_TAURI_SHA = "172cf6110e58a15442bcf97e9db6a8bdbeb6cbfd2f631d91a3031603ed474180"
EXPECTED_OWNERSHIP_SHA = "5292a1ec1ae0a48d76f80258c9e00ba17b1466dab5c5e0cdda60caf4658dabd1"
EXPECTED_PRODUCT = "VSN Dev Platform"
EXPECTED_VERSION = "0.38.1"
EXPECTED_IDENTIFIER = "dev.vsn.platform"
EXPECTED_PUBLISHER = "Vertex Systems Network"
EXPECTED_OWNED_PATHS = ["VSN Dev Platform.exe", "bin/vsn.exe", "bin/vsn-agent.exe"]
EXPECTED_READY_PRE = ["03.06", "03.07", "03.08", "03.09", "03.10"]
EXPECTED_READY_POST = ["03.07", "03.08", "03.09", "03.10"]

EXPECTED_REPAIR = {
    "issue": "ABD-113",
    "merge_commit": EXPECTED_EXECUTION_BASE,
    "certification_source": "bd57e4984845f25c27f11ad5c7348ec6962ed8b7",
    "workflow_run": 33016250087,
    "job": 98335111305,
    "artifact": 9624803036,
    "artifact_digest": "sha256:85ea4a7a2afab8ba04b127d178b53ff12b062820a52660950d28620a086a9cd8",
    "evidence_sha256": "918623688a14dabc8cbeb5e7e577e05699e7d7de747bdb892ce1c4f88f82c0ed",
    "counts_as_0306_acceptance": False,
}

ALLOWED_CHANGED_PATHS = {
    ".ai/features/pkg03-0306/research.md",
    ".ai/features/pkg03-0306/lifecycle-review.md",
    ".ai/features/pkg03-0306/development-preflight.md",
    ".ai/plans/pkg03-0306-nsis-user-install-v1.md",
    ".ai/manifests/pkg03-0306-nsis-user-install.v1.json",
    "docs/PKG03-NSIS-CURRENT-USER-LIFECYCLE-V1.md",
    "scripts/ci/validate-pkg03-0306.py",
    "scripts/ci/pkg03-0306-interactive-nsis.ps1",
    ".github/workflows/pkg03-0306-nsis-user-install.yml",
    "certification/pkg03-windows-installer-v1.json",
    "docs/MASTER-EXECUTION-STATUS.json",
}


def fail(message: str) -> None:
    raise SystemExit(f"PKG-03 03.06 validation failed: {message}")


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
        ["git", "diff", "--name-only", f"{EXPECTED_EXECUTION_BASE}..HEAD"], cwd=ROOT,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=False,
    )
    if proc.returncode:
        fail(f"unable to compare repaired execution base: {proc.stderr.strip()}")
    return {line.strip() for line in proc.stdout.splitlines() if line.strip()}


def main() -> None:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    tracker = json.loads(TRACKER.read_text(encoding="utf-8"))
    status = json.loads(STATUS.read_text(encoding="utf-8"))
    tauri = json.loads(TAURI.read_text(encoding="utf-8"))
    ownership = json.loads(OWNERSHIP.read_text(encoding="utf-8"))

    if manifest.get("feature_id") != "pkg03-0306-nsis-user-install":
        fail("feature identity mismatch")
    if manifest.get("task_id") != "03.06" or manifest.get("linear_issue") != "ABD-81":
        fail("task identity mismatch")
    if manifest.get("canonical_base_sha") != HISTORICAL_PLANNING_BASE:
        fail("historical planning base mismatch")
    if manifest.get("execution_base_sha") != EXPECTED_EXECUTION_BASE:
        fail("repaired execution base mismatch")
    if manifest.get("repair_provenance") != EXPECTED_REPAIR:
        fail("ABD-113 repair provenance mismatch")
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
    if tauri_bytes != git_bytes_at(EXPECTED_EXECUTION_BASE, "apps/desktop/src-tauri/tauri.conf.json"):
        fail("03.06 mutated repaired Tauri config")
    if sha256(tauri_bytes) != EXPECTED_TAURI_SHA:
        fail("repaired Tauri config bytes drifted")
    if sha256(git_bytes("installer/windows/owned-payload.v1.json")) != EXPECTED_OWNERSHIP_SHA:
        fail("accepted owned-payload manifest bytes drifted")

    bundle = tauri.get("bundle", {})
    windows = bundle.get("windows", {})
    if tauri.get("productName") != EXPECTED_PRODUCT:
        fail("product name drifted")
    if tauri.get("mainBinaryName") != EXPECTED_PRODUCT:
        fail("repaired main binary name drifted")
    if tauri.get("version") != EXPECTED_VERSION:
        fail("product version drifted")
    if tauri.get("identifier") != EXPECTED_IDENTIFIER:
        fail("identifier drifted")
    if bundle.get("icon") != ["icons/icon.ico"]:
        fail("accepted Windows icon binding drifted")
    if bundle.get("publisher") != EXPECTED_PUBLISHER:
        fail("publisher drifted")
    if windows.get("nsis", {}).get("installMode") != "currentUser":
        fail("NSIS current-user mode drifted")
    if "externalBin" in bundle or "resources" in bundle:
        fail("03.06 must preserve 03.10 CLI/Agent placement authority")

    owned = ownership.get("owned_files", [])
    paths = [item.get("relative_path") for item in owned]
    if paths != EXPECTED_OWNED_PATHS:
        fail("owned payload path set drifted")
    by_id = {item.get("id"): item for item in owned}
    for item_id in ("cli", "agent"):
        item = by_id.get(item_id, {})
        if item.get("placement_owner") != "03.10" or item.get("placement_status") != "declared-not-yet-packaged":
            fail(f"{item_id} placement authority drifted")

    locked = manifest.get("locked_inputs", {})
    if locked != {
        "node": "22.12.0",
        "rust": "1.97.1",
        "product_version": "0.38.1",
        "tauri_nsis_install_mode": "currentUser",
        "main_binary_name": "VSN Dev Platform",
        "windows_icon": "icons/icon.ico",
        "owned_payload_manifest": "installer/windows/owned-payload.v1.json",
    }:
        fail("locked inputs changed")

    authority = manifest.get("authority", {})
    if authority.get("post_planning_allowed_files") != [
        "scripts/ci/validate-pkg03-0306.py",
        "scripts/ci/pkg03-0306-interactive-nsis.ps1",
        ".github/workflows/pkg03-0306-nsis-user-install.yml",
    ]:
        fail("post-planning allowed file set changed")
    for key in [
        "planning_product_mutation_allowed",
        "custom_nsis_template_allowed",
        "tauri_config_mutation_allowed",
        "per_machine_or_elevation_allowed",
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
    if acceptance.get("runner") != "windows-2025":
        fail("runner drifted")
    if acceptance.get("evidence_artifact") != "pkg03-0306-nsis-user-install":
        fail("evidence artifact name drifted")
    interactive = acceptance.get("interactive_contract", {})
    if interactive.get("installer_arguments") != [] or interactive.get("uninstaller_arguments") != []:
        fail("installer/uninstaller arguments must remain empty")
    if interactive.get("forbidden_arguments") != ["/S", "/P", "/UPDATE"]:
        fail("forbidden argument contract drifted")
    if interactive.get("elevation_verb_allowed") is not False:
        fail("elevation verb became allowed")
    if interactive.get("visible_installer_window_required") is not True or interactive.get("visible_uninstaller_window_required") is not True:
        fail("visible GUI evidence requirement weakened")
    installed = acceptance.get("installed_state", {})
    if installed.get("install_root") != "%LOCALAPPDATA%\\VSN Dev Platform":
        fail("install root contract drifted")
    if installed.get("registry_scope") != "HKCU" or installed.get("hklm_registration_forbidden") is not True:
        fail("registry scope contract drifted")
    if installed.get("required_files") != ["VSN Dev Platform.exe", "uninstall.exe"]:
        fail("installed required file set drifted")
    if installed.get("forbidden_until_03_10") != ["bin/vsn.exe", "bin/vsn-agent.exe"]:
        fail("03.10 placement boundary drifted")

    if HARNESS.exists() or WORKFLOW.exists():
        if not HARNESS.is_file() or not WORKFLOW.is_file():
            fail("partial certification surface")
        harness = HARNESS.read_text(encoding="utf-8")
        workflow = WORKFLOW.read_text(encoding="utf-8")
        required_harness_tokens = [
            "Start-Process -FilePath $SetupPath -PassThru",
            "Start-Process -FilePath $uninstaller -PassThru",
            "UIAutomationClient",
            "visible_installer_window_observed",
            "visible_uninstaller_window_observed",
            "HKCU",
            "HKLM",
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
        for token in ["windows-2025", "22.12.0", "1.97.1", "build --bundles nsis", "pkg03-0306-nsis-user-install"]:
            if token not in workflow:
                fail(f"workflow missing frozen token: {token}")
        if EXPECTED_TAURI_SHA not in workflow:
            fail("workflow is not bound to repaired Tauri config digest")

    tasks = {item["id"]: item for item in tracker.get("tasks", [])}
    expected_ids = [f"03.{i:02d}" for i in range(1, 26)]
    if list(tasks) != expected_ids or tracker.get("required") != 25:
        fail("PKG-03 denominator/order drifted")
    for task_id in ["03.02", "03.03", "03.04", "03.05"]:
        if tasks.get(task_id, {}).get("status") != "DONE":
            fail(f"prerequisite not DONE: {task_id}")
    if tasks["03.06"].get("depends_on") != ["03.02", "03.03", "03.04", "03.05"]:
        fail("03.06 dependencies drifted")

    state = tasks["03.06"].get("status")
    if state == "READY":
        expected_done = 5
        expected_ready = EXPECTED_READY_PRE
        expected_cursor = "03.06"
        phase = "pre_evidence"
        if tasks["03.06"].get("evidence") is not None:
            fail("pre-evidence task must not carry evidence")
    elif state == "DONE":
        expected_done = 6
        expected_ready = EXPECTED_READY_POST
        expected_cursor = "03.07"
        phase = "accepted"
        evidence = tasks["03.06"].get("evidence")
        if not isinstance(evidence, dict):
            fail("accepted 03.06 missing evidence block")
        for key in ["source_commit", "workflow_run", "job", "artifact", "artifact_digest", "evidence_sha256", "setup_sha256"]:
            if not evidence.get(key):
                fail(f"accepted 03.06 evidence missing {key}")
    else:
        fail(f"unexpected 03.06 status: {state}")

    if tracker.get("done") != expected_done or float(tracker.get("percent", -1)) != expected_done * 4.0:
        fail("tracker progress mismatch")
    if tracker.get("active_task") != expected_cursor:
        fail("tracker cursor mismatch")
    if tracker.get("ready_tasks") != expected_ready:
        fail("tracker READY set mismatch")
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
        "task_id": "03.06",
        "phase": phase,
        "historical_planning_base": HISTORICAL_PLANNING_BASE,
        "execution_base": EXPECTED_EXECUTION_BASE,
        "repair_issue": "ABD-113",
        "done": expected_done,
        "cursor": expected_cursor,
        "ready_tasks": expected_ready,
        "changed_paths": sorted(changed_paths()),
    }, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
