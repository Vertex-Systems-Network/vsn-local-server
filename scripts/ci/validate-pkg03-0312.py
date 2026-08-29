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
MANIFEST_CORRECTION_PARENT = "7be65cd3a9c12395955ce5b32c897183f11fbb84"
MANIFEST_CORRECTION_HEAD = "d49e79e96a50934fa2dd1c958ea8b59b5a7dc8ff"
FAILED_MSI_HEAD = "c9792a7e5ab890c162ffb62ab3121cb0d9f4074f"

MANIFEST_PATH = ".ai/manifests/pkg03-0312-installer-acls-state.v1.json"
PLAN_PATH = ".ai/plans/pkg03-0312-installer-acls-state-v1.md"
PREFLIGHT_PATH = ".ai/features/pkg03-0312/development-preflight.md"
AMENDMENT_PATH = ".ai/changes/PKG03-0312-SECURITY-AMENDMENT-2026-08-29.md"
SECURITY_PATH = "crates/vsn-security/src/lib.rs"
TRACKER_PATH = "certification/pkg03-windows-installer-v1.json"
STATUS_PATH = "docs/MASTER-EXECUTION-STATUS.json"

IMPLEMENTATION_PATHS = {
    ".github/workflows/pkg03-0312-acl-state-lifecycle.yml",
    "scripts/ci/pkg03-0312-acl-state-lifecycle.ps1",
    "scripts/ci/validate-pkg03-0312.py",
}
PLANNING_CORRECTION_PATHS = {MANIFEST_PATH}
AMENDMENT_PATHS = {AMENDMENT_PATH}
AUTHORITY_PATHS = IMPLEMENTATION_PATHS | PLANNING_CORRECTION_PATHS | AMENDMENT_PATHS
PRE_ACCEPTANCE_PATHS = AUTHORITY_PATHS | {SECURITY_PATH}
PROJECTION_PATHS = {
    "certification/pkg03-windows-installer-v1.json",
    "docs/MASTER-EXECUTION-STATUS.json",
    "README.md",
    ".ai/README.md",
    "docs/MASTER-EXECUTION-PLAN.md",
}
POST_ACCEPTANCE_PATHS = PRE_ACCEPTANCE_PATHS | PROJECTION_PATHS

FROZEN_PRODUCT_PATHS = (
    "apps/agent/src/main.rs",
    "crates/vsn-config/src/lib.rs",
    "apps/desktop/src-tauri/tauri.windows.conf.json",
    "apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh",
    "apps/desktop/src-tauri/windows/fragments/pkg03-0311-agent-service.wxs",
    "installer/windows/owned-payload.v1.json",
    "scripts/ci/pkg03-0311-agent-service-lifecycle.ps1",
)


def fail(message: str) -> None:
    raise SystemExit(f"PKG-03 03.12 validation failed: {message}")


