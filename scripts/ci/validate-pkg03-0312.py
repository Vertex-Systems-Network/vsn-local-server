#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TASK = "03.12"
CANONICAL_BASE = "0eaa4abb7c5e817334f13672952a5901fbbc8fa9"
ACCEPTED_SOURCE = "24645d61d94169bce64f19d29cf7ef72991726b5"
TRACKER_PATH = "certification/pkg03-windows-installer-v1.json"
STATUS_PATH = "docs/MASTER-EXECUTION-STATUS.json"
SECURITY_PATH = "crates/vsn-security/src/lib.rs"

# These task-owned surfaces were frozen by the accepted 03.12 source and must
# remain byte-identical on all later package descendants. The validator itself
# is intentionally excluded so this historical-state repair can evolve without
# weakening the frozen implementation/harness/planning authority.
FROZEN_BLOBS = {
    ".github/workflows/pkg03-0312-acl-state-lifecycle.yml": "57810f6ebe8639415de41f330468a6b8142ff09d",
    "scripts/ci/pkg03-0312-acl-state-lifecycle.ps1": "f848502e9b18dbe74c7d3fb9d6a800a3c15c5c86",
    ".ai/manifests/pkg03-0312-installer-acls-state.v1.json": "10f7dfeb384f392e91e5b36e487fe5c4883420c3",
    ".ai/plans/pkg03-0312-installer-acls-state-v1.md": "b978e99431f0b07950c8b676f886147f1ce8dd1c",
    ".ai/features/pkg03-0312/development-preflight.md": "0610cb4c79203f8fd488e59908024d05d8cdaee5",
    ".ai/changes/PKG03-0312-SECURITY-AMENDMENT-2026-08-29.md": "8cb78d27dc9fcbe0f260247931334b8d05c69548",
}

# The first blob is the exact accepted 03.12 security source. The second is the
# formatter-produced equivalent used by the isolated rustfmt repair. No other
# security blob is authorized by this historical validator without explicit
# review/update, so semantic mutations cannot be hidden as formatting drift.
ALLOWED_SECURITY_BLOBS = {
    "f49c24b836a67c97de8ce268e34bb6787eba4413",
    "647451a2056b2e2ef79a5a363b6467abcbf458a7",
}

ACCEPTED_EVIDENCE = {
    "source_commit": ACCEPTED_SOURCE,
    "workflow_run": 33225164815,
    "job": 99027271153,
    "artifact": 9706868086,
    "artifact_digest": "sha256:ac152e4706eec15fcead24094c3b819457e37201b3063087602ca963255f8ab0",
    "evidence_sha256": "2e0166d9bd6b3729004925aa86d16608f03eee37981e79c93ae253522b25984a",
    "current_user_setup_sha256": "bd515d7700507aebcb2d063757abd92dc59b4c93ac77fc61097c08abef852c58",
    "per_machine_setup_sha256": "5870eb9a5d6b1d0ce46e55ed152263eac4c61ed340062f02fcf18e84aef8420b",
    "msi_sha256": "c9da9d2e9caf7077c0d2270081937e4c3ded5beefc153ea168a84c83e9fe49f3",
    "product_code": "{58705914-E928-4BAF-9E8E-28EBB8BA33F9}",
    "upgrade_code": "{157F304F-1D1B-55E0-B89C-0610EA27C645}",
    "task_manifest_sha256": "e5da603e8f85f46598d435e3d418eb25f64317e4048ec0e90d53d6bfef538d6c",
}


def fail(message: str) -> None:
    raise SystemExit(f"PKG-03 03.12 validation failed: {message}")


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


def is_ancestor(ancestor: str, descendant: str = "HEAD") -> bool:
    return run(
        "git", "merge-base", "--is-ancestor", ancestor, descendant, check=False
    ).returncode == 0


