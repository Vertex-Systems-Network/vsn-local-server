#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
V4_PLANNING_HEAD = "254d62d30e14a8aab4cdd3fcea6050a1126c2310"
CORRECTED_MAIN = "436dd74ab0a0006d49f6a5ff37cf25c478897248"
V4_PLAN_SHA256 = "c765ba84b940f2dc9d23980d5424bc03b754ed12ed0924f6104d8aeaa1a8a017"

WINDOWS_CONFIG = ROOT / "apps/desktop/src-tauri/tauri.windows.conf.json"
NSIS_HOOK = ROOT / "apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh"
WIX_FRAGMENT = ROOT / "apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs"
HARNESS = ROOT / "scripts/ci/pkg03-0311-agent-service-lifecycle.ps1"
WORKFLOW = ROOT / ".github/workflows/pkg03-0311-agent-service-lifecycle.yml"
MANIFEST = ROOT / ".ai/manifests/pkg03-0311-agent-service-install.v4.json"
PLAN = ROOT / ".ai/plans/pkg03-0311-agent-service-install-v4.md"
TRACKER = ROOT / "certification/pkg03-windows-installer-v1.json"
STATUS = ROOT / "docs/MASTER-EXECUTION-STATUS.json"
CHECKPOINT = ROOT / ".ai/current-work.json"
OWNERSHIP = ROOT / "installer/windows/owned-payload.v1.json"

IMPLEMENTATION_PATHS = {
    "apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs",
    "scripts/ci/pkg03-0311-agent-service-lifecycle.ps1",
    "scripts/ci/validate-pkg03-0311.py",
}

FROZEN_V4_PATHS = (
    "apps/agent/src/main.rs",
    "crates/vsn-system/src/lib.rs",
    "installer/windows/owned-payload.v1.json",
    "apps/desktop/src-tauri/tauri.windows.conf.json",
    "apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh",
    ".github/workflows/pkg03-0311-agent-service-lifecycle.yml",
)

EXPECTED_STOP_COMMAND = (
    '[SystemFolder]cmd.exe /D /V:ON /C "'
    '[SystemFolder]sc.exe stop VSN-Agent >nul 2>&1 & set rc=!errorlevel! '
    '& if !rc! EQU 0 exit /b 0 '
    '& if !rc! EQU 1062 exit /b 0 '
    '& exit /b !rc!"'
)


def fail(message: str) -> None:
    raise SystemExit("PKG-03 03.11 V4 validation failed: " + message)


