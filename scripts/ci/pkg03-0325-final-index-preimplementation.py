#!/usr/bin/env python3
"""Synthetic-only final evidence index builder for PKG-03 03.25."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

TASKS = tuple(f"03.{i:02d}" for i in range(2, 25))
FILES = (
    "nsis-current-user.exe",
    "nsis-per-machine.exe",
    "vsn-platform.msi",
    "VSN Dev Platform.exe",
)


def sha(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def make_index() -> dict:
    packages = [{"file_name": name, "sha256": sha(f"fast-epoch1:signed:{name}")} for name in FILES]
    source = "6" * 40
    return {
        "schema_version": 1,
        "task_id": "03.25",
        "mode": "NON_ACCEPTANCE_PREIMPLEMENTATION",
        "production_evidence_consumed": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "source_commit": source,
        "dependencies": [
            {"task_id": task, "status": "SYNTHETIC_DONE", "evidence_sha256": sha(f"synthetic-evidence:{task}")}
            for task in TASKS
        ],
        "signing": {"task_id": "03.22", "synthetic": True, "source_commit": source, "packages": packages},
        "provenance": {"task_id": "03.23", "synthetic": True, "source_commit": source, "packages": packages},
        "vm_matrix": {"task_id": "03.24", "synthetic": True, "source_commit": source, "packages": packages},
        "full_regression_pass": "SYNTHETIC_ONLY",
        "independent_verification_pass": "SYNTHETIC_ONLY",
        "tracked_drift_zero": True,
        "secret_leak_scan_pass": True,
        "pkg04_handoff": {"sha256": sha("synthetic-pkg04-handoff"), "pkg04_activated": False},
        "pkg03_complete_projected": False,
    }


def validate(index: dict) -> None:
    assert index["mode"] == "NON_ACCEPTANCE_PREIMPLEMENTATION"
    assert index["production_evidence_consumed"] is False
    assert index["canonical_state_changed"] is False
    assert index["implementation_authority"] is False
    assert index["pkg04_handoff"]["pkg04_activated"] is False
    assert index["pkg03_complete_projected"] is False
    deps = index["dependencies"]
    assert len(deps) == len(TASKS)
    assert {d["task_id"] for d in deps} == set(TASKS)
    assert all(d["status"] == "SYNTHETIC_DONE" for d in deps)
    assert index["full_regression_pass"] == "SYNTHETIC_ONLY"
    assert index["independent_verification_pass"] == "SYNTHETIC_ONLY"

    def pmap(section: str) -> dict[str, str]:
        return {row["file_name"]: row["sha256"] for row in index[section]["packages"]}

    assert pmap("signing") == pmap("provenance") == pmap("vm_matrix")
    assert set(pmap("signing")) == set(FILES)
    assert index["signing"]["source_commit"] == index["provenance"]["source_commit"] == index["vm_matrix"]["source_commit"] == index["source_commit"]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true", required=True)
    parser.add_argument("--output-dir", required=True)
    args = parser.parse_args()
    index = make_index()
    validate(index)
    payload = canonical(index)
    out = Path(args.output_dir)
    out.mkdir(parents=True, exist_ok=True)
    (out / "synthetic-final-index.json").write_bytes(payload)
    report = {
        "schema_version": 1,
        "task_id": "03.25",
        "mode": "NON_ACCEPTANCE_PREIMPLEMENTATION",
        "production_evidence_consumed": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "pkg03_complete_projected": False,
        "pkg04_activated": False,
        "dependency_count": len(TASKS),
        "index_sha256": hashlib.sha256(payload).hexdigest(),
        "deterministic_outputs": True,
    }
    (out / "report.json").write_bytes(canonical(report))
    print(json.dumps(report, sort_keys=True))
    print("PKG03_0325_FINAL_INDEX_PREIMPLEMENTATION=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