def git_bytes(path: str, ref: str = "HEAD") -> bytes:
    proc = subprocess.run(
        ["git", "show", f"{ref}:{path}"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if proc.returncode:
        fail(
            f"unable to read {ref}:{path}: "
            f"{proc.stderr.decode(errors='replace').strip()}"
        )
    return proc.stdout


def blob_sha(path: str, ref: str = "HEAD") -> str:
    proc = run("git", "rev-parse", f"{ref}:{path}")
    value = proc.stdout.strip()
    if len(value) != 40:
        fail(f"unexpected blob id for {ref}:{path}: {value!r}")
    return value


def text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def require_tokens(value: str, tokens: tuple[str, ...], label: str) -> None:
    for token in tokens:
        if token not in value:
            fail(f"{label} missing required token: {token}")


def task_map(tracker: dict) -> dict[str, dict]:
    return {item.get("id"): item for item in tracker.get("tasks", [])}


def validate_frozen_task_surfaces() -> None:
    for path, expected_blob in FROZEN_BLOBS.items():
        if not (ROOT / path).is_file():
            fail(f"missing frozen 03.12 surface: {path}")
        actual_blob = blob_sha(path)
        if actual_blob != expected_blob:
            fail(
                f"frozen 03.12 surface drifted: {path}; "
                f"expected_blob={expected_blob} actual_blob={actual_blob}"
            )

    manifest = json.loads(
        text(".ai/manifests/pkg03-0312-installer-acls-state.v1.json")
    )
    identity = (
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
    if identity != wanted:
        fail(f"manifest authority/classification drifted: {identity}")

    plan_path = ".ai/plans/pkg03-0312-installer-acls-state-v1.md"
    if manifest.get("plan", {}).get("path") != plan_path:
        fail("manifest plan path drifted")
    if manifest.get("plan", {}).get("sha256") != hashlib.sha256(
        git_bytes(plan_path)
    ).hexdigest():
        fail("manifest plan digest no longer matches frozen tracked plan bytes")

    amendment = text(
        ".ai/changes/PKG03-0312-SECURITY-AMENDMENT-2026-08-29.md"
    )
    require_tokens(
        amendment,
        (
            "PKG-03 03.12 Security Amendment",
            "conversation:user-2026-08-29-continue-security-amendment",
            "PLAN_REALITY_MISMATCH",
            "SECURITY_ASSUMPTION_CHANGE",
            "SYSTEM=FullControl",
            "Administrators=FullControl",
            "LocalService=Read",
            "ordinary creator",
            "03.17",
        ),
        "03.12 security amendment",
    )


def validate_security_contract() -> None:
    actual_blob = blob_sha(SECURITY_PATH)
    if actual_blob not in ALLOWED_SECURITY_BLOBS:
        fail(
            "vsn-security is neither the accepted 03.12 blob nor the exact "
            f"formatter-only successor: {actual_blob}"
        )

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
            "enum WindowsIpcAclPrincipal",
            "fn windows_ipc_creator_principal",
            "fn windows_ipc_file_grants",
            "fn windows_ipc_directory_grants",
            "windows_ipc_system_creator_preserves_full_control",
            "windows_ipc_local_service_creator_does_not_gain_write",
            "windows_ipc_ordinary_creator_retains_expected_rights",
        ),
        "accepted Windows IPC ACL contract",
    )


def validate_current_product_contract() -> None:
    # Later accepted PKG-03 tasks legitimately changed surrounding installer
    # integration, so descendants validate required behavior tokens rather than
    # incorrectly demanding byte equality with the old 03.12 activation base.
    agent = text("apps/agent/src/main.rs")
    require_tokens(
        agent,
        (
            "vsn_core::provision_local_ipc()?;",
            '"NT AUTHORITY\\\\LocalService",',
            "SERVICE_DISPLAY_NAME",
            "--service-run",
        ),
        "Agent service contract",
    )

    config = text("crates/vsn-config/src/lib.rs")
    require_tokens(
        config,
        (
            'ProjectDirs::from("dev", "VSN", "VSN Platform")',
            'dirs.config_dir().join("config.json")',
        ),
        "ProjectDirs contract",
    )

    nsis = text("apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh")
    require_tokens(
        nsis,
        (
            '!if "${INSTALLMODE}" == "perMachine"',
            '"$INSTDIR\\bin\\vsn-agent.exe" service install',
            '"$INSTDIR\\bin\\vsn-agent.exe" service start',
            '"$SYSDIR\\sc.exe" stop VSN-Agent',
            '"$SYSDIR\\sc.exe" delete VSN-Agent',
            'StrCmp $0 "1062" pkg0311_service_stop_ok',
            'StrCmp $0 "1060" pkg0311_service_remove_ok',
            'StrCmp $0 "1072" pkg0311_service_remove_ok',
        ),
        "Agent NSIS service contract",
    )

    windows = json.loads(text("apps/desktop/src-tauri/tauri.windows.conf.json"))["bundle"]["windows"]
    if windows.get("nsis", {}).get("installerHooks") != "./windows/pkg03-0311-agent-service.nsh":
        fail("accepted NSIS service hook reference drifted")
    if windows.get("wix", {}).get("fragmentPaths") != [
        "./windows/fragments/pkg03-0311-agent-service.wxs"
    ]:
        fail("accepted WiX service fragment reference drifted")
    if windows.get("wix", {}).get("featureRefs") != [
        "Pkg0311AgentServiceLifecycle"
    ]:
        fail("accepted WiX service feature reference drifted")


def validate_certification_surfaces() -> None:
    harness = text("scripts/ci/pkg03-0312-acl-state-lifecycle.ps1")
    require_tokens(
        harness,
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

    workflow = text(".github/workflows/pkg03-0312-acl-state-lifecycle.yml")
    require_tokens(
        workflow,
        (
            "name: PKG-03 03.12 Installer ACL State Lifecycle",
            "python scripts/ci/validate-pkg03-0312.py",
            "Run focused Windows IPC ACL tests",
            "Build exact-head current-user NSIS",
            "Build exact-head per-machine NSIS",
            "Build exact-head MSI/WiX",
            "Exercise genuine 03.12 ACL and state lifecycle",
            "Verify exact 03.12 evidence",
            "pkg03-0312-installer-acl-state-evidence",
        ),
        "03.12 workflow",
    )


def validate_canonical_descendant() -> None:
    tracker = json.loads(text(TRACKER_PATH))
    if tracker.get("package_id") != "PKG-03" or tracker.get("required") != 25:
        fail("PKG-03 tracker identity/denominator drifted")

    done = tracker.get("done")
    percent = tracker.get("percent")
    if not isinstance(done, int) or not 12 <= done <= 25:
        fail(f"accepted descendant has invalid PKG-03 done count: {done!r}")
    expected_percent = round(done * 100.0 / 25, 2)
    if percent != expected_percent:
        fail(
            f"PKG-03 percent mismatch for descendant: done={done} "
            f"percent={percent!r} expected={expected_percent}"
        )

    tasks = task_map(tracker)
    for dep in ("03.07", "03.10", "03.11"):
        if tasks.get(dep, {}).get("status") != "DONE":
            fail(f"03.12 dependency {dep} is no longer DONE")

    task = tasks.get(TASK, {})
    if task.get("status") != "DONE":
        fail("accepted descendant no longer records 03.12 as DONE")
    if task.get("depends_on") != ["03.07", "03.10"]:
        fail("03.12 frozen dependency contract drifted")

    evidence = task.get("evidence")
    if not isinstance(evidence, dict):
        fail("accepted 03.12 evidence object missing")
    for key, expected in ACCEPTED_EVIDENCE.items():
        if evidence.get(key) != expected:
            fail(
                f"accepted 03.12 evidence drifted: {key}; "
                f"expected={expected!r} actual={evidence.get(key)!r}"
            )

    if not is_ancestor(ACCEPTED_SOURCE):
        fail("accepted 03.12 evidence source is not an ancestor of HEAD")

    master = json.loads(text(STATUS_PATH))
    packages = {pkg["id"]: pkg for pkg in master.get("packages", [])}
    pkg03 = packages.get("PKG-03", {})
    if master.get("product_version") != "0.38.1":
        fail("master product version drifted")
    if (
        pkg03.get("required") != 25
        or pkg03.get("done") != done
        or pkg03.get("percent") != percent
    ):
        fail("master PKG-03 projection disagrees with canonical tracker")
    if master.get("active_task") != tracker.get("active_task"):
        fail("master/tracker active-task cursor disagreement")


def main() -> None:
    for ancestor in (CANONICAL_BASE, ACCEPTED_SOURCE):
        if not is_ancestor(ancestor):
            fail(f"required 03.12 authority/evidence head is not an ancestor: {ancestor}")

    validate_frozen_task_surfaces()
    validate_security_contract()
    validate_current_product_contract()
    validate_certification_surfaces()
    validate_canonical_descendant()

    tracker = json.loads(text(TRACKER_PATH))
    print(
        json.dumps(
            {
                "valid": True,
                "task": TASK,
                "mode": "accepted-descendant",
                "accepted_source": ACCEPTED_SOURCE,
                "accepted_evidence_preserved": True,
                "security_blob": blob_sha(SECURITY_PATH),
                "done": tracker.get("done"),
                "required": tracker.get("required"),
                "percent": tracker.get("percent"),
                "active_task": tracker.get("active_task"),
                "ready_tasks": tracker.get("ready_tasks"),
            },
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
