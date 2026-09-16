#!/usr/bin/env python3
"""Fail-closed eligibility validator for the PKG-03 COMPLETE -> PKG-04 04.01 handoff.

Self-test mode is the only mode used by Fast Epoch while PKG-03 remains active.
Real mode consumes canonical PKG-03 completion state plus a genuine 03.25 final
acceptance handoff. It never mutates canonical state or activates PKG-04 itself.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Any

SHA_RE = re.compile(r"^[0-9a-f]{40}$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
EXPECTED_TASKS = tuple(f"03.{index:02d}" for index in range(1, 26))
SECRET_FRAGMENTS = (
    "password", "private_key", "private-key", "pfx", "access_token",
    "api_token", "oidc_token", "client_secret",
)
TRUST_ROUTES = {"authenticode_provider", "microsoft_store"}


def canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8-sig"))
    if not isinstance(value, dict):
        raise AssertionError(f"expected JSON object: {path}")
    return value


def scan_no_secrets(value: object, path: str = "$") -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            lowered = str(key).lower()
            if any(fragment in lowered for fragment in SECRET_FRAGMENTS):
                raise AssertionError(f"secret-bearing field forbidden: {path}.{key}")
            scan_no_secrets(child, f"{path}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            scan_no_secrets(child, f"{path}[{index}]")


def task_map(tracker: dict[str, Any]) -> dict[str, dict[str, Any]]:
    rows = tracker.get("tasks")
    if not isinstance(rows, list):
        raise AssertionError("PKG-03 task records missing")
    result: dict[str, dict[str, Any]] = {}
    for row in rows:
        if not isinstance(row, dict) or not isinstance(row.get("id"), str):
            raise AssertionError("invalid PKG-03 task record")
        task_id = row["id"]
        if task_id in result:
            raise AssertionError(f"duplicate PKG-03 task: {task_id}")
        result[task_id] = row
    return result


def validate_pkg03_complete(tracker: dict[str, Any], status: dict[str, Any]) -> None:
    if tracker.get("package_id") != "PKG-03":
        raise AssertionError("tracker is not PKG-03")
    if tracker.get("done") != 25 or tracker.get("required") != 25:
        raise AssertionError("PKG-03 is not 25/25")
    if float(tracker.get("percent", -1)) != 100.0:
        raise AssertionError("PKG-03 percent is not 100")
    if tracker.get("complete") is not True or tracker.get("status") != "COMPLETE":
        raise AssertionError("PKG-03 is not canonically COMPLETE")
    if tracker.get("active_task") is not None:
        raise AssertionError("completed PKG-03 still has an active task")
    if tracker.get("active_tasks") not in ([], None):
        raise AssertionError("completed PKG-03 still has active tasks")
    if tracker.get("ready_tasks") not in ([], None):
        raise AssertionError("completed PKG-03 still has ready tasks")

    tasks = task_map(tracker)
    if tuple(tasks) != EXPECTED_TASKS:
        raise AssertionError("PKG-03 task denominator/order drifted")
    not_done = [task_id for task_id in EXPECTED_TASKS if tasks[task_id].get("status") != "DONE"]
    if not_done:
        raise AssertionError(f"PKG-03 tasks not DONE: {not_done}")

    packages = status.get("packages")
    if not isinstance(packages, list):
        raise AssertionError("master package table missing")
    pkg = next((row for row in packages if isinstance(row, dict) and row.get("id") == "PKG-03"), None)
    if not pkg:
        raise AssertionError("PKG-03 missing from master status")
    expected = {"id": "PKG-03", "name": "Windows Installer", "done": 25, "required": 25, "percent": 100.0, "status": "COMPLETE"}
    if pkg != expected:
        raise AssertionError("master/tracker PKG-03 completion mismatch")
    if not isinstance(status.get("packages_complete"), int) or status["packages_complete"] < 3:
        raise AssertionError("master package-complete count does not include PKG-03")
    if status.get("active_package") == "PKG-03" or status.get("active_task") in EXPECTED_TASKS:
        raise AssertionError("master status still projects PKG-03 as active after completion")


def validate_final_handoff(handoff: dict[str, Any]) -> dict[str, Any]:
    scan_no_secrets(handoff)
    if handoff.get("schema_version") != 1 or handoff.get("package_id") != "PKG-03" or handoff.get("task_id") != "03.25":
        raise AssertionError("03.25 final handoff identity mismatch")
    if handoff.get("mode") != "FINAL_ACCEPTANCE_HANDOFF" or handoff.get("production_accepted") is not True:
        raise AssertionError("03.25 final handoff is not production-accepted")

    source = handoff.get("source_commit")
    if not isinstance(source, str) or not SHA_RE.fullmatch(source):
        raise AssertionError("invalid final source commit")
    for field in ("workflow_run_id", "workflow_job_id", "artifact_id"):
        if not isinstance(handoff.get(field), int) or handoff[field] <= 0:
            raise AssertionError(f"invalid {field}")
    for field in (
        "final_evidence_index_sha256",
        "release_subjects_sha256",
        "signed_subjects_sha256",
        "sbom_sha256",
        "attestation_bundle_sha256",
        "vm_matrix_sha256",
        "trust_evidence_sha256",
    ):
        value = handoff.get(field)
        if not isinstance(value, str) or not SHA256_RE.fullmatch(value):
            raise AssertionError(f"invalid {field}")

    trust_route = handoff.get("production_trust_route")
    if trust_route not in TRUST_ROUTES:
        raise AssertionError("unknown production trust route")
    required_true = (
        "all_pkg03_tasks_accepted",
        "signatures_reverified",
        "provenance_reverified",
        "vm_acceptance_reverified",
        "cleanup_verified",
        "forbidden_system_mutation_zero",
    )
    for field in required_true:
        if handoff.get(field) is not True:
            raise AssertionError(f"final handoff requirement missing: {field}")
    return {
        "source_commit": source,
        "trust_route": trust_route,
        "final_evidence_index_sha256": handoff["final_evidence_index_sha256"],
        "trust_evidence_sha256": handoff["trust_evidence_sha256"],
    }


def make_complete_tracker() -> dict[str, Any]:
    return {
        "package_id": "PKG-03",
        "done": 25,
        "required": 25,
        "percent": 100.0,
        "complete": True,
        "status": "COMPLETE",
        "active_task": None,
        "active_tasks": [],
        "ready_tasks": [],
        "tasks": [{"id": task_id, "status": "DONE"} for task_id in EXPECTED_TASKS],
    }


def make_complete_status() -> dict[str, Any]:
    return {
        "packages_complete": 3,
        "active_package": None,
        "active_task": None,
        "packages": [
            {"id": "PKG-03", "name": "Windows Installer", "done": 25, "required": 25, "percent": 100.0, "status": "COMPLETE"}
        ],
    }


def make_handoff() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "package_id": "PKG-03",
        "task_id": "03.25",
        "mode": "FINAL_ACCEPTANCE_HANDOFF",
        "production_accepted": True,
        "source_commit": "e" * 40,
        "workflow_run_id": 101,
        "workflow_job_id": 102,
        "artifact_id": 103,
        "final_evidence_index_sha256": "1" * 64,
        "release_subjects_sha256": "2" * 64,
        "signed_subjects_sha256": "3" * 64,
        "sbom_sha256": "4" * 64,
        "attestation_bundle_sha256": "5" * 64,
        "vm_matrix_sha256": "6" * 64,
        "trust_evidence_sha256": "7" * 64,
        "production_trust_route": "microsoft_store",
        "all_pkg03_tasks_accepted": True,
        "signatures_reverified": True,
        "provenance_reverified": True,
        "vm_acceptance_reverified": True,
        "cleanup_verified": True,
        "forbidden_system_mutation_zero": True,
    }


def self_test() -> dict[str, Any]:
    tracker = make_complete_tracker()
    status = make_complete_status()
    handoff = make_handoff()
    validate_pkg03_complete(tracker, status)
    normalized = validate_final_handoff(handoff)

    negatives = 0
    cases: list[tuple[str, str, Any]] = [
        ("tracker-not-complete", "tracker", lambda value: value.__setitem__("complete", False)),
        ("tracker-24-of-25", "tracker", lambda value: value.__setitem__("done", 24)),
        ("task-0325-not-done", "tracker", lambda value: value["tasks"][-1].__setitem__("status", "READY")),
        ("master-still-pkg03-active", "status", lambda value: value.__setitem__("active_package", "PKG-03")),
        ("handoff-not-accepted", "handoff", lambda value: value.__setitem__("production_accepted", False)),
        ("bad-final-index", "handoff", lambda value: value.__setitem__("final_evidence_index_sha256", "bad")),
        ("unknown-trust-route", "handoff", lambda value: value.__setitem__("production_trust_route", "self-signed")),
        ("provenance-not-reverified", "handoff", lambda value: value.__setitem__("provenance_reverified", False)),
        ("vm-not-reverified", "handoff", lambda value: value.__setitem__("vm_acceptance_reverified", False)),
        ("secret-field", "handoff", lambda value: value.__setitem__("client_secret", "forbidden")),
    ]
    for label, target, mutate in cases:
        candidate_tracker = json.loads(json.dumps(tracker))
        candidate_status = json.loads(json.dumps(status))
        candidate_handoff = json.loads(json.dumps(handoff))
        selected = {"tracker": candidate_tracker, "status": candidate_status, "handoff": candidate_handoff}[target]
        mutate(selected)
        try:
            validate_pkg03_complete(candidate_tracker, candidate_status)
            validate_final_handoff(candidate_handoff)
        except AssertionError:
            negatives += 1
        else:
            raise AssertionError(f"negative unexpectedly passed: {label}")

    return {
        "schema_version": 1,
        "task_id": "04.01",
        "mode": "NON_ACCEPTANCE_PREIMPLEMENTATION",
        "production_evidence_consumed": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "pkg04_activated": False,
        "normalized_pkg03_final_handoff_ready": True,
        "accepted_trust_route_fixture": normalized["trust_route"],
        "negative_rejections": negatives,
        "self_test_pass": True,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--pkg03-tracker")
    parser.add_argument("--master-status")
    parser.add_argument("--handoff")
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    if args.self_test:
        report = self_test()
    else:
        if not args.pkg03_tracker or not args.master_status or not args.handoff:
            raise SystemExit("real mode requires --pkg03-tracker, --master-status and --handoff")
        tracker = read_json(Path(args.pkg03_tracker))
        status = read_json(Path(args.master_status))
        handoff = read_json(Path(args.handoff))
        validate_pkg03_complete(tracker, status)
        normalized = validate_final_handoff(handoff)
        handoff_digest = hashlib.sha256(canonical(handoff)).hexdigest()
        report = {
            "schema_version": 1,
            "task_id": "04.01",
            "mode": "ACTIVATION_ELIGIBILITY_VERIFIED",
            "production_evidence_consumed": True,
            "canonical_state_changed": False,
            "implementation_authority": False,
            "pkg04_activated": False,
            "pkg03_source_commit": normalized["source_commit"],
            "production_trust_route": normalized["trust_route"],
            "final_evidence_index_sha256": normalized["final_evidence_index_sha256"],
            "trust_evidence_sha256": normalized["trust_evidence_sha256"],
            "pkg03_final_handoff_sha256": handoff_digest,
            "ready_for_0401_reconciliation": True,
        }

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical(report))
    print(json.dumps(report, sort_keys=True))
    print("PKG04_0401_ACTIVATION_PREFLIGHT=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