def run(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    proc = subprocess.run(
        args,
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if check and proc.returncode:
        fail(f"command failed ({' '.join(args)}): {proc.stderr.strip()}")
    return proc


def git_bytes(path: str, ref: str = "HEAD") -> bytes:
    proc = subprocess.run(
        ["git", "show", f"{ref}:{path}"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if proc.returncode:
        fail(f"unable to read {ref}:{path}")
    return proc.stdout


def require_tokens(text: str, tokens: tuple[str, ...], label: str) -> None:
    for token in tokens:
        if token not in text:
            fail(f"{label} missing V4 token: {token}")


def local(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


def parse_xml_bytes(data: bytes) -> ET.Element:
    try:
        return ET.fromstring(data)
    except ET.ParseError as exc:
        fail(f"unable to parse WiX XML: {exc}")
        raise


def custom_actions(root: ET.Element) -> dict[str, ET.Element]:
    return {
        el.attrib.get("Id", ""): el
        for el in root.iter()
        if local(el.tag) == "CustomAction" and el.attrib.get("Id")
    }


def sequence_rows(root: ET.Element) -> dict[str, tuple[dict[str, str], str]]:
    return {
        el.attrib.get("Action", ""): (dict(el.attrib), (el.text or "").strip())
        for el in root.iter()
        if local(el.tag) == "Custom" and el.attrib.get("Action")
    }


def main() -> None:
    required_files = (
        WINDOWS_CONFIG, NSIS_HOOK, WIX_FRAGMENT, HARNESS, WORKFLOW,
        MANIFEST, PLAN, TRACKER, STATUS, CHECKPOINT, OWNERSHIP,
    )
    for path in required_files:
        if not path.is_file():
            fail(f"missing required file: {path.relative_to(ROOT)}")

    if run("git", "merge-base", "--is-ancestor", CORRECTED_MAIN, "HEAD", check=False).returncode:
        fail("corrected Governance V3 main is not an ancestor")
    if run("git", "merge-base", "--is-ancestor", V4_PLANNING_HEAD, "HEAD", check=False).returncode:
        fail("exact V4 5/5 planning authorization head is not an ancestor")

    changed = {
        line
        for line in run("git", "diff", "--name-only", f"{V4_PLANNING_HEAD}..HEAD").stdout.splitlines()
        if line
    }
    if changed != IMPLEMENTATION_PATHS:
        fail(
            "post-V4-planning implementation delta must be exactly the approved three paths; "
            f"got {sorted(changed)}"
        )
    added = {
        line
        for line in run(
            "git", "diff", "--diff-filter=A", "--name-only", f"{V4_PLANNING_HEAD}..HEAD"
        ).stdout.splitlines()
        if line
    }
    if added:
        fail(f"V4 implementation must not add files; got {sorted(added)}")

    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    identity = (
        manifest.get("feature_id"),
        manifest.get("version"),
        manifest.get("change_classification", {}).get("classification"),
        manifest.get("gap", {}).get("classification"),
        manifest.get("approval", {}).get("scope"),
        manifest.get("parallel_safety", {}).get("classification"),
    )
    if identity != (
        "pkg03-0311-agent-service-install",
        "4.0.0",
        "CORRECTION",
        "PARTIAL_IMPLEMENTATION",
        "TASK",
        "SERIALIZE",
    ):
        fail(f"active V4 manifest identity/classification drifted: {identity}")
    if manifest.get("canonical_base_sha") != CORRECTED_MAIN:
        fail("V4 manifest canonical base drifted")
    if manifest.get("approval", {}).get("approval_ref") != "conversation:user-2026-08-28-continue-v4-scope-expansion":
        fail("V4 explicit approval reference missing")
    if manifest.get("locked_inputs", {}).get("already_stopped_native_code") != 1062:
        fail("V4 native already-stopped code is not frozen to 1062")
    if manifest.get("locked_inputs", {}).get("service_account") != r"NT AUTHORITY\LocalService":
        fail("accepted LocalService identity drifted")
    if manifest.get("plan", {}).get("path") != ".ai/plans/pkg03-0311-agent-service-install-v4.md":
        fail("V4 manifest does not bind active V4 plan")
    if manifest.get("plan", {}).get("sha256") != V4_PLAN_SHA256:
        fail("V4 manifest plan digest drifted")
    if hashlib.sha256(PLAN.read_bytes()).hexdigest() != V4_PLAN_SHA256:
        fail("active V4 plan bytes do not match frozen digest")

    checkpoint = json.loads(CHECKPOINT.read_text(encoding="utf-8"))
    if checkpoint.get("project", {}).get("canonical_main_at_capture") != CORRECTED_MAIN:
        fail("checkpoint is not bound to corrected canonical main")
    if checkpoint.get("governance_work", {}).get("active_v4_manifest") != ".ai/manifests/pkg03-0311-agent-service-install.v4.json":
        fail("checkpoint does not bind active V4 manifest")
    if checkpoint.get("governance_work", {}).get("active_v4_plan") != ".ai/plans/pkg03-0311-agent-service-install-v4.md":
        fail("checkpoint does not bind active V4 plan")
    approved_paths = set(checkpoint.get("governance_work", {}).get("approved_v4_implementation_paths", []))
    if approved_paths != IMPLEMENTATION_PATHS:
        fail(f"checkpoint V4 approved path set drifted: {sorted(approved_paths)}")

    tracker = json.loads(TRACKER.read_text(encoding="utf-8"))
    tasks = {item["id"]: item for item in tracker.get("tasks", [])}
    if (tracker.get("done"), tracker.get("required"), tracker.get("active_task")) != (10, 25, "03.11"):
        fail("PKG-03 tracker progress/cursor drifted before acceptance")
    if tasks.get("03.11", {}).get("status") != "READY":
        fail("03.11 must remain READY until genuine V4 lifecycle acceptance")
    if tasks.get("03.07", {}).get("status") != "DONE" or tasks.get("03.10", {}).get("status") != "DONE":
        fail("03.11 dependencies are no longer DONE")

    master = json.loads(STATUS.read_text(encoding="utf-8"))
    pkg03 = {p["id"]: p for p in master.get("packages", [])}.get("PKG-03", {})
    if master.get("product_version") != "0.38.1" or master.get("active_task") != "03.11":
        fail("master product/cursor drifted")
    if (pkg03.get("done"), pkg03.get("required"), pkg03.get("percent")) != (10, 25, 40.0):
        fail("master PKG-03 progress drifted")

    for readonly in FROZEN_V4_PATHS:
        if git_bytes(readonly) != git_bytes(readonly, V4_PLANNING_HEAD):
            fail(f"frozen V4 input changed: {readonly}")

    ownership = json.loads(OWNERSHIP.read_text(encoding="utf-8"))
    agent_entries = [
        item for item in ownership.get("owned_files", [])
        if item.get("relative_path") == "bin/vsn-agent.exe"
    ]
    if len(agent_entries) != 1 or agent_entries[0].get("placement_owner") != "03.10":
        fail("bin/vsn-agent.exe no longer has exactly one 03.10 placement owner")

    config = json.loads(WINDOWS_CONFIG.read_text(encoding="utf-8"))
    expected_resources = {
        "../../../target/pkg03/03.10/vsn.exe": "bin/vsn.exe",
        "../../../target/pkg03/03.10/vsn-agent.exe": "bin/vsn-agent.exe",
    }
    bundle = config.get("bundle", {})
    if bundle.get("resources") != expected_resources:
        fail("03.10 payload resource ownership/destinations drifted")
    windows = bundle.get("windows", {})
    if windows.get("nsis", {}).get("installerHooks") != "./windows/pkg03-0311-agent-service.nsh":
        fail("NSIS hook binding drifted")
    wix_cfg = windows.get("wix", {})
    if wix_cfg.get("fragmentPaths") != ["./windows/fragments/pkg03-0311-agent-service.wxs"]:
        fail("WiX fragment binding drifted")
    if wix_cfg.get("featureRefs") != ["Pkg0311AgentServiceLifecycle"]:
        fail("WiX feature linker anchor drifted")

    current_root = ET.parse(WIX_FRAGMENT).getroot()
    baseline_root = parse_xml_bytes(git_bytes(
        "apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs",
        V4_PLANNING_HEAD,
    ))
    if any(local(el.tag) in {"File", "Component", "RegistryKey", "RegistryValue"} for el in current_root.iter()):
        fail("WiX fragment owns forbidden file/component/registry state")

    current_actions = custom_actions(current_root)
    baseline_actions = custom_actions(baseline_root)
    required_ids = {
        "Pkg0311InstallService",
        "Pkg0311StartService",
        "Pkg0311StopService",
        "Pkg0311RemoveService",
    }
    if set(current_actions) != required_ids or set(baseline_actions) != required_ids:
        fail("WiX custom-action set drifted")

    for action_id in ("Pkg0311InstallService", "Pkg0311StartService", "Pkg0311RemoveService"):
        if current_actions[action_id].attrib != baseline_actions[action_id].attrib:
            fail(f"non-stop WiX action drifted: {action_id}")

    stop = current_actions["Pkg0311StopService"].attrib
    baseline_stop = baseline_actions["Pkg0311StopService"].attrib
    for attr in ("Id", "Directory", "Execute", "Impersonate", "Return"):
        if stop.get(attr) != baseline_stop.get(attr):
            fail(f"Pkg0311StopService {attr} drifted from approved privilege/identity contract")
    if (
        stop.get("Directory") != "INSTALLDIR"
        or stop.get("Execute") != "deferred"
        or stop.get("Impersonate") != "no"
        or stop.get("Return") != "check"
    ):
        fail("Pkg0311StopService privilege/return contract is not deferred/no-impersonate/check")
    if stop.get("ExeCommand") != EXPECTED_STOP_COMMAND:
        fail(f"Pkg0311StopService command is not the exact V4 0/1062 wrapper: {stop.get('ExeCommand')}")
    if "Return=\"ignore\"" in WIX_FRAGMENT.read_text(encoding="utf-8"):
        fail("broad Return=ignore suppression is forbidden")

    if sequence_rows(current_root) != sequence_rows(baseline_root):
        fail("WiX InstallExecuteSequence drifted; V4 does not authorize sequencing changes")

    harness = HARNESS.read_text(encoding="utf-8")
    require_tokens(
        harness,
        (
            "function Probe-StoppedServiceNativeCode",
            "$code -eq 1062",
            "expected_already_stopped_code=1062",
            "msi-certification-pre-uninstall",
            "Wait-ServiceState Stopped",
            "native_stopped_service_probe",
            "service_state_before_uninstall='Stopped'",
            "live_running_coordination_owner='03.19'",
            "live_running_uninstall_certified=$false",
            "tracked_repository_drift_zero",
        ),
        "V4 lifecycle harness",
    )
    if "Return=ignore" in harness:
        fail("harness must not describe broad failure suppression as accepted")

    workflow = WORKFLOW.read_text(encoding="utf-8")
    require_tokens(
        workflow,
        (
            "windows-2025",
            "22.12.0",
            "1.97.1",
            "tauri-cli 2.11.4",
            "--bundles nsis",
            "--bundles msi",
            "pkg03-0311-agent-service-lifecycle",
        ),
        "workflow",
    )

    print(json.dumps({
        "valid": True,
        "task": "03.11",
        "v4_planning_head": V4_PLANNING_HEAD,
        "implementation_paths": sorted(changed),
        "changed_file_count": len(changed),
        "new_file_count": len(added),
        "current_user_service_mutation": False,
        "single_agent_payload_owner": "03.10",
        "wix_stop_action": {
            "execute": stop.get("Execute"),
            "impersonate": stop.get("Impersonate"),
            "return": stop.get("Return"),
            "normalized_native_code": 1062,
        },
        "live_running_uninstall_owner": "03.19",
    }, indent=2))


if __name__ == "__main__":
    main()
