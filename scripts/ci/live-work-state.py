#!/usr/bin/env python3
"""Emit live canonical work state from repository authority.

This intentionally ignores .ai/current-work.json for acceptance because that file is a
historical/non-authoritative checkpoint. The command fails closed if the active package
tracker cannot be resolved uniquely.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MASTER = ROOT / "docs" / "MASTER-EXECUTION-STATUS.json"
CERTIFICATION = ROOT / "certification"


def load_json(path: Path) -> dict:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:  # noqa: BLE001 - fail-closed CLI boundary
        raise SystemExit(f"failed to read JSON {path.relative_to(ROOT)}: {exc}") from exc
    if not isinstance(value, dict):
        raise SystemExit(f"expected JSON object: {path.relative_to(ROOT)}")
    return value


def resolve_head(explicit: str | None) -> str:
    if explicit:
        return explicit
    env_head = os.environ.get("GITHUB_SHA")
    if env_head:
        return env_head
    try:
        return subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True, stderr=subprocess.DEVNULL
        ).strip()
    except Exception as exc:  # noqa: BLE001
        raise SystemExit(f"cannot resolve live HEAD: {exc}") from exc


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--head", help="override repository HEAD recorded in output")
    parser.add_argument("--compact", action="store_true", help="emit one-line JSON")
    args = parser.parse_args()

    master = load_json(MASTER)
    active_package = master.get("active_package")
    if not isinstance(active_package, str) or not active_package:
        raise SystemExit("master status has no valid active_package")

    matches: list[tuple[Path, dict]] = []
    for path in sorted(CERTIFICATION.glob("*.json")):
        try:
            obj = json.loads(path.read_text(encoding="utf-8"))
        except Exception:
            continue
        if isinstance(obj, dict) and obj.get("package_id") == active_package:
            matches.append((path, obj))

    if len(matches) != 1:
        names = [p.name for p, _ in matches]
        raise SystemExit(
            f"expected exactly one active tracker for {active_package}; found {len(matches)}: {names}"
        )

    tracker_path, tracker = matches[0]
    master_task = master.get("active_task")
    tracker_task = tracker.get("active_task")
    if master_task != tracker_task:
        raise SystemExit(
            f"canonical active_task mismatch: master={master_task!r} tracker={tracker_task!r}"
        )

    tasks = tracker.get("tasks") if isinstance(tracker.get("tasks"), list) else []
    ready = tracker.get("ready_tasks") if isinstance(tracker.get("ready_tasks"), list) else []
    blocked = [
        t.get("id")
        for t in tasks
        if isinstance(t, dict) and t.get("status") == "BLOCKED" and isinstance(t.get("id"), str)
    ]
    in_progress = [
        t.get("id")
        for t in tasks
        if isinstance(t, dict) and t.get("status") == "IN_PROGRESS" and isinstance(t.get("id"), str)
    ]

    output = {
        "schema_version": 1,
        "source": "live-canonical-repository-state",
        "head": resolve_head(args.head),
        "product_version": master.get("product_version"),
        "active_package": active_package,
        "active_task": tracker_task,
        "ready_tasks": ready,
        "in_progress_tasks": in_progress,
        "blocked_tasks": blocked,
        "max_parallel_tasks": tracker.get("max_parallel_tasks"),
        "package_progress": {
            "done": tracker.get("done"),
            "required": tracker.get("required"),
            "percent": tracker.get("percent"),
            "status": tracker.get("status"),
            "complete": tracker.get("complete"),
        },
        "tracker": tracker_path.relative_to(ROOT).as_posix(),
        "acceptance_authority": [
            "docs/MASTER-EXECUTION-STATUS.json",
            tracker_path.relative_to(ROOT).as_posix(),
        ],
        "current_work_checkpoint_authoritative": False,
        "fast_profile": ".ai/governance/FAST-DEVELOPMENT.md",
    }

    json.dump(output, sys.stdout, sort_keys=True, indent=None if args.compact else 2)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
