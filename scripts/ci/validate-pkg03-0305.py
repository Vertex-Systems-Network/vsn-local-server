#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import re
import subprocess
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / ".ai/manifests/pkg03-0305-owned-payload.v1.json"
OWNERSHIP = ROOT / "installer/windows/owned-payload.v1.json"
TRACKER = ROOT / "certification/pkg03-windows-installer-v1.json"
STATUS = ROOT / "docs/MASTER-EXECUTION-STATUS.json"
TAURI = ROOT / "apps/desktop/src-tauri/tauri.conf.json"
MACHINE = ROOT / "apps/desktop/src-tauri/tauri.per-machine.conf.json"

EXPECTED_BASE = "7cd671de8af410ee348083c42c716cce1dd22543"
EXPECTED_PARENT_SHA = "9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e"
EXPECTED_TAURI_SHA = "366291310a2353a0e6cfbaa2acb748ce87ab5f51efe077b531b8dc7b3449f9e7"
EXPECTED_MACHINE_SHA = "48fd4eb22ffe99a884ce5f4770de83e29ad919650d7c254b5d180fca3add7429"
EXPECTED_PRODUCT = "VSN Dev Platform"
EXPECTED_VERSION = "0.38.1"
EXPECTED_IDENTIFIER = "dev.vsn.platform"
EXPECTED_PUBLISHER = "Vertex Systems Network"
EXPECTED_UPGRADE_CODE = "157f304f-1d1b-55e0-b89c-0610ea27c645"
EXPECTED_OWNED_PATHS = ["VSN Dev Platform.exe", "bin/vsn.exe", "bin/vsn-agent.exe"]
EXPECTED_READY_AFTER = ["03.06", "03.07", "03.08", "03.09", "03.10"]

ALLOWED_CHANGED_PATHS = {
    ".ai/features/pkg03-0305/research.md",
    ".ai/features/pkg03-0305/lifecycle-review.md",
    ".ai/features/pkg03-0305/development-preflight.md",
    ".ai/plans/pkg03-0305-owned-payload-v1.md",
    ".ai/manifests/pkg03-0305-owned-payload.v1.json",
    "docs/PKG03-WINDOWS-OWNED-PAYLOAD-V1.md",
    "installer/windows/owned-payload.v1.json",
    "scripts/ci/validate-pkg03-0305.py",
    ".github/workflows/pkg03-0305-owned-payload.yml",
    "certification/pkg03-windows-installer-v1.json",
    "docs/MASTER-EXECUTION-STATUS.json",
}

RESERVED = {"CON", "PRN", "AUX", "NUL"}
RESERVED.update({f"COM{i}" for i in range(1, 10)})
RESERVED.update({f"LPT{i}" for i in range(1, 10)})


def fail(message: str) -> None:
    raise SystemExit(f"PKG-03 03.05 validation failed: {message}")


def git_bytes(path: str) -> bytes:
    proc = subprocess.run(
        ["git", "show", f"HEAD:{path}"], cwd=ROOT,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False,
    )
    if proc.returncode:
        fail(f"unable to read git blob {path}: {proc.stderr.decode(errors='replace')}")
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


def validate_relative_windows_path(path: str) -> tuple[bool, str]:
    if not isinstance(path, str) or not path:
        return False, "empty"
    if "\\" in path:
        return False, "non-canonical-separator"
    if path.startswith("/"):
        return False, "absolute"
    if re.match(r"^[A-Za-z]:", path):
        return False, "drive-qualified"
    if path.startswith("//"):
        return False, "unc-or-device"
    if ":" in path:
        return False, "ads"
    if any(ord(ch) < 32 or ord(ch) == 127 for ch in path):
        return False, "control-character"
    if "*" in path or "?" in path:
        return False, "wildcard"

    segments = path.split("/")
    if any(segment == "" for segment in segments):
        return False, "empty-segment"
    for segment in segments:
        if segment in {".", ".."}:
            return False, "dot-segment"
        if segment.endswith(" ") or segment.endswith("."):
            return False, "trailing-space-or-dot"
        base = segment.split(".", 1)[0].upper()
        if base in RESERVED:
            return False, "reserved-device-name"
    return True, "ok"


