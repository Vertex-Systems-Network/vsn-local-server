#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CANONICAL_BASE = "0eaa4abb7c5e817334f13672952a5901fbbc8fa9"
PLANNING_HEAD = "7b9e6143eca468f2573bef2a1f2e211994c426b6"
UNPAUSE_HEAD = "5ab1a1caaf4e29fdf947e208051755fca32a5c67"

MANIFEST = ROOT / ".ai/manifests/pkg03-0312-installer-acls-state.v1.json"
PLAN = ROOT / ".ai/plans/pkg03-0312-installer-acls-state-v1.md"
PREFLIGHT = ROOT / ".ai/features/pkg03-0312/development-preflight.md"
TRACKER = ROOT / "certification/pkg03-windows-installer-v1.json"
STATUS = ROOT / "docs/MASTER-EXECUTION-STATUS.json"
OWNERSHIP = ROOT / "installer/windows/owned-payload.v1.json"
SECURITY = ROOT / "crates/vsn-security/src/lib.rs"
CONFIG = ROOT / "crates/vsn-config/src/lib.rs"
AGENT = ROOT / "apps/agent/src/main.rs"
WINDOWS_CONFIG = ROOT / "apps/desktop/src-tauri/tauri.windows.conf.json"
NSIS_0311 = ROOT / "apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh"
WIX_0311 = ROOT / "apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs"
HARNESS_0311 = ROOT / "scripts/ci/pkg03-0311-agent-service-lifecycle.ps1"
HARNESS = ROOT / "scripts/ci/pkg03-0312-acl-state-lifecycle.ps1"
WORKFLOW = ROOT / ".github/workflows/pkg03-0312-acl-state-lifecycle.yml"

IMPLEMENTATION_PATHS = {
    "scripts/ci/pkg03-0312-acl-state-lifecycle.ps1",
    "scripts/ci/validate-pkg03-0312.py",
    ".github/workflows/pkg03-0312-acl-state-lifecycle.yml",
}
STATE_PROJECTION_PATHS = {
    "certification/pkg03-windows-installer-v1.json",
    "docs/MASTER-EXECUTION-STATUS.json",
}
LIVE_PROJECTION_PATHS = {
    "README.md",
    ".ai/README.md",
    "docs/MASTER-EXECUTION-PLAN.md",
}
POST_ACCEPTANCE_PATHS = IMPLEMENTATION_PATHS | STATE_PROJECTION_PATHS | LIVE_PROJECTION_PATHS
FROZEN_PRODUCT_PATHS = (
    "apps/agent/src/main.rs",
    "crates/vsn-security/src/lib.rs",
    "crates/vsn-config/src/lib.rs",
    "apps/desktop/src-tauri/tauri.windows.conf.json",
    "apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh",
    "apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs",
    "installer/windows/owned-payload.v1.json",
    "scripts/ci/pkg03-0311-agent-service-lifecycle.ps1",
)

EXPECTED_SECURITY_TOKENS = (
    'program_data.join("VSN").join("security")',
    'let path = directory.join("ipc.key");',
    'let system = "*S-1-5-18:(F)";',
    'let administrators = "*S-1-5-32-544:(F)";',
    'let local_service = "*S-1-5-19:(R)";',
    'let system = "*S-1-5-18:(OI)(CI)(F)";',
    'let administrators = "*S-1-5-32-544:(OI)(CI)(F)";',
    'let local_service = "*S-1-5-19:(OI)(CI)(R)";',
    '"/inheritance:r"',
)
EXPECTED_AGENT_TOKENS = (
    'vsn_core::provision_local_ipc()?;',
    '"create",',
    'SERVICE_NAME,',
    '"start=",',
    '"auto",',
    '"obj=",',
    r'"NT AUTHORITY\\LocalService",',
    '"DisplayName=",',
    'SERVICE_DISPLAY_NAME,',
    '--service-run',
)
EXPECTED_NSIS_TOKENS = (
    '!if "${INSTALLMODE}" == "perMachine"',
    r"""nsExec::ExecToLog '"$INSTDIR\bin\vsn-agent.exe" service install'""",
    r"""nsExec::ExecToLog '"$INSTDIR\bin\vsn-agent.exe" service start'""",
    r"""nsExec::ExecToLog '"$INSTDIR\bin\vsn-agent.exe" service stop'""",
    r"""nsExec::ExecToLog '"$INSTDIR\bin\vsn-agent.exe" service uninstall'""",
)
EXPECTED_CONFIG_TOKENS = (
    'ProjectDirs::from("dev", "VSN", "VSN Platform")',
    'dirs.config_dir().join("config.json")',
)
EXPECTED_HARNESS_TOKENS = (
    "task_id='03.12'",
    "S-1-5-18",
    "S-1-5-32-544",
    "S-1-5-19",
    "Assert-IpcAclContract",
    "Invoke-LocalServiceProjectDirsProbe",
    "scripts/ci/pkg03-0311-agent-service-lifecycle.ps1",
    "machine_security_created_by_current_user_install=$false",
    "tracked_repository_drift_zero",
    "comprehensive_uninstall_preservation_owner='03.17'",
)
EXPECTED_WORKFLOW_TOKENS = (
    "name: PKG-03 03.12 Installer ACL State Lifecycle",
    "python scripts/ci/validate-pkg03-0312.py",
    "pkg03-0312-acl-state-lifecycle.ps1",
    "Build exact-head current-user NSIS",
    "Build exact-head per-machine NSIS",
    "Build exact-head MSI/WiX",
    "Verify exact 03.12 evidence",
    "pkg03-0312-installer-acl-state-evidence",
)

