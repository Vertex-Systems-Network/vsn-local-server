#!/usr/bin/env python3
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / ".ai/manifests/pkg04-pkg08-parallel-preplanning.v1.json"
MASTER = ROOT / "docs/MASTER-EXECUTION-PLAN.md"
EXPECTED = {
    "PKG-04": ("04", 18, "PKG-03 COMPLETE"),
    "PKG-05": ("05", 23, "PKG-04 COMPLETE"),
    "PKG-06": ("06", 20, "PKG-05 COMPLETE"),
    "PKG-07": ("07", 22, "PKG-06 COMPLETE"),
    "PKG-08": ("08", 25, "PKG-07 COMPLETE"),
}


def fail(message: str) -> None:
    raise SystemExit(f"PKG-04..08 preplan validation failed: {message}")


def master_denominators(markdown: str) -> dict[str, int]:
    result: dict[str, int] = {}
    for line in markdown.splitlines():
        match = re.match(r"^\|\s*(PKG-0[4-8])\s*\|[^|]*\|\s*(\d+)\s*\|", line)
        if match:
            result[match.group(1)] = int(match.group(2))
    return result


def main() -> None:
    data = json.loads(MANIFEST.read_text(encoding="utf-8"))
    if data.get("status") != "PREPARED_BLOCKED":
        fail("manifest must remain PREPARED_BLOCKED")
    if data.get("current_canonical_package") != "PKG-03":
        fail("portfolio preplan must not advance canonical package")
    if data.get("max_parallel_implementation_tasks") != 5:
        fail("max implementation concurrency must remain 5")
    if data.get("total_prepared_tasks") != 108:
        fail("total prepared denominator must be 108")
    if data.get("activation_order") != ["PKG-03", "PKG-04", "PKG-05", "PKG-06", "PKG-07", "PKG-08"]:
        fail("activation order changed")

    packages = data.get("packages") or []
    if [p.get("id") for p in packages] != list(EXPECTED):
        fail("package order/identity changed")

    counted = 0
    for package in packages:
        pid = package["id"]
        prefix, count, activation = EXPECTED[pid]
        if package.get("task_count") != count:
            fail(f"{pid} task_count must be {count}")
        if package.get("activation_requires") != activation:
            fail(f"{pid} activation prerequisite changed")
        tasks = package.get("tasks") or []
        expected_ids = [f"{prefix}.{i:02d}" for i in range(1, count + 1)]
        ids = [t.get("id") for t in tasks]
        if ids != expected_ids:
            fail(f"{pid} task IDs/order are not contiguous {expected_ids[0]}..{expected_ids[-1]}")
        index = {task_id: i for i, task_id in enumerate(ids)}
        for task in tasks:
            tid = task["id"]
            deps = task.get("depends_on")
            if not isinstance(deps, list):
                fail(f"{tid} depends_on must be a list")
            if tid.endswith(".01") and deps:
                fail(f"{tid} activation task must not depend on a same-package task")
            if len(deps) != len(set(deps)):
                fail(f"{tid} contains duplicate dependencies")
            for dep in deps:
                if dep not in index:
                    fail(f"{tid} depends on unknown/out-of-package task {dep}")
                if index[dep] >= index[tid]:
                    fail(f"{tid} depends on non-predecessor task {dep}")
        counted += len(tasks)

    if counted != 108:
        fail(f"counted {counted} tasks, expected 108")

    observed = master_denominators(MASTER.read_text(encoding="utf-8"))
    expected_counts = {pid: count for pid, (_, count, _) in EXPECTED.items()}
    if observed != expected_counts:
        fail(f"master denominators changed: expected {expected_counts}, observed {observed}")

    print("PKG-04..PKG-08 preplan DAG valid: 5 packages, 108 tasks, package-gated activation, max concurrency 5")


if __name__ == "__main__":
    main()
