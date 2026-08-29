#!/usr/bin/env python3
from __future__ import annotations

import json
import subprocess
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
V4_PLANNING_HEAD = "254d62d30e14a8aab4cdd3fcea6050a1126c2310"
CORRECTED_MAIN = "436dd74ab0a0006d49f6a5ff37cf25c478897248"

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
LIVE_README = ROOT / "README.md"
AI_README = ROOT / ".ai/README.md"
MASTER_PLAN = ROOT / "docs/MASTER-EXECUTION-PLAN.md"

VALIDATOR_PATH = "scripts/ci/validate-pkg03-0311.py"
IMPLEMENTATION_PATHS = {
    "apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs",
    "scripts/ci/pkg03-0311-agent-service-lifecycle.ps1",
    VALIDATOR_PATH,
}
EVIDENCE_BEHAVIOR_PATHS = {
    "apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs",
    "scripts/ci/pkg03-0311-agent-service-lifecycle.ps1",
}
STATE_PROJECTION_PATHS = {
    "certification/pkg03-windows-installer-v1.json",
    "docs/MASTER-EXECUTION-STATUS.json",
}
DESIGNATED_LIVE_PROJECTION_PATHS = {
    "README.md",
    ".ai/README.md",
    "docs/MASTER-EXECUTION-PLAN.md",
}
POST_ACCEPTANCE_PATHS = IMPLEMENTATION_PATHS | STATE_PROJECTION_PATHS | DESIGNATED_LIVE_PROJECTION_PATHS

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

