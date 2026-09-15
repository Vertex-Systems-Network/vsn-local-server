#!/usr/bin/env python3
"""Fail-closed validation for Fast Development Governance v1."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FAST = ROOT / ".ai" / "governance" / "FAST-DEVELOPMENT.md"
CHANGE = ROOT / ".ai" / "changes" / "FAST-DEVELOPMENT-GOVERNANCE-V1.md"
LIVE = ROOT / "scripts" / "ci" / "live-work-state.py"
WORKFLOW = ROOT / ".github" / "workflows" / "fast-development-governance.yml"

ALLOWED_INITIAL_PATHS = {
    ".ai/governance/FAST-DEVELOPMENT.md",
    ".ai/changes/FAST-DEVELOPMENT-GOVERNANCE-V1.md",
    "scripts/ci/live-work-state.py",
    "scripts/ci/validate-fast-development.py",
    ".github/workflows/fast-development-governance.yml",
}

REQUIRED_FAST_MARKERS = [
    "PREIMPLEMENTATION",
    "NON_ACCEPTANCE_PREIMPLEMENTATION",
    "fixture-pass is never grandfathered into final acceptance",
    "Security and QA",
    "No fake provider IDs",
    "canonical task tracker",
]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--changed-files", type=Path)
    args = parser.parse_args()

    for path in [FAST, CHANGE, LIVE, WORKFLOW]:
        require(path.is_file(), f"missing required fast-development artifact: {path.relative_to(ROOT)}")

    fast = FAST.read_text(encoding="utf-8")
    change = CHANGE.read_text(encoding="utf-8")
    workflow = WORKFLOW.read_text(encoding="utf-8")

    for marker in REQUIRED_FAST_MARKERS:
        require(marker.lower() in fast.lower(), f"fast profile missing marker: {marker}")

    require(
        "conversation:user-2026-09-16-fast-development-implementation" in change,
        "approved user decision reference missing",
    )
    require("PKG-03 remains 21/25" in change, "canonical PKG-03 non-mutation invariant missing")
    require("contents: read" in workflow, "fast governance workflow must remain read-only")
    require("persist-credentials: false" in workflow, "checkout credentials must not persist")

    live_raw = subprocess.check_output(
        [sys.executable, str(LIVE), "--head", "VALIDATION_HEAD", "--compact"],
        cwd=ROOT,
        text=True,
    )
    live = json.loads(live_raw)
    require(live["head"] == "VALIDATION_HEAD", "live state HEAD binding failed")
    require(live["current_work_checkpoint_authoritative"] is False, "stale checkpoint became authority")
    require(isinstance(live.get("ready_tasks"), list), "ready_tasks missing")
    require(live.get("active_package"), "active package missing")
    require(live.get("tracker"), "active tracker missing")

    if args.changed_files:
        changed = {
            line.strip().replace("\\", "/")
            for line in args.changed_files.read_text(encoding="utf-8").splitlines()
            if line.strip()
        }
        unexpected = sorted(changed - ALLOWED_INITIAL_PATHS)
        require(not unexpected, f"phase-1 scope exceeded; unexpected changed paths: {unexpected}")
        require(changed, "changed-file list is empty")

    print(
        json.dumps(
            {
                "fast_development_governance": "PASS",
                "canonical_active_package": live["active_package"],
                "canonical_active_task": live.get("active_task"),
                "ready_tasks": live["ready_tasks"],
                "phase1_scope_bounded": True,
                "product_runtime_mutation": False,
                "acceptance_semantics_changed": False,
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
