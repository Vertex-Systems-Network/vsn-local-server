#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PARENT_PLAN = ROOT / ".ai/plans/pkg03-windows-installer-v1.md"
TASK_PLAN = ROOT / ".ai/plans/pkg03-0301-architecture-contract-v1.md"
MANIFEST = ROOT / ".ai/manifests/pkg03-0301-architecture-contract.v1.json"
ARCH = ROOT / "docs/PKG03-WINDOWS-INSTALLER-ARCHITECTURE-V1.md"
TAURI = ROOT / "apps/desktop/src-tauri/tauri.conf.json"
TRACKER = ROOT / "certification/pkg03-windows-installer-v1.json"
STATUS = ROOT / "docs/MASTER-EXECUTION-STATUS.json"
EXPECTED_PARENT_SHA = "9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e"


def fail(message: str) -> None:
    raise SystemExit(f"PKG-03 03.01 validation failed: {message}")


def git_blob_sha256(path: Path) -> str:
    relative = path.relative_to(ROOT).as_posix()
    result = subprocess.run(
        ["git", "show", f"HEAD:{relative}"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        fail(f"unable to read repository blob for {relative}: {result.stderr.decode(errors='replace')}")
    return hashlib.sha256(result.stdout).hexdigest()


def main() -> None:
    if git_blob_sha256(PARENT_PLAN) != EXPECTED_PARENT_SHA:
        fail("parent package plan digest drifted")

    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    tauri = json.loads(TAURI.read_text(encoding="utf-8"))
    tracker = json.loads(TRACKER.read_text(encoding="utf-8"))
    status = json.loads(STATUS.read_text(encoding="utf-8"))
    arch = ARCH.read_text(encoding="utf-8")
    task_plan = TASK_PLAN.read_text(encoding="utf-8")

    if manifest.get("task_id") != "03.01" or manifest.get("linear_issue") != "ABD-76":
        fail("task authority identity mismatch")
    if manifest.get("canonical_base_sha") != "4606579e07ae57785d1bc1dc12073ea1d036ab4d":
        fail("canonical base mismatch")
    if manifest.get("market_delta", {}).get("change_required") is not False:
        fail("material market delta is unresolved")
    if manifest.get("authority", {}).get("product_installer_implementation_allowed") is not False:
        fail("03.01 must not authorize downstream installer implementation")

    expected_formats = ["nsis", "msi_wix"]
    if manifest.get("acceptance", {}).get("package_formats") != expected_formats:
        fail("package format contract changed")

    if tauri.get("productName") != "VSN Dev Platform":
        fail("unexpected productName")
    if tauri.get("version") != "0.38.1":
        fail("unexpected Tauri version")
    if tauri.get("identifier") != "dev.vsn.platform":
        fail("unexpected Tauri identifier")
    if tauri.get("bundle", {}).get("active") is not True:
        fail("Tauri bundle must remain active")

    required_arch = [
        "NSIS setup executable",
        "MSI produced through Tauri's Windows MSI/WiX path",
        "apps/desktop/src-tauri/tauri.conf.json",
        "03.03",
        "03.04",
        "03.05",
        "must not silently modify",
        "PKG-04 owns updater/apply/rollback orchestration",
    ]
    for token in required_arch:
        if token not in arch:
            fail(f"architecture contract missing: {token}")

    if "03.02" not in task_plan or "03.05" not in task_plan:
        fail("task plan does not preserve downstream ownership")

    tasks = {task["id"]: task for task in tracker.get("tasks", [])}
    if len(tasks) != 25 or list(tasks) != [f"03.{i:02d}" for i in range(1, 26)]:
        fail("PKG-03 task denominator/order drifted")

    dormant = (
        tracker.get("done") == 0
        and tasks["03.01"].get("status") == "BLOCKED"
        and tracker.get("active_task") is None
        and tracker.get("active_tasks") == []
        and tracker.get("ready_tasks") == []
        and status.get("active_package") == "PKG-02"
        and status.get("active_task") is None
    )

    accepted = (
        tracker.get("done") == 1
        and float(tracker.get("percent")) == 4.0
        and tracker.get("complete") is False
        and tasks["03.01"].get("status") == "DONE"
        and all(tasks[x].get("status") == "READY" for x in ["03.02", "03.03", "03.04", "03.05"])
        and all(tasks[f"03.{i:02d}"].get("status") == "BLOCKED" for i in range(6, 26))
        and tracker.get("active_tasks") == []
        and tracker.get("ready_tasks") == ["03.02", "03.03", "03.04", "03.05"]
        and tracker.get("active_task") == "03.02"
        and status.get("active_package") == "PKG-03"
        and status.get("active_task") == "03.02"
    )

    if not (dormant or accepted):
        fail("state is neither valid pre-evidence dormant phase nor accepted 03.01 exit state")

    phase = "accepted" if accepted else "pre_evidence"
    print(json.dumps({
        "task": "03.01",
        "phase": phase,
        "package_formats": expected_formats,
        "identity": {
            "productName": tauri["productName"],
            "version": tauri["version"],
            "identifier": tauri["identifier"],
        },
        "parent_plan_sha256": EXPECTED_PARENT_SHA,
        "task_plan_sha256": git_blob_sha256(TASK_PLAN),
        "architecture_sha256": git_blob_sha256(ARCH),
        "manifest_sha256": git_blob_sha256(MANIFEST),
        "valid": True,
    }, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