LIVE_MACHINE_MARKER = (
    "Canonical active-package machine state: PKG-03 11/25 IN_PROGRESS; "
    "READY 03.12,03.13,03.14,03.15; deterministic cursor 03.12; query live main SHA at execution time"
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


def is_ancestor(ancestor: str, descendant: str = "HEAD") -> bool:
    return run("git", "merge-base", "--is-ancestor", ancestor, descendant, check=False).returncode == 0


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


def validate_live_projections() -> None:
    readme = LIVE_README.read_text(encoding="utf-8")
    ai_readme = AI_README.read_text(encoding="utf-8")
    master_plan = MASTER_PLAN.read_text(encoding="utf-8")

    for label, text in (
        ("README.md", readme),
        (".ai/README.md", ai_readme),
        ("docs/MASTER-EXECUTION-PLAN.md", master_plan),
    ):
        if LIVE_MACHINE_MARKER not in text:
            fail(f"{label} does not project the accepted 11/25 machine state")

    require_tokens(
        readme,
        (
            "Current genuine PKG-03 progress: `11/25 = 44.00%`.",
            "`03.01`–`03.11` are canonically DONE",
            "Deterministic resume cursor: `03.12`; dependency-ready tasks: `03.12`, `03.13`, `03.14`, `03.15`.",
        ),
        "README accepted-state projection",
    )
    require_tokens(
        master_plan,
        (
            "PKG-03 — Windows Installer** at `11/25 = 44.00%`",
            "Tasks `03.01`–`03.11` are canonically DONE",
            "The deterministic resume cursor is `03.12`. Dependency-ready tasks are `03.12`, `03.13`, `03.14`, and `03.15`",
        ),
        "master-plan accepted-state projection",
    )


def validate_projection_evidence(task: dict, notes: list[str]) -> dict:
    evidence = task.get("evidence")
    if not isinstance(evidence, dict):
        fail("DONE 03.11 is missing evidence")
    required = (
        "source_commit",
        "workflow_run",
        "job",
        "artifact",
        "artifact_digest",
        "evidence_sha256",
        "current_user_setup_sha256",
        "per_machine_setup_sha256",
        "msi_sha256",
        "product_code",
    )
    for key in required:
        if not evidence.get(key):
            fail(f"DONE 03.11 evidence missing {key}")

    source = str(evidence["source_commit"])
    if len(source) != 40 or not is_ancestor(V4_PLANNING_HEAD, source) or not is_ancestor(source, "HEAD"):
        fail("03.11 evidence source is not an accepted ancestor on the V4 lineage")

    for path in EVIDENCE_BEHAVIOR_PATHS:
        if git_bytes(path, "HEAD") != git_bytes(path, source):
            fail(f"post-acceptance product/lifecycle behavior drift after evidence source: {path}")

    if git_bytes(VALIDATOR_PATH, "HEAD") != git_bytes(VALIDATOR_PATH, source):
        validator_delta = {
            line
            for line in run("git", "diff", "--name-only", f"{source}..HEAD").stdout.splitlines()
            if line in IMPLEMENTATION_PATHS
        }
        if validator_delta != {VALIDATOR_PATH}:
            fail(f"post-evidence implementation drift is not validator-only: {sorted(validator_delta)}")

    for key in ("workflow_run", "job", "artifact"):
        if not isinstance(evidence[key], int) or evidence[key] <= 0:
            fail(f"03.11 evidence {key} must be a positive integer")
    if not str(evidence["artifact_digest"]).startswith("sha256:"):
        fail("03.11 artifact digest is not SHA-256 bound")
    if len(str(evidence["evidence_sha256"])) != 64:
        fail("03.11 evidence.json digest is malformed")
    for key in ("current_user_setup_sha256", "per_machine_setup_sha256", "msi_sha256"):
        if len(str(evidence[key])) != 64:
            fail(f"03.11 {key} digest is malformed")

    note_blob = "\n".join(str(item) for item in notes)
    for token in (source, str(evidence["workflow_run"]), str(evidence["artifact"]), str(evidence["evidence_sha256"])):
        if token not in note_blob:
            fail(f"master status 03.11 acceptance note missing evidence token: {token}")
    return evidence


def main() -> None:
    required_files = (
        WINDOWS_CONFIG,
        NSIS_HOOK,
        WIX_FRAGMENT,
        HARNESS,
        WORKFLOW,
        MANIFEST,
        PLAN,
        TRACKER,
        STATUS,
        CHECKPOINT,
        OWNERSHIP,
        LIVE_README,
        AI_README,
        MASTER_PLAN,
    )
    for path in required_files:
        if not path.is_file():
            fail(f"missing required file: {path.relative_to(ROOT)}")

    if not is_ancestor(CORRECTED_MAIN):
        fail("corrected Governance V3 main is not an ancestor")
    if not is_ancestor(V4_PLANNING_HEAD):
        fail("exact V4 5/5 planning authorization head is not an ancestor")

    changed = {
        line
        for line in run("git", "diff", "--name-only", f"{V4_PLANNING_HEAD}..HEAD").stdout.splitlines()
        if line
    }
    if changed == IMPLEMENTATION_PATHS:
        reconciliation_mode = "pre_acceptance"
    elif changed == POST_ACCEPTANCE_PATHS:
        reconciliation_mode = "post_acceptance_projection"
    else:
        fail(
            "post-V4-planning delta must be exactly the approved implementation paths, "
            "or those paths plus the two evidence-bound canonical state files and three designated live projections; "
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
        fail(f"V4 implementation/reconciliation must not add files; got {sorted(added)}")

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
    manifest_plan_sha = manifest.get("plan", {}).get("sha256")
    if not isinstance(manifest_plan_sha, str) or len(manifest_plan_sha) != 64:
        fail("V4 manifest plan digest metadata is malformed")
    plan_path = ".ai/plans/pkg03-0311-agent-service-install-v4.md"
    if git_bytes(plan_path, "HEAD") != git_bytes(plan_path, V4_PLANNING_HEAD):
        fail("active V4 plan Git blob drifted from the 5/5 planning authorization head")
    manifest_path = ".ai/manifests/pkg03-0311-agent-service-install.v4.json"
    if git_bytes(manifest_path, "HEAD") != git_bytes(manifest_path, V4_PLANNING_HEAD):
        fail("active V4 manifest drifted after planning authorization")

    checkpoint = json.loads(CHECKPOINT.read_text(encoding="utf-8"))
    if checkpoint.get("project", {}).get("canonical_main_at_capture") != CORRECTED_MAIN:
        fail("checkpoint is not bound to corrected canonical main")
    if checkpoint.get("governance_work", {}).get("active_v4_manifest") != manifest_path:
        fail("checkpoint does not bind active V4 manifest")
    if checkpoint.get("governance_work", {}).get("active_v4_plan") != plan_path:
        fail("checkpoint does not bind active V4 plan")
    approved_paths = set(checkpoint.get("governance_work", {}).get("approved_v4_implementation_paths", []))
    if approved_paths != IMPLEMENTATION_PATHS:
        fail(f"checkpoint V4 approved path set drifted: {sorted(approved_paths)}")

    tracker = json.loads(TRACKER.read_text(encoding="utf-8"))
    tasks = {item["id"]: item for item in tracker.get("tasks", [])}
    if tasks.get("03.07", {}).get("status") != "DONE" or tasks.get("03.10", {}).get("status") != "DONE":
        fail("03.11 dependencies are no longer DONE")
    if tracker.get("required") != 25:
        fail("PKG-03 tracker denominator drifted")

    master = json.loads(STATUS.read_text(encoding="utf-8"))
    pkg03 = {p["id"]: p for p in master.get("packages", [])}.get("PKG-03", {})
    if master.get("product_version") != "0.38.1" or pkg03.get("required") != 25:
        fail("master product/PKG-03 denominator drifted")

    accepted_evidence = None
    if reconciliation_mode == "pre_acceptance":
        if (tracker.get("done"), tracker.get("active_task"), tracker.get("percent")) != (10, "03.11", 40.0):
            fail("PKG-03 tracker progress/cursor drifted before acceptance")
        if tasks.get("03.11", {}).get("status") != "READY":
            fail("03.11 must remain READY before genuine V4 lifecycle acceptance")
        if tracker.get("ready_tasks") != ["03.11", "03.12", "03.13", "03.14", "03.15"]:
            fail("pre-acceptance READY set drifted")
        if master.get("active_task") != "03.11":
            fail("master cursor drifted before acceptance")
        if (pkg03.get("done"), pkg03.get("percent"), pkg03.get("status")) != (10, 40.0, "IN_PROGRESS"):
            fail("master PKG-03 progress drifted before acceptance")
    else:
        if (tracker.get("done"), tracker.get("active_task"), tracker.get("percent")) != (11, "03.12", 44.0):
            fail("post-acceptance tracker must project 11/25 with cursor 03.12")
        if tasks.get("03.11", {}).get("status") != "DONE":
            fail("post-acceptance projection must mark 03.11 DONE")
        if tracker.get("ready_tasks") != ["03.12", "03.13", "03.14", "03.15"]:
            fail("post-acceptance READY set must be exactly 03.12-03.15")
        for task_id in ("03.12", "03.13", "03.14", "03.15"):
            if tasks.get(task_id, {}).get("status") != "READY":
                fail(f"post-acceptance {task_id} must remain READY")
        for task_id in ("03.16", "03.17", "03.18", "03.19"):
            if tasks.get(task_id, {}).get("status") != "BLOCKED":
                fail(f"post-acceptance {task_id} must remain BLOCKED")
        if master.get("active_task") != "03.12":
            fail("post-acceptance master cursor must be 03.12")
        if (pkg03.get("done"), pkg03.get("percent"), pkg03.get("status")) != (11, 44.0, "IN_PROGRESS"):
            fail("post-acceptance master PKG-03 progress must be 11/25 = 44%")
        accepted_evidence = validate_projection_evidence(tasks["03.11"], master.get("notes", []))
        validate_live_projections()

    for readonly in FROZEN_V4_PATHS:
        if git_bytes(readonly) != git_bytes(readonly, V4_PLANNING_HEAD):
            fail(f"frozen V4 input changed: {readonly}")

    ownership = json.loads(OWNERSHIP.read_text(encoding="utf-8"))
    agent_entries = [
        item
        for item in ownership.get("owned_files", [])
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
    baseline_root = parse_xml_bytes(
        git_bytes(
            "apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs",
            V4_PLANNING_HEAD,
        )
    )
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
    if 'Return="ignore"' in WIX_FRAGMENT.read_text(encoding="utf-8"):
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

    print(
        json.dumps(
            {
                "valid": True,
                "task": "03.11",
                "reconciliation_mode": reconciliation_mode,
                "v4_planning_head": V4_PLANNING_HEAD,
                "changed_paths": sorted(changed),
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
                "accepted_evidence_source": accepted_evidence.get("source_commit") if accepted_evidence else None,
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
