#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PAUSE_LIFT_HEAD = "5ac6ebf9e2927edc260a8e32e4ddd2589c7b301d"
CORRECTED_MAIN = "436dd74ab0a0006d49f6a5ff37cf25c478897248"
OWNERSHIP_SHA = "5292a1ec1ae0a48d76f80258c9e00ba17b1466dab5c5e0cdda60caf4658dabd1"

WINDOWS_CONFIG = ROOT / "apps/desktop/src-tauri/tauri.windows.conf.json"
NSIS_HOOK = ROOT / "apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh"
WIX_FRAGMENT = ROOT / "apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs"
HARNESS = ROOT / "scripts/ci/pkg03-0311-agent-service-lifecycle.ps1"
WORKFLOW = ROOT / ".github/workflows/pkg03-0311-agent-service-lifecycle.yml"
MANIFEST = ROOT / ".ai/manifests/pkg03-0311-agent-service-install.v2.json"
TRACKER = ROOT / "certification/pkg03-windows-installer-v1.json"
STATUS = ROOT / "docs/MASTER-EXECUTION-STATUS.json"
CHECKPOINT = ROOT / ".ai/current-work.json"
OWNERSHIP = ROOT / "installer/windows/owned-payload.v1.json"

IMPLEMENTATION_PATHS = {
    "apps/desktop/src-tauri/tauri.windows.conf.json",
    "apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh",
    "apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs",
    "scripts/ci/pkg03-0311-agent-service-lifecycle.ps1",
    "scripts/ci/validate-pkg03-0311.py",
    ".github/workflows/pkg03-0311-agent-service-lifecycle.yml",
}
MANIFEST_CORRECTION_PATH = ".ai/manifests/pkg03-0311-agent-service-install.v2.json"
EXPECTED_DELTA_PATHS = IMPLEMENTATION_PATHS | {MANIFEST_CORRECTION_PATH}


def fail(message: str) -> None:
    raise SystemExit("PKG-03 03.11 validation failed: " + message)


