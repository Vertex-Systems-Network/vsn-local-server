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

EXPECTED_PARENT_SHA = "9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e"
EXPECTED_BASE = "9d33682f7c0cc30080792493c8f760f3fd120759"
EXPECTED_LOCK_SHA = "b2f41ab8c7a116cb9c78d41fd8036e7e1b1307bc3b78cd9a33ef37d5911c0aa6"
EXPECTED_PRODUCT = "VSN Dev Platform"
EXPECTED_VERSION = "0.38.1"
EXPECTED_IDENTIFIER = "dev.vsn.platform"
EXPECTED_PUBLISHER = "Vertex Systems Network"
EXPECTED_UPGRADE_CODE = "157f304f-1d1b-55e0-b89c-0610ea27c645"


def fail(message: str) -> None:
    raise SystemExit(f"PKG-03 03.03 validation failed: {message}")


def git_bytes(path: str) -> bytes:
    p = subprocess.run(
        ["git", "show", f"HEAD:{path}"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if p.returncode != 0:
        fail(f"unable to read git blob {path}: {p.stderr.decode(errors='replace')}")
    return p.stdout


def digest(data: bytes) -> str:
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
    if digest(git_bytes(".ai/plans/pkg03-windows-installer-v1.md")) != EXPECTED_PARENT_SHA:
        fail("parent package plan bytes drifted")

    for key, obj in [
        ("task plan", manifest["task_plan"]),
        ("research", manifest["research"]),
        ("lifecycle", manifest["lifecycle"]),
        ("development preflight", manifest["development_preflight"]),
        ("identity contract", manifest["identity_contract"]),
    ]:
        path = obj.get("path") or obj.get("artifact")
        if digest(git_bytes(path)) != obj.get("sha256"):
            fail(f"{key} digest mismatch")

    if manifest.get("research", {}).get("change_required") is not False:
        fail("material research delta unresolved")

    authority = manifest.get("authority", {})
    if authority.get("identity_source") != "apps/desktop/src-tauri/tauri.conf.json":
        fail("identity source changed")
    if authority.get("allowed_product_fields") != [
        "bundle.publisher",
        "bundle.windows.allowDowngrades",
        "bundle.windows.wix.upgradeCode",
    ]:
        fail("allowed identity mutation set changed")
    for key in [
        "install_scope_mutation_allowed",
        "payload_ownership_mutation_allowed",
        "installer_execution_allowed",
        "privileged_mutation_allowed",
        "signing_secret_access_allowed",
        "updater_mutation_allowed",
        "delegated_scope_may_expand",
    ]:
        if authority.get(key) is not False:
            fail(f"authority widened: {key}")

    locked = manifest.get("locked_inputs", {})
    if locked.get("node") != "22.12.0" or locked.get("rust") != "1.97.1":
        fail("toolchain lock changed")
    if locked.get("desktop_package_lock_sha256") != EXPECTED_LOCK_SHA:
        fail("Desktop lock authority changed")
    if digest(git_bytes("apps/desktop/package-lock.json")) != EXPECTED_LOCK_SHA:
        fail("Desktop package-lock digest drifted")
    if b'channel = "1.97.1"' not in git_bytes("rust-toolchain.toml"):
        fail("Rust toolchain pin drifted")

    acceptance = manifest.get("acceptance", {})
    identity = acceptance.get("identity", {})
    expected = {
        "product_name": EXPECTED_PRODUCT,
        "product_version": EXPECTED_VERSION,
        "identifier": EXPECTED_IDENTIFIER,
        "publisher": EXPECTED_PUBLISHER,
        "wix_upgrade_code": EXPECTED_UPGRADE_CODE,
        "allow_downgrades": False,
    }
    if identity != expected:
        fail("frozen identity acceptance contract changed")
    if acceptance.get("runner") != "windows-2025":
        fail("runner boundary changed")

    if tauri.get("productName") != EXPECTED_PRODUCT:
        fail("productName drifted")
    if tauri.get("version") != EXPECTED_VERSION:
        fail("product version drifted")
    if tauri.get("identifier") != EXPECTED_IDENTIFIER:
        fail("application identifier drifted")
    bundle = tauri.get("bundle", {})
    if bundle.get("publisher") != EXPECTED_PUBLISHER:
        fail("publisher is not frozen")
    windows = bundle.get("windows", {})
    if windows.get("allowDowngrades") is not False:
        fail("downgrades are not blocked")
    wix = windows.get("wix", {})
    if str(wix.get("upgradeCode", "")).lower() != EXPECTED_UPGRADE_CODE:
        fail("WiX UpgradeCode drifted")

    derived = str(uuid.uuid5(uuid.NAMESPACE_DNS, f"{EXPECTED_PRODUCT}.exe.app.x64"))
    if derived != EXPECTED_UPGRADE_CODE:
        fail("pinned UpgradeCode no longer matches frozen deterministic baseline")

    tasks = {x["id"]: x for x in tracker.get("tasks", [])}
    if list(tasks) != [f"03.{i:02d}" for i in range(1, 26)]:
        fail("PKG-03 denominator/order drifted")
    if tasks["03.01"].get("status") != "DONE":
        fail("03.01 prerequisite is not DONE")
    if tasks["03.03"].get("depends_on") != ["03.01"]:
        fail("03.03 dependency changed")
    if tasks["03.02"].get("status") not in {"READY", "DONE"}:
        fail("unexpected sibling 03.02 state")
    if tasks["03.03"].get("status") not in {"READY", "IN_PROGRESS", "DONE"}:
        fail("03.03 is not actionable/accepted")
    if tasks["03.04"].get("status") != "READY" or tasks["03.05"].get("status") != "READY":
        fail("03.04/03.05 sibling readiness drifted")
    if tasks["03.06"].get("status") != "BLOCKED":
        fail("03.06 advanced before full Wave 1 completion")

    done_count = sum(x.get("status") == "DONE" for x in tasks.values())
    if tracker.get("done") != done_count:
        fail("tracker DONE count mismatch")
    if float(tracker.get("percent")) != round(done_count * 100.0 / 25, 2):
        fail("tracker percent mismatch")

    actionable = sorted(
        x["id"] for x in tasks.values() if x.get("status") in {"READY", "IN_PROGRESS"}
    )
    cursor = actionable[0] if actionable else None
    if tracker.get("active_task") != cursor:
        fail("tracker deterministic cursor mismatch")

    pkg = next((x for x in status.get("packages", []) if x.get("id") == "PKG-03"), None)
    if not pkg:
        fail("PKG-03 missing from master status")
    if pkg.get("done") != tracker.get("done") or pkg.get("required") != 25:
        fail("master/tracker count mismatch")
    if status.get("active_package") != "PKG-03" or status.get("active_task") != cursor:
        fail("master active cursor mismatch")

    phase = "accepted" if tasks["03.03"].get("status") == "DONE" else "pre_evidence"
    print(json.dumps({
        "task": "03.03",
        "phase": phase,
        "publisher": EXPECTED_PUBLISHER,
        "product_name": EXPECTED_PRODUCT,
        "product_version": EXPECTED_VERSION,
        "identifier": EXPECTED_IDENTIFIER,
        "wix_upgrade_code": EXPECTED_UPGRADE_CODE,
        "allow_downgrades": False,
        "sibling_03_02": tasks["03.02"].get("status"),
        "done": done_count,
        "cursor": cursor,
        "valid": True,
    }, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
