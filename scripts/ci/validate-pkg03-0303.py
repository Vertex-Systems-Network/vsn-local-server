#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / ".ai/manifests/pkg03-0303-package-identity.v1.json"
TRACKER = ROOT / "certification/pkg03-windows-installer-v1.json"
STATUS = ROOT / "docs/MASTER-EXECUTION-STATUS.json"
TAURI = ROOT / "apps/desktop/src-tauri/tauri.conf.json"

EXPECTED_BASE = "d1d3e6997878aa16b8d4ad05f094754b5b1699b2"
EXPECTED_PARENT_SHA = "9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e"
EXPECTED_LOCK_SHA = "b2f41ab8c7a116cb9c78d41fd8036e7e1b1307bc3b78cd9a33ef37d5911c0aa6"
EXPECTED_PRODUCT = "VSN Dev Platform"
EXPECTED_VERSION = "0.38.1"
EXPECTED_IDENTIFIER = "dev.vsn.platform"
EXPECTED_PUBLISHER = "Vertex Systems Network"
EXPECTED_UPGRADE_CODE = "157f304f-1d1b-55e0-b89c-0610ea27c645"


def fail(message: str) -> None:
    raise SystemExit(f"PKG-03 03.03 validation failed: {message}")


def git_bytes(path: str) -> bytes:
    proc = subprocess.run(["git", "show", f"HEAD:{path}"], cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
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

    if manifest.get("task_id") != "03.03" or manifest.get("linear_issue") != "ABD-78":
        fail("task identity mismatch")
    if manifest.get("canonical_base_sha") != EXPECTED_BASE:
        fail("canonical base mismatch")
    if manifest.get("parent_plan", {}).get("sha256") != EXPECTED_PARENT_SHA:
        fail("parent plan digest mismatch")
    if sha256(git_bytes(".ai/plans/pkg03-windows-installer-v1.md")) != EXPECTED_PARENT_SHA:
        fail("parent package plan bytes drifted")

    for label, obj in [
        ("task plan", manifest["task_plan"]),
        ("research", manifest["research"]),
        ("lifecycle", manifest["lifecycle"]),
        ("development preflight", manifest["development_preflight"]),
        ("identity contract", manifest["identity_contract"]),
    ]:
        path = obj.get("path") or obj.get("artifact")
        if sha256(git_bytes(path)) != obj.get("sha256"):
            fail(f"{label} digest mismatch")
    if manifest.get("research", {}).get("change_required") is not False:
        fail("material research delta unresolved")

    authority = manifest.get("authority", {})
    if authority.get("identity_source") != "apps/desktop/src-tauri/tauri.conf.json":
        fail("identity source changed")
    if authority.get("allowed_product_fields") != ["bundle.publisher", "bundle.windows.allowDowngrades", "bundle.windows.wix.upgradeCode"]:
        fail("allowed product mutation set changed")
    for key in ["install_scope_mutation_allowed", "payload_ownership_mutation_allowed", "installer_execution_allowed", "privileged_mutation_allowed", "signing_secret_access_allowed", "updater_mutation_allowed", "delegated_scope_may_expand"]:
        if authority.get(key) is not False:
            fail(f"authority widened: {key}")

    locked = manifest.get("locked_inputs", {})
    if locked.get("node") != "22.12.0" or locked.get("rust") != "1.97.1":
        fail("toolchain lock changed")
    if locked.get("desktop_package_lock_sha256") != EXPECTED_LOCK_SHA or sha256(git_bytes("apps/desktop/package-lock.json")) != EXPECTED_LOCK_SHA:
        fail("Desktop package-lock authority drifted")
    if b'channel = "1.97.1"' not in git_bytes("rust-toolchain.toml"):
        fail("Rust toolchain pin drifted")

    expected_identity = {"product_name": EXPECTED_PRODUCT, "product_version": EXPECTED_VERSION, "identifier": EXPECTED_IDENTIFIER, "publisher": EXPECTED_PUBLISHER, "wix_upgrade_code": EXPECTED_UPGRADE_CODE, "allow_downgrades": False}
    if manifest.get("acceptance", {}).get("identity") != expected_identity or manifest.get("acceptance", {}).get("runner") != "windows-2025":
        fail("frozen acceptance identity/runner changed")

    bundle = tauri.get("bundle", {})
    windows = bundle.get("windows", {})
    wix = windows.get("wix", {})
    if tauri.get("productName") != EXPECTED_PRODUCT or tauri.get("version") != EXPECTED_VERSION or tauri.get("identifier") != EXPECTED_IDENTIFIER:
        fail("application identity drifted")
    if bundle.get("publisher") != EXPECTED_PUBLISHER or windows.get("allowDowngrades") is not False or str(wix.get("upgradeCode", "")).lower() != EXPECTED_UPGRADE_CODE:
        fail("Windows package metadata drifted")
    if str(uuid.uuid5(uuid.NAMESPACE_DNS, f"{EXPECTED_PRODUCT}.exe.app.x64")) != EXPECTED_UPGRADE_CODE:
        fail("pinned UpgradeCode no longer matches deterministic baseline")

    tasks = {item["id"]: item for item in tracker.get("tasks", [])}
    if list(tasks) != [f"03.{i:02d}" for i in range(1, 26)] or tracker.get("required") != 25:
        fail("PKG-03 denominator/order drifted")
    if tasks["03.01"].get("status") != "DONE" or tasks["03.02"].get("status") != "DONE":
        fail("03.01/03.02 canonical prerequisites are not DONE")
    if tasks["03.03"].get("depends_on") != ["03.01"]:
        fail("03.03 dependency drifted")
    if tasks["03.04"].get("status") != "READY" or tasks["03.05"].get("status") != "READY" or tasks["03.06"].get("status") != "BLOCKED":
        fail("Wave 1/2 boundary drifted")

    state_0303 = tasks["03.03"].get("status")
    if state_0303 == "READY":
        expected_done, expected_ready, expected_cursor, phase = 2, ["03.03", "03.04", "03.05"], "03.03", "pre_evidence"
    elif state_0303 == "DONE":
        expected_done, expected_ready, expected_cursor, phase = 3, ["03.04", "03.05"], "03.04", "accepted"
    else:
        fail(f"unexpected 03.03 state: {state_0303}")

    if tracker.get("done") != expected_done or float(tracker.get("percent")) != round(expected_done * 100.0 / 25, 2):
        fail("tracker progress mismatch")
    if tracker.get("ready_tasks") != expected_ready or tracker.get("active_task") != expected_cursor or tracker.get("active_tasks") != []:
        fail("tracker ready/cursor projection mismatch")

    pkg = next((item for item in status.get("packages", []) if item.get("id") == "PKG-03"), None)
    if not pkg or pkg.get("done") != expected_done or pkg.get("required") != 25 or float(pkg.get("percent")) != round(expected_done * 100.0 / 25, 2):
        fail("master PKG-03 progress mismatch")
    if status.get("active_package") != "PKG-03" or status.get("active_task") != expected_cursor:
        fail("master active cursor mismatch")

    print(json.dumps({"task": "03.03", "phase": phase, "done": expected_done, "cursor": expected_cursor, "product_name": EXPECTED_PRODUCT, "product_version": EXPECTED_VERSION, "identifier": EXPECTED_IDENTIFIER, "publisher": EXPECTED_PUBLISHER, "wix_upgrade_code": EXPECTED_UPGRADE_CODE, "allow_downgrades": False, "valid": True}, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
