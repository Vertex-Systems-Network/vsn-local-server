#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / ".ai/manifests/pkg03-0302-windows-bundle-build.v1.json"
TRACKER = ROOT / "certification/pkg03-windows-installer-v1.json"
STATUS = ROOT / "docs/MASTER-EXECUTION-STATUS.json"
TAURI = ROOT / "apps/desktop/src-tauri/tauri.conf.json"
EXPECTED_PARENT_SHA = "9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e"
EXPECTED_BASE = "9d33682f7c0cc30080792493c8f760f3fd120759"
EXPECTED_LOCK_SHA = "b2f41ab8c7a116cb9c78d41fd8036e7e1b1307bc3b78cd9a33ef37d5911c0aa6"
EXPECTED_BUILD_COMMAND = '.\\node_modules\\.bin\\tauri.cmd build --bundles "nsis,msi"'
EXPECTED_WINDOWS_ICON = "icons/icon.ico"
EXPECTED_EOL_POLICY = {
    "apps/desktop/src-tauri/Cargo.toml text eol=lf",
    "apps/desktop/src-tauri/gen/schemas/desktop-schema.json text eol=lf",
}


def fail(message: str) -> None:
    raise SystemExit(f"PKG-03 03.02 validation failed: {message}")


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


def digest_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def main() -> None:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    tracker = json.loads(TRACKER.read_text(encoding="utf-8"))
    status = json.loads(STATUS.read_text(encoding="utf-8"))
    tauri = json.loads(TAURI.read_text(encoding="utf-8"))

    if manifest.get("task_id") != "03.02" or manifest.get("linear_issue") != "ABD-77":
        fail("task authority identity mismatch")
    if manifest.get("canonical_base_sha") != EXPECTED_BASE:
        fail("canonical base mismatch")
    if manifest.get("parent_plan", {}).get("sha256") != EXPECTED_PARENT_SHA:
        fail("manifest parent plan digest mismatch")
    if digest_bytes(git_bytes(".ai/plans/pkg03-windows-installer-v1.md")) != EXPECTED_PARENT_SHA:
        fail("parent package plan digest drifted")

    for key, item in [
        ("task plan", manifest["task_plan"]),
        ("research", manifest["research"]),
        ("lifecycle", manifest["lifecycle"]),
        ("development preflight", manifest["development_preflight"]),
    ]:
        artifact = item["path"] if key == "task plan" else item["artifact"]
        if digest_bytes(git_bytes(artifact)) != item["sha256"]:
            fail(f"{key} digest mismatch")

    if manifest.get("research", {}).get("change_required") is not False:
        fail("material market delta is unresolved")
    acceptance = manifest.get("acceptance", {})
    if acceptance.get("runner") != "windows-2025" or acceptance.get("architecture") != "x64":
        fail("runner boundary changed")
    if acceptance.get("build_command") != EXPECTED_BUILD_COMMAND:
        fail("build command changed")
    if acceptance.get("required_outputs") != ["nsis-setup-exe", "msi"]:
        fail("required output contract changed")
    authority = manifest.get("authority", {})
    if authority.get("bundle_framework") != "tauri-v2":
        fail("unexpected bundle framework")
    if authority.get("package_formats") != ["nsis", "msi"]:
        fail("package format contract changed")
    for key in [
        "product_identity_mutation_allowed",
        "installer_execution_allowed",
        "privileged_mutation_allowed",
        "signing_secret_access_allowed",
        "delegated_scope_may_expand",
    ]:
        if authority.get(key) is not False:
            fail(f"authority widened: {key}")

    locked = manifest.get("locked_inputs", {})
    if locked.get("node") != "22.12.0" or locked.get("rust") != "1.97.1":
        fail("toolchain lock changed")
    if locked.get("desktop_package_lock_sha256") != EXPECTED_LOCK_SHA:
        fail("manifest Desktop lock digest mismatch")
    if digest_bytes(git_bytes("apps/desktop/package-lock.json")) != EXPECTED_LOCK_SHA:
        fail("Desktop package-lock digest drifted")
    if b'channel = "1.97.1"' not in git_bytes("rust-toolchain.toml"):
        fail("Rust toolchain pin drifted")

    attributes = {
        line.strip()
        for line in git_bytes(".gitattributes").decode("utf-8").splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    }
    if not EXPECTED_EOL_POLICY.issubset(attributes):
        fail("Tauri-managed Windows text files are not pinned to LF")

    if tauri.get("productName") != "VSN Dev Platform":
        fail("productName changed outside 03.03")
    if tauri.get("version") != "0.38.1":
        fail("Tauri version changed outside 03.03")
    if tauri.get("identifier") != "dev.vsn.platform":
        fail("identifier changed outside 03.03")
    bundle = tauri.get("bundle", {})
    if bundle.get("active") is not True or bundle.get("targets") != "all":
        fail("Tauri bundle boundary changed")
    if bundle.get("icon") != [EXPECTED_WINDOWS_ICON]:
        fail("Windows bundle icon binding changed")
    if not (ROOT / "apps/desktop/src-tauri/icons/icon.ico").is_file():
        fail("accepted Windows .ico resource is missing")

    tasks = {task["id"]: task for task in tracker.get("tasks", [])}
    if len(tasks) != 25 or list(tasks) != [f"03.{i:02d}" for i in range(1, 26)]:
        fail("PKG-03 task denominator/order drifted")
    if tasks["03.01"].get("status") != "DONE":
        fail("03.01 prerequisite is not DONE")
    if tasks["03.02"].get("depends_on") != ["03.01"]:
        fail("03.02 dependency changed")

    pre_evidence = (
        tracker.get("done") == 1
        and float(tracker.get("percent")) == 4.0
        and tracker.get("complete") is False
        and tasks["03.02"].get("status") == "READY"
        and tracker.get("active_tasks") == []
        and tracker.get("ready_tasks") == ["03.02", "03.03", "03.04", "03.05"]
        and tracker.get("active_task") == "03.02"
        and status.get("active_package") == "PKG-03"
        and status.get("active_task") == "03.02"
    )
    accepted = (
        tracker.get("done") == 2
        and float(tracker.get("percent")) == 8.0
        and tracker.get("complete") is False
        and tasks["03.02"].get("status") == "DONE"
        and all(tasks[x].get("status") == "READY" for x in ["03.03", "03.04", "03.05"])
        and tracker.get("active_tasks") == []
        and tracker.get("ready_tasks") == ["03.03", "03.04", "03.05"]
        and tracker.get("active_task") == "03.03"
        and status.get("active_package") == "PKG-03"
        and status.get("active_task") == "03.03"
    )
    if not (pre_evidence or accepted):
        fail("state is neither valid 03.02 pre-evidence nor accepted exit state")

    pkg = next((p for p in status.get("packages", []) if p.get("id") == "PKG-03"), None)
    if not pkg:
        fail("PKG-03 missing from master status")
    if pkg.get("done") != tracker.get("done") or pkg.get("required") != 25:
        fail("master/tracker count mismatch")

    phase = "accepted" if accepted else "pre_evidence"
    print(json.dumps({
        "task": "03.02",
        "phase": phase,
        "canonical_base_sha": EXPECTED_BASE,
        "parent_plan_sha256": EXPECTED_PARENT_SHA,
        "desktop_package_lock_sha256": EXPECTED_LOCK_SHA,
        "build_command": EXPECTED_BUILD_COMMAND,
        "windows_icon": EXPECTED_WINDOWS_ICON,
        "windows_eol_policy": sorted(EXPECTED_EOL_POLICY),
        "formats": ["nsis", "msi"],
        "valid": True,
    }, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
