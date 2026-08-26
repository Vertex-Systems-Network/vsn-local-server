#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / ".ai/manifests/pkg03-0304-install-scope-elevation.v1.json"
TRACKER = ROOT / "certification/pkg03-windows-installer-v1.json"
STATUS = ROOT / "docs/MASTER-EXECUTION-STATUS.json"
TAURI = ROOT / "apps/desktop/src-tauri/tauri.conf.json"
MACHINE = ROOT / "apps/desktop/src-tauri/tauri.per-machine.conf.json"

EXPECTED_BASE = "8f2919923005ba29b1475bd646a3f6953100ca9e"
EXPECTED_PARENT_SHA = "9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e"
EXPECTED_LOCK_SHA = "b2f41ab8c7a116cb9c78d41fd8036e7e1b1307bc3b78cd9a33ef37d5911c0aa6"
EXPECTED_PRODUCT = "VSN Dev Platform"
EXPECTED_VERSION = "0.38.1"
EXPECTED_IDENTIFIER = "dev.vsn.platform"
EXPECTED_PUBLISHER = "Vertex Systems Network"
EXPECTED_UPGRADE_CODE = "157f304f-1d1b-55e0-b89c-0610ea27c645"


def fail(message: str) -> None:
    raise SystemExit(f"PKG-03 03.04 validation failed: {message}")


def git_bytes(path: str) -> bytes:
    proc = subprocess.run(
        ["git", "show", f"HEAD:{path}"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if proc.returncode:
        fail(f"unable to read git blob {path}: {proc.stderr.decode(errors='replace')}")
    return proc.stdout


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def main() -> None:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    tracker = json.loads(TRACKER.read_text(encoding="utf-8"))
    status = json.loads(STATUS.read_text(encoding="utf-8"))
    tauri = json.loads(TAURI.read_text(encoding="utf-8"))
    machine = json.loads(MACHINE.read_text(encoding="utf-8"))

    if manifest.get("task_id") != "03.04" or manifest.get("linear_issue") != "ABD-79":
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
        ("install scope contract", manifest["install_scope_contract"]),
    ]
    for label, obj in digest_objects:
        path = obj.get("path") or obj.get("artifact")
        if not path or sha256(git_bytes(path)) != obj.get("sha256"):
            fail(f"{label} digest mismatch")
    if manifest.get("research", {}).get("change_required") is not False:
        fail("material research delta unresolved")

    authority = manifest.get("authority", {})
    expected_fields = [
        "bundle.windows.nsis.installMode",
        "apps/desktop/src-tauri/tauri.per-machine.conf.json:bundle.windows.nsis.installMode",
    ]
    if authority.get("allowed_product_fields") != expected_fields:
        fail("allowed product mutation set changed")
    for key in [
        "custom_nsis_template_allowed",
        "custom_wix_template_allowed",
        "delegated_scope_may_expand",
        "installer_execution_allowed",
        "payload_ownership_mutation_allowed",
        "privileged_mutation_allowed",
        "service_registration_allowed",
        "signing_secret_access_allowed",
        "updater_mutation_allowed",
    ]:
        if authority.get(key) is not False:
            fail(f"authority widened: {key}")

    locked = manifest.get("locked_inputs", {})
    if locked.get("node") != "22.12.0" or locked.get("rust") != "1.97.1":
        fail("toolchain lock changed")
    if locked.get("desktop_package_lock_sha256") != EXPECTED_LOCK_SHA:
        fail("manifest Desktop lock digest changed")
    if sha256(git_bytes("apps/desktop/package-lock.json")) != EXPECTED_LOCK_SHA:
        fail("Desktop package-lock authority drifted")
    if b'channel = "1.97.1"' not in git_bytes("rust-toolchain.toml"):
        fail("Rust toolchain pin drifted")

    bundle = tauri.get("bundle", {})
    windows = bundle.get("windows", {})
    wix = windows.get("wix", {})
    nsis = windows.get("nsis", {})
    if tauri.get("productName") != EXPECTED_PRODUCT or tauri.get("version") != EXPECTED_VERSION or tauri.get("identifier") != EXPECTED_IDENTIFIER:
        fail("application identity drifted")
    if bundle.get("publisher") != EXPECTED_PUBLISHER:
        fail("publisher drifted")
    if windows.get("allowDowngrades") is not False or str(wix.get("upgradeCode", "")).lower() != EXPECTED_UPGRADE_CODE:
        fail("accepted Windows package metadata drifted")
    if nsis.get("installMode") != "currentUser":
        fail("default NSIS install mode must be currentUser")
    if "template" in nsis or "template" in wix:
        fail("custom installer template is outside 03.04 authority")

    expected_overlay = {
        "$schema": "https://schema.tauri.app/config/2",
        "bundle": {"windows": {"nsis": {"installMode": "perMachine"}}},
    }
    if machine != expected_overlay:
        fail("per-machine overlay widened or changed")

    modes = [nsis.get("installMode"), machine["bundle"]["windows"]["nsis"]["installMode"]]
    if modes != ["currentUser", "perMachine"] or "both" in modes:
        fail("install-scope contract mismatch")

    expected_scope = {
        "default_nsis_install_mode": "currentUser",
        "machine_nsis_install_mode": "perMachine",
        "forbidden_nsis_install_mode": "both",
        "msi_install_scope": "perMachine",
        "current_user_registry_scope": "HKCU",
        "per_machine_registry_scope": "HKLM",
        "current_user_elevation_required": False,
        "per_machine_elevation_required": True,
    }
    acceptance = manifest.get("acceptance", {})
    if acceptance.get("runner") != "windows-2025" or acceptance.get("scope_contract") != expected_scope:
        fail("frozen acceptance scope/runner changed")

    tasks = {item["id"]: item for item in tracker.get("tasks", [])}
    if list(tasks) != [f"03.{i:02d}" for i in range(1, 26)] or tracker.get("required") != 25:
        fail("PKG-03 denominator/order drifted")
    if any(tasks[f"03.{i:02d}"].get("status") != "DONE" for i in range(1, 4)):
        fail("03.01-03.03 canonical prerequisites are not DONE")
    if tasks["03.04"].get("depends_on") != ["03.01"]:
        fail("03.04 dependency drifted")
    if tasks["03.05"].get("status") != "READY" or tasks["03.06"].get("status") != "BLOCKED":
        fail("03.05/03.06 boundary drifted")

    state_0304 = tasks["03.04"].get("status")
    if state_0304 == "READY":
        expected_done, expected_ready, expected_cursor, phase = 3, ["03.04", "03.05"], "03.04", "pre_evidence"
    elif state_0304 == "DONE":
        expected_done, expected_ready, expected_cursor, phase = 4, ["03.05"], "03.05", "accepted"
        evidence = tasks["03.04"].get("evidence")
        if not isinstance(evidence, dict) or not evidence.get("source_commit") or not evidence.get("workflow_run") or not evidence.get("artifact"):
            fail("accepted 03.04 state lacks exact evidence binding")
    else:
        fail(f"unexpected 03.04 state: {state_0304}")

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
        "task": "03.04",
        "phase": phase,
        "done": expected_done,
        "cursor": expected_cursor,
        "default_nsis_install_mode": "currentUser",
        "machine_nsis_install_mode": "perMachine",
        "msi_install_scope": "perMachine",
        "current_user_elevation_required": False,
        "per_machine_elevation_required": True,
        "valid": True,
    }, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
