#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / ".ai/manifests/pkg03-0308-msi-enterprise.v1.json"
TRACKER = ROOT / "certification/pkg03-windows-installer-v1.json"
STATUS = ROOT / "docs/MASTER-EXECUTION-STATUS.json"
TAURI = ROOT / "apps/desktop/src-tauri/tauri.conf.json"
MACHINE = ROOT / "apps/desktop/src-tauri/tauri.per-machine.conf.json"
OWNERSHIP = ROOT / "installer/windows/owned-payload.v1.json"
HARNESS = ROOT / "scripts/ci/pkg03-0308-interactive-msi.ps1"
WORKFLOW = ROOT / ".github/workflows/pkg03-0308-msi-enterprise.yml"

EXPECTED_BASE = "0ac71c6392c19ad070a9ec442323c46f3c0e08b9"
EXPECTED_PARENT_SHA = "9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e"
EXPECTED_TAURI_SHA = "172cf6110e58a15442bcf97e9db6a8bdbeb6cbfd2f631d91a3031603ed474180"
EXPECTED_MACHINE_SHA = "48fd4eb22ffe99a884ce5f4770de83e29ad919650d7c254b5d180fca3add7429"
EXPECTED_OWNERSHIP_SHA = "5292a1ec1ae0a48d76f80258c9e00ba17b1466dab5c5e0cdda60caf4658dabd1"
EXPECTED_PRODUCT = "VSN Dev Platform"
EXPECTED_VERSION = "0.38.1"
EXPECTED_PUBLISHER = "Vertex Systems Network"
EXPECTED_UPGRADE_CODE = "157f304f-1d1b-55e0-b89c-0610ea27c645"
EXPECTED_READY_PRE = ["03.08", "03.09", "03.10"]
EXPECTED_READY_POST = ["03.09", "03.10", "03.13", "03.15"]

ALLOWED_CHANGED_PATHS = {
    ".ai/features/pkg03-0308/research.md",
    ".ai/features/pkg03-0308/lifecycle-review.md",
    ".ai/features/pkg03-0308/development-preflight.md",
    ".ai/plans/pkg03-0308-msi-enterprise-v1.md",
    ".ai/manifests/pkg03-0308-msi-enterprise.v1.json",
    "docs/PKG03-MSI-WIX-ENTERPRISE-LIFECYCLE-V1.md",
    "scripts/ci/validate-pkg03-0308.py",
    "scripts/ci/pkg03-0308-interactive-msi.ps1",
    ".github/workflows/pkg03-0308-msi-enterprise.yml",
    "certification/pkg03-windows-installer-v1.json",
    "docs/MASTER-EXECUTION-STATUS.json",
}


def fail(msg: str) -> None:
    raise SystemExit(f"PKG-03 03.08 validation failed: {msg}")


