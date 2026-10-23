"""Accepted-descendant check shared by the PKG-03 task validators.

A task validator binds the repository state at the moment its task was accepted. Later
PKG-03 tasks and product work legitimately change that state, so once a task is DONE its
validator accepts any consistent canonical descendant instead: the task and its
dependencies are DONE, tracker counts and cursor agree with master status, and the
accepted evidence source (when recorded) is an ancestor of HEAD.
"""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TRACKER = ROOT / "certification/pkg03-windows-installer-v1.json"
STATUS = ROOT / "docs/MASTER-EXECUTION-STATUS.json"


def _git(*args: str) -> int:
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, check=False).returncode


def _is_ancestor(commit: str) -> bool:
    # Gate workflows check out shallowly; fetch the evidence commit and its history if absent.
    if _git("cat-file", "-e", f"{commit}^{{commit}}") != 0:
        _git("fetch", "--quiet", "--unshallow", "origin")
        _git("fetch", "--quiet", "origin", commit)
    proc = subprocess.run(
        ["git", "merge-base", "--is-ancestor", commit, "HEAD"],
        cwd=ROOT,
        capture_output=True,
        check=False,
    )
    return proc.returncode == 0


def accepted_descendant(task_id: str) -> dict | None:
    """Summary dict when HEAD is a consistent descendant of the accepted task, else None."""
    tracker = json.loads(TRACKER.read_text(encoding="utf-8"))
    status = json.loads(STATUS.read_text(encoding="utf-8"))
    tasks = {t["id"]: t for t in tracker.get("tasks", [])}
    task = tasks.get(task_id)
    done = tracker.get("done")
    if not task or task.get("status") != "DONE" or tracker.get("required") != 25:
        return None
    if not isinstance(done, int) or done <= int(task_id.split(".")[1]):
        return None  # the task's own acceptance snapshot keeps its strict checks
    done_ids = [tid for tid, t in tasks.items() if t.get("status") == "DONE"]
    if done != len(done_ids) or round(float(tracker.get("percent")), 2) != round(done * 100 / 25, 2):
        raise SystemExit(f"PKG-03 {task_id} descendant: tracker progress is inconsistent")
    for tid in done_ids:
        for dep in tasks[tid].get("depends_on", []):
            if tasks.get(dep, {}).get("status") != "DONE":
                raise SystemExit(f"PKG-03 {task_id} descendant: {tid} is DONE but {dep} is not")
    pkg = next((p for p in status.get("packages", []) if p.get("id") == "PKG-03"), {})
    if pkg.get("done") != done or pkg.get("required") != 25:
        raise SystemExit(f"PKG-03 {task_id} descendant: master status disagrees with tracker")
    if status.get("active_task") != tracker.get("active_task"):
        raise SystemExit(f"PKG-03 {task_id} descendant: master/tracker cursor disagree")
    evidence = task.get("evidence") if isinstance(task.get("evidence"), dict) else {}
    source = evidence.get("source_commit")
    if source and not _is_ancestor(str(source)):
        raise SystemExit(f"PKG-03 {task_id} descendant: accepted evidence source is not an ancestor")
    return {"task": task_id, "phase": "accepted-descendant", "done": done,
            "cursor": tracker.get("active_task"), "evidence_source": source, "valid": True}
