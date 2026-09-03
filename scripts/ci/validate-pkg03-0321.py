#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MANIFEST_PATH = ROOT / ".ai/manifests/pkg03-0321-silent-deployment.v1.json"
TRACKER_PATH = ROOT / "certification/pkg03-windows-installer-v1.json"
BASE = "3edb4e1dcd2c062e7b2e270cde626c90a2c5459f"

def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

def fail(message: str) -> None:
    raise SystemExit(message)

manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
if manifest.get("task_id") != "03.21" or manifest.get("linear_issue") != "ABD-96":
    fail("03.21 manifest identity mismatch")
if manifest.get("status") != "frozen" or manifest.get("canonical_base_sha") != BASE:
    fail("03.21 manifest base/status mismatch")
if manifest.get("dependencies") != ["03.16", "03.17", "03.20"] or manifest.get("lane") != "automation":
    fail("03.21 dependency/lane contract mismatch")

parent = manifest["parent_plan"]
parent_path = ROOT / parent["path"]
if sha256(parent_path) != parent["sha256"] or parent["sha256"] != "9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e":
    fail("Frozen PKG-03 parent plan digest mismatch")

for section, key in [
    ("research", "artifact"),
    ("lifecycle", "artifact"),
    ("development_preflight", "artifact"),
    ("task_plan", "path"),
    ("lifecycle_contract", "artifact"),
]:
    record = manifest[section]
    path = ROOT / record[key]
    if not path.is_file():
        fail(f"Missing 03.21 {section} artifact: {path}")
    if sha256(path) != record["sha256"]:
        fail(f"03.21 {section} digest mismatch")

if manifest["research"].get("change_required") is not False:
    fail("03.21 research requires change control before mutation")

acceptance = manifest.get("acceptance", {})
required_true = [
    "exact_three_package_builds_required",
    "nsis_uppercase_s_required",
    "msi_quiet_required",
    "msi_norestart_required",
    "zero_ui_input_required",
    "no_visible_installer_family_titled_window_required",
    "bounded_completion_required",
    "current_user_service_must_remain_absent",
    "machine_service_contract_required",
    "msi_really_suppress_evidence_required",
    "tracked_repository_drift_zero_required",
]
for key in required_true:
    if acceptance.get(key) is not True:
        fail(f"03.21 acceptance flag false/missing: {key}")
if acceptance.get("nsis_success_codes") != [0]:
    fail("03.21 NSIS exit contract widened")
if acceptance.get("msi_success_codes") != [0, 3010]:
    fail("03.21 MSI success contract mismatch")
if acceptance.get("msi_reboot_initiated_code_forbidden") != 1641:
    fail("03.21 MSI reboot-initiated code mismatch")

authority = manifest.get("authority", {})
for key, value in authority.items():
    if value is not False:
        fail(f"03.21 initial authority widened: {key}={value!r}")

tracker = json.loads(TRACKER_PATH.read_text(encoding="utf-8"))
if tracker.get("package_id") != "PKG-03" or tracker.get("done") != 20 or tracker.get("required") != 25:
    fail("Canonical PKG-03 tracker denominator/progress mismatch")
if tracker.get("active_task") != "03.21":
    fail("Canonical PKG-03 cursor is not 03.21")
if tracker.get("active_tasks") != []:
    fail("Canonical tracker unexpectedly projects an active implementation before 03.21 evidence")
if tracker.get("ready_tasks") != ["03.21", "03.22"]:
    fail("Canonical PKG-03 READY set mismatch")

tasks = {row["id"]: row for row in tracker.get("tasks", [])}
for dep in ("03.16", "03.17", "03.20"):
    if tasks.get(dep, {}).get("status") != "DONE":
        fail(f"03.21 dependency {dep} is not canonically DONE")
if tasks.get("03.21", {}).get("status") != "READY":
    fail("03.21 is not canonically READY")
if tasks.get("03.22", {}).get("status") != "READY":
    fail("Independent 03.22 READY lane was unexpectedly changed")

allowed = {
    ".ai/features/pkg03-0321/development-preflight.md",
    ".ai/features/pkg03-0321/lifecycle-review.md",
    ".ai/features/pkg03-0321/research.md",
    ".ai/manifests/pkg03-0321-silent-deployment.v1.json",
    ".ai/plans/pkg03-0321-silent-deployment-v1.md",
    ".github/workflows/pkg03-0321-silent-deployment.yml",
    "docs/PKG03-SILENT-DEPLOYMENT-V1.md",
    "scripts/ci/pkg03-0321-silent-deployment.ps1",
    "scripts/ci/validate-pkg03-0321.py",
    "certification/pkg03-windows-installer-v1.json",
    "docs/MASTER-EXECUTION-STATUS.json",
    "docs/MASTER-EXECUTION-PLAN.md",
    "README.md",
    ".ai/README.md",
}
try:
    changed = subprocess.check_output(
        ["git", "diff", "--name-only", f"{BASE}...HEAD"],
        cwd=ROOT,
        text=True,
    ).splitlines()
except subprocess.CalledProcessError as exc:
    fail(f"Unable to compute 03.21 changed-file scope: {exc}")
unexpected = sorted(set(changed) - allowed)
if unexpected:
    fail("03.21 changed-file scope escaped frozen authority: " + ", ".join(unexpected))

plan_text = (ROOT / manifest["task_plan"]["path"]).read_text(encoding="utf-8")
for token in ("/S", "/quiet", "/qn", "/norestart", "3010", "1641", "03.19", "03.20"):
    if token not in plan_text:
        fail(f"03.21 plan missing acceptance token: {token}")
if "/passive" not in plan_text or "not strict silent" not in plan_text:
    fail("03.21 passive-mode nonclaim missing")

print(json.dumps({
    "valid": True,
    "task": "03.21",
    "canonical_base": BASE,
    "changed_files": changed,
    "parent_plan_sha256": parent["sha256"],
}, indent=2))