def main() -> None:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    ownership = json.loads(OWNERSHIP.read_text(encoding="utf-8"))
    tracker = json.loads(TRACKER.read_text(encoding="utf-8"))
    status = json.loads(STATUS.read_text(encoding="utf-8"))
    tauri = json.loads(TAURI.read_text(encoding="utf-8"))
    machine = json.loads(MACHINE.read_text(encoding="utf-8"))

    if manifest.get("task_id") != "03.05" or manifest.get("linear_issue") != "ABD-80":
        fail("task identity mismatch")
    if manifest.get("canonical_base_sha") != EXPECTED_BASE:
        fail("canonical base mismatch")
    if manifest.get("parent_plan", {}).get("sha256") != EXPECTED_PARENT_SHA:
        fail("parent plan digest mismatch")
    if sha256(git_bytes(".ai/plans/pkg03-windows-installer-v1.md")) != EXPECTED_PARENT_SHA:
        fail("parent package plan bytes drifted")

    digest_objects = [
        ("task plan", manifest["task_plan"]),
        ("research", manifest["research"]),
        ("lifecycle", manifest["lifecycle"]),
        ("development preflight", manifest["development_preflight"]),
        ("ownership contract", manifest["ownership_contract"]),
    ]
    for label, obj in digest_objects:
        path = obj.get("path") or obj.get("artifact")
        if not path or sha256(git_bytes(path)) != obj.get("sha256"):
            fail(f"{label} digest mismatch")
    if manifest.get("research", {}).get("change_required") is not False:
        fail("material research delta unresolved")

    authority = manifest.get("authority", {})
    if authority.get("allowed_product_files") != ["installer/windows/owned-payload.v1.json"]:
        fail("allowed product file set changed")
    for key in [
        "delegated_scope_may_expand",
        "installer_execution_allowed",
        "tauri_config_mutation_allowed",
        "custom_installer_template_allowed",
        "cli_agent_real_placement_allowed",
        "service_registration_allowed",
        "acl_mutation_allowed",
        "privileged_mutation_allowed",
        "signing_secret_access_allowed",
        "updater_mutation_allowed",
    ]:
        if authority.get(key) is not False:
            fail(f"authority widened: {key}")

    unexpected = changed_paths() - ALLOWED_CHANGED_PATHS
    if unexpected:
        fail(f"out-of-scope changed paths: {sorted(unexpected)}")

    if sha256(git_bytes("apps/desktop/src-tauri/tauri.conf.json")) != EXPECTED_TAURI_SHA:
        fail("accepted Tauri config bytes drifted")
    if sha256(git_bytes("apps/desktop/src-tauri/tauri.per-machine.conf.json")) != EXPECTED_MACHINE_SHA:
        fail("accepted per-machine overlay bytes drifted")

    bundle = tauri.get("bundle", {})
    windows = bundle.get("windows", {})
    if tauri.get("productName") != EXPECTED_PRODUCT or tauri.get("version") != EXPECTED_VERSION or tauri.get("identifier") != EXPECTED_IDENTIFIER:
        fail("application identity drifted")
    if bundle.get("publisher") != EXPECTED_PUBLISHER:
        fail("publisher drifted")
    if windows.get("allowDowngrades") is not False:
        fail("downgrade policy drifted")
    if str(windows.get("wix", {}).get("upgradeCode", "")).lower() != EXPECTED_UPGRADE_CODE:
        fail("WiX upgrade identity drifted")
    if windows.get("nsis", {}).get("installMode") != "currentUser":
        fail("default NSIS scope drifted")
    if machine.get("bundle", {}).get("windows", {}).get("nsis", {}).get("installMode") != "perMachine":
        fail("per-machine overlay scope drifted")
    if "externalBin" in bundle or "resources" in bundle:
        fail("03.05 must not realize CLI/Agent placement in Tauri configuration")

    expected_policy = {
        "canonical_separator": "/",
        "case_sensitive": False,
        "wildcards_allowed": False,
        "allow_absolute": False,
        "allow_drive_qualified": False,
        "allow_unc": False,
        "allow_device_paths": False,
        "allow_dot_segments": False,
        "allow_ads": False,
        "allow_control_characters": False,
        "allow_trailing_space_or_dot": False,
        "allow_reserved_device_names": False,
        "allow_empty_segments": False,
        "require_reparse_containment_downstream": True,
    }
    if ownership.get("schema_version") != 1 or ownership.get("package_id") != "PKG-03" or ownership.get("task_id") != "03.05" or ownership.get("version") != "1.0.0":
        fail("ownership manifest identity mismatch")
    if ownership.get("install_root_token") != "${INSTALL_ROOT}":
        fail("ownership root token changed")
    if ownership.get("path_policy") != expected_policy:
        fail("path policy changed")

    expected_entries = [
        {"id":"desktop","relative_path":"VSN Dev Platform.exe","source_package":"vsn-desktop","source_path":"apps/desktop/src-tauri","placement_owner":"03.02","placement_status":"already-bundled"},
        {"id":"cli","relative_path":"bin/vsn.exe","source_package":"vsn","source_path":"apps/cli","placement_owner":"03.10","placement_status":"declared-not-yet-packaged"},
        {"id":"agent","relative_path":"bin/vsn-agent.exe","source_package":"vsn-agent","source_path":"apps/agent","placement_owner":"03.10","placement_status":"declared-not-yet-packaged"},
    ]
    if ownership.get("owned_files") != expected_entries:
        fail("owned file set/order/metadata changed")
    owned_paths = [item["relative_path"] for item in ownership["owned_files"]]
    if owned_paths != EXPECTED_OWNED_PATHS:
        fail("owned path set changed")

    folded: set[str] = set()
    for path in owned_paths:
        valid, reason = validate_relative_windows_path(path)
        if not valid:
            fail(f"accepted owned path is invalid ({reason}): {path!r}")
        key = path.casefold()
        if key in folded:
            fail(f"case-insensitive owned-path collision: {path}")
        folded.add(key)

    bad_vectors = [
        "", "C:/evil.exe", "C:\\evil.exe", "/evil.exe", "//server/share/evil.exe",
        "\\\\?\\C:\\evil.exe", "\\\\.\\C:\\evil.exe", "../evil.exe",
        "bin/../evil.exe", "./bin/vsn.exe", "bin\\vsn.exe", "bin/vsn.exe:stream",
        "CON", "NUL.txt", "bin/COM1.exe", "bin/LPT9", "bin/vsn.exe.",
        "bin/vsn.exe ", "bin//vsn.exe", "bin/\x01bad.exe", "bin/*.exe",
    ]
    for value in bad_vectors:
        valid, _ = validate_relative_windows_path(value)
        if valid:
            fail(f"malicious path vector accepted: {value!r}")
    if "BIN/VSN.EXE".casefold() != "bin/vsn.exe".casefold():
        fail("case-insensitive collision model broken")

    excluded = ownership.get("excluded_classes", [])
    if not isinstance(excluded, list) or len(excluded) < 7:
        fail("excluded ownership classes incomplete")
    joined_excluded = " ".join(str(x).lower() for x in excluded)
    for required in ["updater", "projects", "configuration", "runtime state", "database", "credentials", "logs"]:
        if required not in joined_excluded:
            fail(f"missing excluded ownership class: {required}")
    if "updater-helper" in json.dumps(ownership.get("owned_files", [])).lower():
        fail("updater-helper must not be installer-owned by 03.05")

    cargo_expectations = {
        "apps/desktop/src-tauri/Cargo.toml": "vsn-desktop",
        "apps/cli/Cargo.toml": "vsn",
        "apps/agent/Cargo.toml": "vsn-agent",
    }
    for path, package_name in cargo_expectations.items():
        doc = tomllib.loads(git_bytes(path).decode("utf-8"))
        package = doc.get("package", {})
        if package.get("name") != package_name or package.get("version") != EXPECTED_VERSION:
            fail(f"Cargo package identity drifted: {path}")

    acceptance = manifest.get("acceptance", {})
    if acceptance.get("runner") != "windows-2025":
        fail("evidence runner changed")
    expected_contract = {
        "install_root_token": "${INSTALL_ROOT}",
        "owned_paths": EXPECTED_OWNED_PATHS,
        "path_case": "windows-case-insensitive",
        "canonical_separator": "/",
        "wildcards_allowed": False,
    }
    if acceptance.get("ownership_contract") != expected_contract:
        fail("frozen ownership acceptance contract changed")

    tasks = {item["id"]: item for item in tracker.get("tasks", [])}
    if list(tasks) != [f"03.{i:02d}" for i in range(1, 26)] or tracker.get("required") != 25:
        fail("PKG-03 denominator/order drifted")
    if any(tasks[f"03.{i:02d}"].get("status") != "DONE" for i in range(1, 5)):
        fail("03.01-03.04 canonical prerequisites are not DONE")
    if tasks["03.05"].get("depends_on") != ["03.01"]:
        fail("03.05 dependency drifted")

    state_0305 = tasks["03.05"].get("status")
    if state_0305 == "READY":
        expected_done = 4
        expected_ready = ["03.05"]
        expected_cursor = "03.05"
        phase = "pre_evidence"
        if any(tasks[f"03.{i:02d}"].get("status") != "BLOCKED" for i in range(6, 11)):
            fail("03.06-03.10 must remain BLOCKED before 03.05 acceptance")
    elif state_0305 == "DONE":
        expected_done = 5
        expected_ready = EXPECTED_READY_AFTER
        expected_cursor = "03.06"
        phase = "accepted"
        if any(tasks[task_id].get("status") != "READY" for task_id in EXPECTED_READY_AFTER):
            fail("03.06-03.10 must all become READY after 03.05 acceptance")
        evidence = tasks["03.05"].get("evidence")
        if not isinstance(evidence, dict) or not evidence.get("source_commit") or not evidence.get("workflow_run") or not evidence.get("artifact") or not evidence.get("evidence_sha256"):
            fail("accepted 03.05 state lacks exact evidence binding")
    else:
        fail(f"unexpected 03.05 state: {state_0305}")

    expected_percent = round(expected_done * 100.0 / 25, 2)
    if tracker.get("done") != expected_done or float(tracker.get("percent")) != expected_percent:
        fail("tracker progress mismatch")
    if tracker.get("ready_tasks") != expected_ready or tracker.get("active_task") != expected_cursor or tracker.get("active_tasks") != []:
        fail("tracker ready/cursor projection mismatch")
    if tracker.get("complete") is not False or tracker.get("status") != "IN_PROGRESS":
        fail("tracker lifecycle state drifted")

    pkg = next((item for item in status.get("packages", []) if item.get("id") == "PKG-03"), None)
    if not pkg or pkg.get("done") != expected_done or pkg.get("required") != 25 or float(pkg.get("percent")) != expected_percent:
        fail("master PKG-03 progress mismatch")
    if status.get("active_package") != "PKG-03" or status.get("active_task") != expected_cursor:
        fail("master active cursor mismatch")

    print(json.dumps({
        "task": "03.05",
        "phase": phase,
        "done": expected_done,
        "cursor": expected_cursor,
        "owned_paths": owned_paths,
        "negative_vectors_rejected": len(bad_vectors),
        "updater_helper_owned": False,
        "valid": True,
    }, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