def run(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    p = subprocess.run(args, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if check and p.returncode:
        fail(f"command failed ({' '.join(args)}): {p.stderr.strip()}")
    return p


def is_ancestor(ancestor: str, descendant: str = "HEAD") -> bool:
    return run("git", "merge-base", "--is-ancestor", ancestor, descendant, check=False).returncode == 0


def git_bytes(path: str, ref: str = "HEAD") -> bytes:
    p = subprocess.run(["git", "show", f"{ref}:{path}"], cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if p.returncode:
        fail(f"unable to read {ref}:{path}: {p.stderr.decode(errors='replace').strip()}")
    return p.stdout


def text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def require_tokens(value: str, tokens: tuple[str, ...], label: str) -> None:
    for token in tokens:
        if token not in value:
            fail(f"{label} missing required token: {token}")


def validate_frozen_planning() -> None:
    for path in (MANIFEST_PATH, PLAN_PATH, PREFLIGHT_PATH):
        if not (ROOT / path).is_file():
            fail(f"missing planning artifact: {path}")

    correction_delta = {
        line
        for line in run(
            "git", "diff", "--name-only", f"{MANIFEST_CORRECTION_PARENT}..{MANIFEST_CORRECTION_HEAD}"
        ).stdout.splitlines()
        if line
    }
    if correction_delta != PLANNING_CORRECTION_PATHS:
        fail(f"manifest correction commit is not path-bounded: {sorted(correction_delta)}")
    if git_bytes(MANIFEST_PATH) != git_bytes(MANIFEST_PATH, MANIFEST_CORRECTION_HEAD):
        fail("corrected 03.12 manifest drifted after manifest correction authority")
    if git_bytes(PLAN_PATH) != git_bytes(PLAN_PATH, PLANNING_HEAD):
        fail("frozen 03.12 plan drifted after planning authorization")
    if git_bytes(PREFLIGHT_PATH) != git_bytes(PREFLIGHT_PATH, PLANNING_HEAD):
        fail("frozen 03.12 preflight drifted after planning authorization")

    manifest = json.loads(text(MANIFEST_PATH))
    expected_identity = (
        manifest.get("feature_id"),
        manifest.get("version"),
        manifest.get("canonical_base_sha"),
        manifest.get("change_classification", {}).get("classification"),
        manifest.get("gap", {}).get("classification"),
        manifest.get("approval", {}).get("scope"),
        manifest.get("approval", {}).get("approval_ref"),
        manifest.get("parallel_safety", {}).get("classification"),
    )
    wanted = (
        "pkg03-0312-installer-acls-state",
        "1.0.0",
        CANONICAL_BASE,
        "COMPLETION",
        "MISSING_IMPLEMENTATION",
        "TASK",
        "conversation:user-2026-08-29-continue-0312",
        "SERIALIZE",
    )
    if expected_identity != wanted:
        fail(f"manifest authority/classification drifted: {expected_identity}")
    if manifest.get("plan", {}).get("path") != PLAN_PATH:
        fail("manifest plan path drifted")
    digest = hashlib.sha256(git_bytes(PLAN_PATH)).hexdigest()
    if manifest.get("plan", {}).get("sha256") != digest:
        fail("manifest plan digest does not match frozen plan Git bytes")
    storage = (
        manifest.get("specification", {})
        .get("modules", [{}])[0]
        .get("options", [{}])[0]
        .get("storage")
    )
    if storage != r"%PROGRAMDATA%\VSN\security\ipc.key":
        fail(f"corrected manifest storage contract drifted: {storage!r}")


def validate_security_amendment() -> None:
    if not (ROOT / AMENDMENT_PATH).is_file():
        fail(f"missing approved security amendment: {AMENDMENT_PATH}")
    amendment = text(AMENDMENT_PATH)
    require_tokens(
        amendment,
        (
            "PKG-03 03.12 Security Amendment",
            "conversation:user-2026-08-29-continue-security-amendment",
            FAILED_MSI_HEAD,
            "33222396953",
            "99019015513",
            "9705943122",
            "sha256:d4e7a0e055eeabeccec7962cfc4444f018eb29e5cca108fdc62f0827361270a8",
            "PLAN_REALITY_MISMATCH",
            "SECURITY_ASSUMPTION_CHANGE",
            SECURITY_PATH,
            "SYSTEM=FullControl",
            "Administrators=FullControl",
            "LocalService=Read",
            "ordinary creator",
            "03.17",
        ),
        "03.12 security amendment",
    )


def validate_accepted_integration(mode: str) -> None:
    for path in FROZEN_PRODUCT_PATHS:
        if not (ROOT / path).is_file():
            fail(f"missing accepted integration path: {path}")
        if git_bytes(path) != git_bytes(path, CANONICAL_BASE):
            fail(f"forbidden accepted product/integration drift: {path}")

    security = text(SECURITY_PATH)
    require_tokens(
        security,
        (
            'program_data.join("VSN").join("security")',
            'directory.join("ipc.key")',
            '*S-1-5-18:(F)',
            '*S-1-5-32-544:(F)',
            '*S-1-5-19:(R)',
            '*S-1-5-18:(OI)(CI)(F)',
            '*S-1-5-32-544:(OI)(CI)(F)',
            '*S-1-5-19:(OI)(CI)(R)',
            '"/inheritance:r"',
        ),
        "vsn-security Windows IPC ACL authority",
    )
    if mode == "amendment_authorized":
        if git_bytes(SECURITY_PATH) != git_bytes(SECURITY_PATH, CANONICAL_BASE):
            fail("vsn-security changed before the approved amendment implementation slice")
    else:
        if git_bytes(SECURITY_PATH) == git_bytes(SECURITY_PATH, CANONICAL_BASE):
            fail("approved 03.12 security correction is missing")
        require_tokens(
            security,
            (
                "enum WindowsIpcAclPrincipal",
                "fn windows_ipc_creator_principal",
                "fn windows_ipc_file_grants",
                "fn windows_ipc_directory_grants",
                "WindowsIpcAclPrincipal::System",
                "WindowsIpcAclPrincipal::Administrators",
                "WindowsIpcAclPrincipal::LocalService",
                "windows_ipc_system_creator_preserves_full_control",
                "windows_ipc_local_service_creator_does_not_gain_write",
                "windows_ipc_ordinary_creator_retains_expected_rights",
            ),
            "amended vsn-security creator ACL semantics",
        )

    agent = text("apps/agent/src/main.rs")
    require_tokens(
        agent,
        (
            "vsn_core::provision_local_ipc()?;",
            '"start=",',
            '"auto",',
            '"obj=",',
            '"NT AUTHORITY\\\\LocalService",',
            "SERVICE_DISPLAY_NAME",
            "--service-run",
        ),
        "accepted Agent service-install authority",
    )

    config = text("crates/vsn-config/src/lib.rs")
    require_tokens(
        config,
        (
            'ProjectDirs::from("dev", "VSN", "VSN Platform")',
            'dirs.config_dir().join("config.json")',
        ),
        "vsn-config ProjectDirs authority",
    )

    nsis = text("apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh")
    require_tokens(
        nsis,
        (
            '!if "${INSTALLMODE}" == "perMachine"',
            "service install",
            "service start",
            "service stop",
            "service uninstall",
        ),
        "accepted 03.11 NSIS service hook",
    )

    windows = json.loads(text("apps/desktop/src-tauri/tauri.windows.conf.json"))["bundle"]["windows"]
    if windows.get("nsis", {}).get("installerHooks") != "./windows/pkg03-0311-agent-service.nsh":
        fail("accepted 03.11 NSIS hook reference drifted")
    if windows.get("wix", {}).get("fragmentPaths") != ["./windows/fragments/pkg03-0311-agent-service.wxs"]:
        fail("accepted 03.11 WiX fragment reference drifted")
    if windows.get("wix", {}).get("featureRefs") != ["Pkg0311AgentServiceLifecycle"]:
        fail("accepted 03.11 WiX feature reference drifted")


def validate_task_certification_surfaces() -> None:
    harness_path = "scripts/ci/pkg03-0312-acl-state-lifecycle.ps1"
    workflow_path = ".github/workflows/pkg03-0312-acl-state-lifecycle.yml"
    for path in IMPLEMENTATION_PATHS:
        if not (ROOT / path).is_file():
            fail(f"missing 03.12 implementation file: {path}")

    require_tokens(
        text(harness_path),
        (
            "task_id='03.12'",
            "scripts/ci/pkg03-0311-agent-service-lifecycle.ps1",
            "Assert-IpcAclContract",
            "Invoke-LocalServiceProjectDirsProbe",
            "S-1-5-18",
            "S-1-5-32-544",
            "S-1-5-19",
            "machine_security_created_by_current_user_install=$false",
            "comprehensive_uninstall_preservation_owner='03.17'",
            "tracked_repository_drift_zero",
        ),
        "03.12 lifecycle harness",
    )
    require_tokens(
        text(workflow_path),
        (
            "name: PKG-03 03.12 Installer ACL State Lifecycle",
            "python scripts/ci/validate-pkg03-0312.py",
            "Build exact-head current-user NSIS",
            "Build exact-head per-machine NSIS",
            "Build exact-head MSI/WiX",
            "Exercise genuine 03.12 ACL and state lifecycle",
            "Verify exact 03.12 evidence",
            "pkg03-0312-installer-acl-state-evidence",
        ),
        "03.12 lifecycle workflow",
    )


def validate_canonical_state(mode: str) -> None:
    tracker = json.loads(text(TRACKER_PATH))
    if tracker.get("package_id") != "PKG-03" or tracker.get("required") != 25:
        fail("PKG-03 tracker identity/denominator drifted")
    tasks = {task["id"]: task for task in tracker.get("tasks", [])}
    for dep in ("03.07", "03.10", "03.11"):
        if tasks.get(dep, {}).get("status") != "DONE":
            fail(f"03.12 dependency {dep} is not DONE")
    task = tasks.get("03.12")
    if not task:
        fail("03.12 task missing from tracker")

    master = json.loads(text(STATUS_PATH))
    packages = {pkg["id"]: pkg for pkg in master.get("packages", [])}
    if master.get("product_version") != "0.38.1" or packages.get("PKG-03", {}).get("required") != 25:
        fail("master product version or PKG-03 denominator drifted")

    if mode != "post_acceptance":
        if (tracker.get("done"), tracker.get("percent"), tracker.get("active_task")) != (11, 44.0, "03.12"):
            fail("pre-acceptance PKG-03 progress/cursor drifted")
        if task.get("status") != "READY":
            fail("03.12 must remain READY before genuine exact-head evidence")
        return

    if (tracker.get("done"), tracker.get("percent")) != (12, 48.0):
        fail("post-acceptance PKG-03 projection must be 12/25 = 48%")
    if task.get("status") != "DONE":
        fail("post-acceptance 03.12 is not DONE")
    evidence = task.get("evidence")
    if not isinstance(evidence, dict):
        fail("post-acceptance 03.12 evidence object missing")
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
    )
    for key in required:
        if not evidence.get(key):
            fail(f"post-acceptance 03.12 evidence missing {key}")
    source = str(evidence["source_commit"])
    if len(source) != 40 or not is_ancestor(UNPAUSE_HEAD, source) or not is_ancestor(source, "HEAD"):
        fail("post-acceptance source_commit is not on authorized 03.12 lineage")
    if not str(evidence["artifact_digest"]).startswith("sha256:"):
        fail("post-acceptance artifact digest is not SHA-256 bound")
    if len(str(evidence["evidence_sha256"])) != 64:
        fail("post-acceptance evidence SHA-256 malformed")
    for path in PRE_ACCEPTANCE_PATHS:
        if git_bytes(path) != git_bytes(path, source):
            fail(f"03.12 behavior/authority drifted after exact-head evidence: {path}")


def main() -> None:
    for ancestor in (
        CANONICAL_BASE,
        PLANNING_HEAD,
        UNPAUSE_HEAD,
        MANIFEST_CORRECTION_PARENT,
        MANIFEST_CORRECTION_HEAD,
        FAILED_MSI_HEAD,
    ):
        if not is_ancestor(ancestor):
            fail(f"required authority/evidence head is not an ancestor of HEAD: {ancestor}")

    changed = {
        line
        for line in run("git", "diff", "--name-only", f"{UNPAUSE_HEAD}..HEAD").stdout.splitlines()
        if line
    }
    if changed == AUTHORITY_PATHS:
        mode = "amendment_authorized"
    elif changed == PRE_ACCEPTANCE_PATHS:
        mode = "pre_acceptance"
    elif changed == POST_ACCEPTANCE_PATHS:
        mode = "post_acceptance"
    else:
        fail(f"unexpected post-unpause path set: {sorted(changed)}")

    added = {
        line
        for line in run("git", "diff", "--diff-filter=A", "--name-only", f"{UNPAUSE_HEAD}..HEAD").stdout.splitlines()
        if line
    }
    expected_added = IMPLEMENTATION_PATHS | AMENDMENT_PATHS
    if added != expected_added:
        fail(f"unexpected newly-added 03.12 paths: {sorted(added)}")

    validate_frozen_planning()
    validate_security_amendment()
    validate_accepted_integration(mode)
    validate_task_certification_surfaces()
    validate_canonical_state(mode)

    print(
        "PKG-03 03.12 static authority PASS: security amendment bound; "
        f"mode={mode}; product correction surface={SECURITY_PATH}"
    )


if __name__ == "__main__":
    main()