def fail(message: str) -> None:
    raise SystemExit("PKG-03 03.12 validation failed: " + message)

def run(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    proc = subprocess.run(
        args, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE
    )
    if check and proc.returncode:
        fail(f"command failed ({' '.join(args)}): {proc.stderr.strip()}")
    return proc

def is_ancestor(ancestor: str, descendant: str = "HEAD") -> bool:
    return run("git", "merge-base", "--is-ancestor", ancestor, descendant, check=False).returncode == 0

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

def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()

def require_tokens(text: str, tokens: tuple[str, ...], label: str) -> None:
    for token in tokens:
        if token not in text:
            fail(f"{label} missing token: {token}")

def validate_static_authority() -> None:
    security = SECURITY.read_text(encoding="utf-8")
    config = CONFIG.read_text(encoding="utf-8")
    agent = AGENT.read_text(encoding="utf-8")
    nsis = NSIS_0311.read_text(encoding="utf-8")
    harness = HARNESS.read_text(encoding="utf-8")
    workflow = WORKFLOW.read_text(encoding="utf-8")

    require_tokens(security, EXPECTED_SECURITY_TOKENS, "vsn-security")
    require_tokens(config, EXPECTED_CONFIG_TOKENS, "vsn-config")
    require_tokens(agent, EXPECTED_AGENT_TOKENS, "Agent service install")
    require_tokens(nsis, EXPECTED_NSIS_TOKENS, "accepted 03.11 NSIS service hook")
    require_tokens(harness, EXPECTED_HARNESS_TOKENS, "03.12 harness")
    require_tokens(workflow, EXPECTED_WORKFLOW_TOKENS, "03.12 workflow")

    windows_config = json.loads(WINDOWS_CONFIG.read_text(encoding="utf-8"))
    windows = windows_config["bundle"]["windows"]
    if windows["nsis"].get("installerHooks") != "./windows/pkg03-0311-agent-service.nsh":
        fail("03.11 NSIS hook authority drifted")
    wix = windows.get("wix", {})
    if wix.get("fragmentPaths") != ["./windows/fragments/pkg03-0311-agent-service.wxs"]:
        fail("03.11 WiX fragment authority drifted")
    if wix.get("featureRefs") != ["Pkg0311AgentServiceLifecycle"]:
        fail("03.11 WiX feature authority drifted")

    # The accepted 03.11 service-install invocation already provisions the authoritative
    # machine IPC state. 03.12 therefore certifies that path instead of adding a second ACL writer.
    for path in FROZEN_PRODUCT_PATHS:
        if git_bytes(path, "HEAD") != git_bytes(path, CANONICAL_BASE):
            fail(f"forbidden product/integration drift from accepted base: {path}")

def validate_manifest_and_plan() -> None:
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
        "pkg03-0312-installer-acls-state",
        "1.0.0",
        "COMPLETION",
        "MISSING_IMPLEMENTATION",
        "TASK",
        "SERIALIZE",
    ):
        fail(f"manifest identity/classification drifted: {identity}")
    if manifest.get("canonical_base_sha") != CANONICAL_BASE:
        fail("manifest canonical base drifted")
    if manifest.get("approval", {}).get("approval_ref") != "conversation:user-2026-08-29-continue-0312":
        fail("manifest approval reference drifted")
    if manifest.get("plan", {}).get("path") != ".ai/plans/pkg03-0312-installer-acls-state-v1.md":
        fail("manifest plan path drifted")
    plan_sha = manifest.get("plan", {}).get("sha256")
    if plan_sha != sha256_bytes(PLAN.read_bytes()):
        fail("manifest plan digest does not match frozen plan bytes")
    if git_bytes(str(PLAN.relative_to(ROOT)), "HEAD") != git_bytes(str(PLAN.relative_to(ROOT)), PLANNING_HEAD):
        fail("frozen plan drifted after planning authorization")
    if git_bytes(str(MANIFEST.relative_to(ROOT)), "HEAD") != git_bytes(str(MANIFEST.relative_to(ROOT)), PLANNING_HEAD):
        fail("frozen manifest drifted after planning authorization")