def run(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    proc = subprocess.run(args, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if check and proc.returncode:
        fail(f"command failed ({' '.join(args)}): {proc.stderr.strip()}")
    return proc


def git_bytes(path: str, ref: str = "HEAD") -> bytes:
    proc = subprocess.run(["git", "show", f"{ref}:{path}"], cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if proc.returncode:
        fail(f"unable to read {ref}:{path}")
    return proc.stdout


def main() -> None:
    for path in (WINDOWS_CONFIG, NSIS_HOOK, WIX_FRAGMENT, HARNESS, WORKFLOW, MANIFEST, TRACKER, STATUS, CHECKPOINT, OWNERSHIP):
        if not path.is_file():
            fail(f"missing required file: {path.relative_to(ROOT)}")

    if run("git", "merge-base", "--is-ancestor", CORRECTED_MAIN, "HEAD", check=False).returncode != 0:
        fail("corrected Governance V3 main is not an ancestor of implementation head")
    if run("git", "merge-base", "--is-ancestor", PAUSE_LIFT_HEAD, "HEAD", check=False).returncode != 0:
        fail("exact pause-lift gate head is not an ancestor of implementation head")

    changed = {line for line in run("git", "diff", "--name-only", f"{PAUSE_LIFT_HEAD}..HEAD").stdout.splitlines() if line}
    if changed != EXPECTED_DELTA_PATHS:
        fail(
            "implementation/correction delta must be exactly the frozen six implementation paths "
            f"plus bounded manifest correction; got {sorted(changed)}"
        )

    added = {
        line for line in run(
            "git", "diff", "--diff-filter=A", "--name-only", f"{PAUSE_LIFT_HEAD}..HEAD"
        ).stdout.splitlines() if line
    }

    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    if (manifest.get("feature_id"), manifest.get("version"), manifest.get("change_classification", {}).get("classification")) != (
        "pkg03-0311-agent-service-install", "2.0.0", "COMPLETION"
    ):
        fail("manifest identity/classification drifted")
    if manifest.get("gap", {}).get("classification") != "MISSING_IMPLEMENTATION":
        fail("03.11 gap classification drifted")
    if manifest.get("approval", {}).get("scope") != "TASK":
        fail("03.11 approval scope drifted")
    if manifest.get("parallel_safety", {}).get("classification") != "COORDINATED_PARALLEL":
        fail("03.11 parallel classification drifted")

    budget = manifest.get("preflight", {}).get("scope_budget", {})
    if len(changed) > int(budget.get("max_changed_files", 0)):
        fail(f"changed-file budget exceeded: {len(changed)} > {budget.get('max_changed_files')}")
    if len(added) > int(budget.get("max_new_files", 0)):
        fail(f"new-file budget exceeded: {len(added)} > {budget.get('max_new_files')}")
    if manifest.get("locked_inputs", {}).get("service_account") != r"NT AUTHORITY\LocalService":
        fail("manifest service account is not the accepted LocalService identity")
    fast_commands = manifest.get("quality_gates", {}).get("fast_gate", {}).get("commands", [])
    if not fast_commands or fast_commands[0] != "python scripts/ci/validate-pkg03-0311.py --static":
        fail("manifest fast-gate validator path is invalid")
    acceptance_commands = manifest.get("acceptance", {}).get("commands", [])
    if not acceptance_commands or acceptance_commands[0] != "python scripts/ci/validate-pkg03-0311.py":
        fail("manifest acceptance validator path is invalid")

    checkpoint = json.loads(CHECKPOINT.read_text(encoding="utf-8"))
    if checkpoint.get("pause", {}).get("product_development_paused") is not False:
        fail("03.11 implementation head is not descended from explicit unpaused checkpoint")
    if checkpoint.get("project", {}).get("canonical_main_at_capture") != CORRECTED_MAIN:
        fail("checkpoint is not bound to corrected canonical main")

    tracker = json.loads(TRACKER.read_text(encoding="utf-8"))
    tasks = {item["id"]: item for item in tracker.get("tasks", [])}
    if tracker.get("required") != 25 or tracker.get("done") != 10 or tracker.get("active_task") != "03.11":
        fail("PKG-03 canonical progress/cursor drifted before acceptance")
    if tasks.get("03.11", {}).get("status") != "READY":
        fail("03.11 must remain READY until genuine lifecycle acceptance")
    if tasks.get("03.07", {}).get("status") != "DONE" or tasks.get("03.10", {}).get("status") != "DONE":
        fail("03.11 dependencies are no longer DONE")

    master = json.loads(STATUS.read_text(encoding="utf-8"))
    pkg03 = {p["id"]: p for p in master.get("packages", [])}.get("PKG-03", {})
    if master.get("product_version") != "0.38.1" or master.get("active_task") != "03.11":
        fail("master product/cursor drifted")
    if (pkg03.get("done"), pkg03.get("required"), pkg03.get("percent")) != (10, 25, 40.0):
        fail("master PKG-03 progress drifted")

    ownership_bytes = git_bytes("installer/windows/owned-payload.v1.json")
    if hashlib.sha256(ownership_bytes).hexdigest() != OWNERSHIP_SHA:
        fail("accepted ownership manifest drifted")
    ownership = json.loads(ownership_bytes.decode("utf-8"))
    agent_entries = [x for x in ownership.get("owned_files", []) if x.get("relative_path") == "bin/vsn-agent.exe"]
    if len(agent_entries) != 1 or agent_entries[0].get("placement_owner") != "03.10":
        fail("bin/vsn-agent.exe no longer has exactly one 03.10 owner")

    for readonly in ("apps/agent/src/main.rs", "crates/vsn-system/src/lib.rs", "installer/windows/owned-payload.v1.json"):
        if git_bytes(readonly) != git_bytes(readonly, PAUSE_LIFT_HEAD):
            fail(f"forbidden read-only product input changed: {readonly}")

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
        fail("NSIS installerHooks binding missing")
    wix = windows.get("wix", {})
    if wix.get("fragmentPaths") != ["./windows/fragments/pkg03-0311-agent-service.wxs"]:
        fail("WiX fragmentPaths binding mismatch")
    if wix.get("featureRefs") != ["Pkg0311AgentServiceLifecycle"]:
        fail("WiX feature linker anchor mismatch")

    nsis = NSIS_HOOK.read_text(encoding="utf-8")
    for token in (
        '!if "${INSTALLMODE}" == "perMachine"',
        '"$INSTDIR\\bin\\vsn-agent.exe" service install',
        '"$INSTDIR\\bin\\vsn-agent.exe" service start',
        '"$INSTDIR\\bin\\vsn-agent.exe" service stop',
        '"$INSTDIR\\bin\\vsn-agent.exe" service uninstall',
        "NSIS_HOOK_POSTINSTALL", "NSIS_HOOK_PREUNINSTALL", "Abort",
    ):
        if token not in nsis:
            fail(f"NSIS hook missing frozen token: {token}")
    if "currentUser" in nsis:
        fail("NSIS hook must not add a current-user SCM mutation branch")

    tree = ET.parse(WIX_FRAGMENT)
    root = tree.getroot()
    local = lambda tag: tag.rsplit("}", 1)[-1]
    if any(local(el.tag) in {"File", "Component", "RegistryKey", "RegistryValue"} for el in root.iter()):
        fail("WiX fragment owns files/components/registry state")
    feature = next((el for el in root.iter() if local(el.tag) == "Feature" and el.attrib.get("Id") == "Pkg0311AgentServiceLifecycle"), None)
    if feature is None:
        fail("WiX linker anchor feature missing")
    actions = {el.attrib.get("Id"): el for el in root.iter() if local(el.tag) == "CustomAction"}
    verbs = {
        "Pkg0311InstallService": "service install",
        "Pkg0311StartService": "service start",
        "Pkg0311StopService": "service stop",
        "Pkg0311RemoveService": "service uninstall",
    }
    for action_id, verb in verbs.items():
        action = actions.get(action_id)
        if action is None:
            fail(f"WiX custom action missing: {action_id}")
        attrs = action.attrib
        if attrs.get("Directory") != "INSTALLDIR" or attrs.get("Execute") != "deferred" or attrs.get("Impersonate") != "no" or attrs.get("Return") != "check":
            fail(f"WiX custom action privilege/return contract drifted: {action_id}")
        cmd = attrs.get("ExeCommand", "")
        if "[INSTALLDIR]bin\\vsn-agent.exe" not in cmd or verb not in cmd:
            fail(f"WiX custom action command mismatch: {action_id}")

    sequence = {el.attrib.get("Action"): (el.attrib, (el.text or "").strip()) for el in root.iter() if local(el.tag) == "Custom"}
    required_sequence = {
        "Pkg0311InstallService": ({"After": "InstallFiles"}, 'NOT Installed AND NOT REMOVE~="ALL"'),
        "Pkg0311StartService": ({"After": "Pkg0311InstallService"}, 'NOT Installed AND NOT REMOVE~="ALL"'),
        "Pkg0311StopService": ({"Before": "Pkg0311RemoveService"}, 'REMOVE~="ALL"'),
        "Pkg0311RemoveService": ({"Before": "RemoveFiles"}, 'REMOVE~="ALL"'),
    }
    for action_id, (ordering, condition) in required_sequence.items():
        attrs, actual_condition = sequence.get(action_id, ({}, ""))
        if any(attrs.get(k) != v for k, v in ordering.items()) or actual_condition != condition:
            fail(f"WiX sequence/condition mismatch: {action_id}")

    harness = HARNESS.read_text(encoding="utf-8")
    for token in ("current-user", "per-machine", "msi", "VSN-Agent", "NT AUTHORITY\\LocalService", "--service-run", "service stop", "service start", "ping", "tracked_repository_drift_zero"):
        if token not in harness:
            fail(f"lifecycle harness missing token: {token}")
    workflow = WORKFLOW.read_text(encoding="utf-8")
    for token in ("windows-2025", "22.12.0", "1.97.1", "tauri-cli 2.11.4", "--bundles nsis", "--bundles msi", "pkg03-0311-agent-service-lifecycle"):
        if token not in workflow:
            fail(f"workflow missing frozen token: {token}")

    print(json.dumps({
        "valid": True,
        "task": "03.11",
        "pause_lift_head": PAUSE_LIFT_HEAD,
        "implementation_paths": sorted(changed),
        "bounded_manifest_correction": True,
        "changed_file_count": len(changed),
        "new_file_count": len(added),
        "current_user_service_mutation": False,
        "single_agent_payload_owner": "03.10",
        "wix_custom_actions": sorted(actions),
    }, indent=2))


if __name__ == "__main__":
    main()