def git_bytes(path: str, ref: str = "HEAD") -> bytes:
    p = subprocess.run(["git", "show", f"{ref}:{path}"], cwd=ROOT,
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if p.returncode:
        fail(f"unable to read {ref}:{path}: {p.stderr.decode(errors='replace')}")
    return p.stdout


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def changed_paths() -> set[str]:
    p = subprocess.run(["git", "diff", "--name-only", f"{EXPECTED_BASE}..HEAD"],
                       cwd=ROOT, text=True, stdout=subprocess.PIPE,
                       stderr=subprocess.PIPE, check=False)
    if p.returncode:
        fail(f"unable to compare canonical base: {p.stderr.strip()}")
    return {x.strip() for x in p.stdout.splitlines() if x.strip()}


def main() -> None:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    tracker = json.loads(TRACKER.read_text(encoding="utf-8"))
    status = json.loads(STATUS.read_text(encoding="utf-8"))
    tauri = json.loads(TAURI.read_text(encoding="utf-8"))
    machine = json.loads(MACHINE.read_text(encoding="utf-8"))
    ownership = json.loads(OWNERSHIP.read_text(encoding="utf-8"))

    if manifest.get("feature_id") != "pkg03-0308-msi-enterprise":
        fail("feature identity mismatch")
    if manifest.get("task_id") != "03.08" or manifest.get("linear_issue") != "ABD-83":
        fail("task identity mismatch")
    if manifest.get("version") != "1.0.0" or manifest.get("status") != "frozen":
        fail("manifest version/status mismatch")
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

    for path, expected_sha in [
        ("apps/desktop/src-tauri/tauri.conf.json", EXPECTED_TAURI_SHA),
        ("apps/desktop/src-tauri/tauri.per-machine.conf.json", EXPECTED_MACHINE_SHA),
        ("installer/windows/owned-payload.v1.json", EXPECTED_OWNERSHIP_SHA),
    ]:
        now = git_bytes(path)
        if now != git_bytes(path, EXPECTED_BASE):
            fail(f"03.08 mutated accepted file: {path}")
        if sha256(now) != expected_sha:
            fail(f"accepted digest drifted: {path}")

    if tauri.get("productName") != EXPECTED_PRODUCT or tauri.get("mainBinaryName") != EXPECTED_PRODUCT:
        fail("product/main binary name drifted")
    if tauri.get("version") != EXPECTED_VERSION or tauri.get("identifier") != "dev.vsn.platform":
        fail("version/identifier drifted")
    bundle = tauri.get("bundle", {})
    win = bundle.get("windows", {})
    wix = win.get("wix", {})
    if bundle.get("publisher") != EXPECTED_PUBLISHER:
        fail("publisher drifted")
    if win.get("allowDowngrades") is not False:
        fail("allowDowngrades drifted")
    if wix.get("upgradeCode", "").lower() != EXPECTED_UPGRADE_CODE:
        fail("WiX UpgradeCode drifted")
    if bundle.get("windows", {}).get("nsis", {}).get("installMode") != "currentUser":
        fail("default NSIS currentUser mode drifted")
    if machine.get("bundle", {}).get("windows", {}).get("nsis", {}).get("installMode") != "perMachine":
        fail("per-machine NSIS overlay drifted")
    if "externalBin" in bundle or "resources" in bundle:
        fail("03.10 placement authority was widened")

    paths = [x.get("relative_path") for x in ownership.get("owned_files", [])]
    if paths != ["VSN Dev Platform.exe", "bin/vsn.exe", "bin/vsn-agent.exe"]:
        fail("owned payload set drifted")
    by_id = {x.get("id"): x for x in ownership.get("owned_files", [])}
    for item_id in ("cli", "agent"):
        item = by_id.get(item_id, {})
        if item.get("placement_owner") != "03.10" or item.get("placement_status") != "declared-not-yet-packaged":
            fail(f"{item_id} placement authority drifted")

    authority = manifest.get("authority", {})
    if authority.get("msi_execution_allowed_after_planning_gates") is not True:
        fail("MSI execution authority missing")
    for key in [
        "planning_product_mutation_allowed", "custom_wix_template_allowed",
        "tauri_config_mutation_allowed", "shortcut_semantics_claim_allowed",
        "cli_agent_real_placement_allowed", "service_registration_allowed",
        "acl_mutation_allowed", "silent_or_passive_deployment_allowed",
        "signing_secret_access_allowed", "updater_mutation_allowed",
        "delegated_scope_may_expand",
    ]:
        if authority.get(key) is not False:
            fail(f"authority widened: {key}")

    acceptance = manifest.get("acceptance", {})
    if acceptance.get("runner") != "windows-2025" or acceptance.get("evidence_artifact") != "pkg03-0308-msi-enterprise":
        fail("acceptance runner/artifact drifted")
    msi = acceptance.get("msi_contract", {})
    if msi.get("install_scope") != "perMachine":
        fail("MSI install scope drifted")
    if msi.get("install_command_shape") != "msiexec /i <exact-msi>":
        fail("MSI install command drifted")
    if msi.get("uninstall_identity") != "exact ProductCode/package":
        fail("MSI uninstall identity drifted")
    if msi.get("visible_ui_required") is not True or msi.get("product_code_runtime_extraction_required") is not True:
        fail("MSI evidence requirements weakened")
    if msi.get("forbidden_ui_suppression") != ["/quiet", "/passive", "/qn", "/qb", "/qr", "/qf"]:
        fail("MSI UI suppression boundary drifted")
    if msi.get("arp_registry_root") != "HKLM" or "{ProductCode}" not in msi.get("arp_key_shape", ""):
        fail("ARP binding drifted")
    if msi.get("blanket_hkcu_nonmutation_claim_allowed") is not False:
        fail("HKCU non-mutation nonclaim drifted")

    if not HARNESS.is_file() or not WORKFLOW.is_file():
        fail("certification surface incomplete")
    harness = HARNESS.read_text(encoding="utf-8")
    workflow = WORKFLOW.read_text(encoding="utf-8")
    for token in [
        "WindowsInstaller.Installer", "ProductCode", "UpgradeCode",
        "msiexec.exe", "'/i'", "'/x'", "UIAutomationClient",
        "ProgramFiles", "HKLM", "VSN Dev Platform.exe",
        "bin/vsn.exe", "bin/vsn-agent.exe", "visible_install_ui_observed",
        "visible_uninstall_ui_observed", "arp_product_code_key_observed",
    ]:
        if token not in harness:
            fail(f"interactive MSI harness missing token: {token}")
    forbidden = ["/quiet", "/passive", "/qn", "/qb", "/qr", "/qf"]
    for token in forbidden:
        for launch in re.findall(r"Start-Process[^\r\n]+", harness, flags=re.IGNORECASE):
            if token.lower() in launch.lower():
                fail(f"forbidden MSI UI suppression used in launch: {token}")
    for token in [
        "windows-2025", "22.12.0", "1.97.1", "build --bundles msi",
        "pkg03-0308-msi-enterprise", EXPECTED_TAURI_SHA,
        EXPECTED_MACHINE_SHA, EXPECTED_OWNERSHIP_SHA,
    ]:
        if token not in workflow:
            fail(f"workflow missing frozen token: {token}")

    tasks = {item["id"]: item for item in tracker.get("tasks", [])}
    expected_ids = [f"03.{i:02d}" for i in range(1, 26)]
    if list(tasks) != expected_ids or tracker.get("required") != 25:
        fail("PKG-03 denominator/order drifted")
    for task_id in [f"03.{i:02d}" for i in range(1, 8)]:
        if tasks.get(task_id, {}).get("status") != "DONE":
            fail(f"completed task regressed: {task_id}")
    if tasks["03.08"].get("depends_on") != ["03.02", "03.03", "03.04", "03.05"]:
        fail("03.08 dependencies drifted")

    state = tasks["03.08"].get("status")
    if state == "READY":
        expected_done, expected_ready, expected_cursor = 7, EXPECTED_READY_PRE, "03.08"
        if tasks["03.08"].get("evidence") is not None:
            fail("pre-evidence task carries evidence")
    elif state == "DONE":
        expected_done, expected_ready, expected_cursor = 8, EXPECTED_READY_POST, "03.09"
        evidence = tasks["03.08"].get("evidence")
        if not isinstance(evidence, dict):
            fail("accepted task missing evidence")
        for key in ["source_commit", "workflow_run", "job", "artifact", "artifact_digest", "evidence_sha256", "msi_sha256", "product_code"]:
            if not evidence.get(key):
                fail(f"accepted evidence missing {key}")
    else:
        fail(f"unexpected 03.08 state: {state}")

    if tracker.get("done") != expected_done or float(tracker.get("percent", -1)) != expected_done * 4.0:
        fail("tracker progress mismatch")
    if tracker.get("active_task") != expected_cursor or tracker.get("ready_tasks") != expected_ready:
        fail("tracker cursor/READY mismatch")

    if state == "READY":
        for task_id in ("03.08", "03.09", "03.10"):
            if tasks[task_id].get("status") != "READY":
                fail(f"pre-evidence READY task regressed: {task_id}")
    else:
        for task_id in EXPECTED_READY_POST:
            if tasks[task_id].get("status") != "READY":
                fail(f"post-evidence READY task missing: {task_id}")

    packages = {item["id"]: item for item in status.get("packages", [])}
    pkg03 = packages.get("PKG-03", {})
    if status.get("active_package") != "PKG-03" or status.get("active_task") != expected_cursor:
        fail("master active package/task mismatch")
    if pkg03.get("done") != expected_done or pkg03.get("required") != 25 or float(pkg03.get("percent", -1)) != expected_done * 4.0:
        fail("master PKG-03 progress mismatch")

    print(json.dumps({
        "valid": True,
        "task": "03.08",
        "state": state,
        "done": expected_done,
        "cursor": expected_cursor,
        "ready": expected_ready,
        "changed_paths": sorted(changed_paths()),
    }, indent=2))


if __name__ == "__main__":
    main()