def validate_canonical_state(mode: str) -> None:
    tracker = json.loads(TRACKER.read_text(encoding="utf-8"))
    if tracker.get("package_id") != "PKG-03" or tracker.get("required") != 25:
        fail("PKG-03 tracker identity/denominator drifted")
    tasks = {item["id"]: item for item in tracker.get("tasks", [])}
    for dep in ("03.07", "03.10", "03.11"):
        if tasks.get(dep, {}).get("status") != "DONE":
            fail(f"03.12 prerequisite {dep} is not DONE")
    task = tasks.get("03.12")
    if not task:
        fail("03.12 task missing from tracker")

    status = json.loads(STATUS.read_text(encoding="utf-8"))
    packages = {item["id"]: item for item in status.get("packages", [])}
    pkg03 = packages.get("PKG-03", {})
    if status.get("product_version") != "0.38.1" or pkg03.get("required") != 25:
        fail("master product version or PKG-03 denominator drifted")

    if mode == "pre_acceptance":
        if (tracker.get("done"), tracker.get("active_task"), tracker.get("percent")) != (11, "03.12", 44.0):
            fail("pre-acceptance tracker progress/cursor drifted")
        if task.get("status") != "READY":
            fail("03.12 must remain READY before evidence projection")
        return

    if (tracker.get("done"), tracker.get("percent")) != (12, 48.0):
        fail("post-acceptance tracker must project 12/25 = 48%")
    if task.get("status") != "DONE":
        fail("post-acceptance 03.12 is not DONE")
    evidence = task.get("evidence")
    if not isinstance(evidence, dict):
        fail("DONE 03.12 is missing evidence")
    for key in (
        "source_commit",
        "workflow_run",
        "job",
        "artifact",
        "artifact_digest",
        "evidence_sha256",
        "current_user_setup_sha256",
        "per_machine_setup_sha256",
        "msi_sha256",
    ):
        if not evidence.get(key):
            fail(f"DONE 03.12 evidence missing {key}")
    source = str(evidence["source_commit"])
    if len(source) != 40 or not is_ancestor(UNPAUSE_HEAD, source) or not is_ancestor(source, "HEAD"):
        fail("03.12 evidence source is not on the authorized lineage")
    for path in IMPLEMENTATION_PATHS:
        if git_bytes(path, "HEAD") != git_bytes(path, source):
            fail(f"post-evidence implementation drift: {path}")
    if not str(evidence["artifact_digest"]).startswith("sha256:"):
        fail("03.12 artifact digest is not SHA-256 bound")
    if len(str(evidence["evidence_sha256"])) != 64:
        fail("03.12 evidence digest malformed")

def main() -> None:
    required = (
        MANIFEST, PLAN, PREFLIGHT, TRACKER, STATUS, OWNERSHIP, SECURITY, CONFIG, AGENT,
        WINDOWS_CONFIG, NSIS_0311, WIX_0311, HARNESS_0311, HARNESS, WORKFLOW
    )
    for path in required:
        if not path.is_file():
            fail(f"missing required file: {path.relative_to(ROOT)}")

    for ancestor in (CANONICAL_BASE, PLANNING_HEAD, UNPAUSE_HEAD):
        if not is_ancestor(ancestor):
            fail(f"required authority head is not an ancestor: {ancestor}")

    changed = {
        line for line in run("git", "--name-only", f"{UNPAUSE_HEAD}..HEAD").stdout.splitlines()
        if line
    }
    if changed == IMPLEMENTATION_PATHS:
        mode = "pre_acceptance"
    elif changed == POST_ACCEPTANCE_PATHS:
        mode = "post_acceptance"
    else:
        fail(
            "post-unpause delta must be exactly the three certification implementation paths "
            "or those paths plus the bounded canonical/live projection paths; "
            f"got {sorted(changed)}"
        )

    added = {
        line for line in run("git", "diff", "--diff-filter=A", "--name-only", f"{UNPAUSE_HEAD}..HEAD").stdout.splitlines()
        if line
    }
    expected_added = IMPLEMENTATION_PATHS
    if added != expected_added:
        fail(f"only the three task-owned certification files may be newly added; got {sorted(added)}")

    validate_manifest_and_plan()
    validate_static_authority()
    validate_canonical_state(mode)

    print(
        "PKG-03 03.12 static authority PASS: accepted 03.11 integration retained; "
        f"mode={mode}; exact-head certification surfaces={sorted(IMPLEMENTATION_PATHS)}"
    )

if __name__ == "__main__":
    main()
