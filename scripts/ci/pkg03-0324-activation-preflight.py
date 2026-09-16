#!/usr/bin/env python3
"""Fail-closed activation validator for the PKG-03 03.23 -> 03.24 handoff."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import tempfile
from pathlib import Path
from typing import Any

EXPECTED_INSTALLERS = ("nsis-current-user.exe", "nsis-per-machine.exe", "vsn-platform.msi")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
SHA_RE = re.compile(r"^[0-9a-f]{40}$")
SECRET_FRAGMENTS = ("password", "private_key", "private-key", "pfx", "access_token", "api_token", "oidc_token", "client_secret")


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


def task(tracker: dict[str, Any], task_id: str) -> dict[str, Any]:
    rows = tracker.get("tasks")
    if not isinstance(rows, list):
        raise AssertionError("tracker tasks missing")
    matches = [row for row in rows if isinstance(row, dict) and row.get("id") == task_id]
    if len(matches) != 1:
        raise AssertionError(f"expected one task {task_id}")
    return matches[0]


def validate_tracker(tracker: dict[str, Any]) -> None:
    if tracker.get("package_id") != "PKG-03":
        raise AssertionError("tracker is not PKG-03")
    if tracker.get("active_task") != "03.24" or tracker.get("ready_tasks") != ["03.24"]:
        raise AssertionError("03.24 is not the sole canonical active/ready task")
    if task(tracker, "03.23").get("status") != "DONE":
        raise AssertionError("03.23 is not canonically DONE")
    if task(tracker, "03.24").get("status") not in {"READY", "IN_PROGRESS"}:
        raise AssertionError("03.24 is not READY/IN_PROGRESS")
    if tracker.get("done") != 23 or tracker.get("required") != 25:
        raise AssertionError("PKG-03 progress is not expected 23/25 activation state")


def validate_handoff(handoff: dict[str, Any]) -> dict[str, Any]:
    scan_no_secrets(handoff)
    if handoff.get("schema_version") != 1 or handoff.get("package_id") != "PKG-03" or handoff.get("task_id") != "03.23":
        raise AssertionError("03.23 handoff identity mismatch")
    if handoff.get("mode") != "PROVENANCE_ACCEPTANCE_HANDOFF" or handoff.get("production_accepted") is not True:
        raise AssertionError("03.23 provenance handoff is not accepted")
    source = handoff.get("source_commit")
    if not isinstance(source, str) or not SHA_RE.fullmatch(source):
        raise AssertionError("invalid source commit")
    for field in ("workflow_run_id", "workflow_job_id", "artifact_id"):
        if not isinstance(handoff.get(field), int) or handoff[field] <= 0:
            raise AssertionError(f"invalid {field}")
    for field in ("handoff_sha256", "sbom_sha256", "provenance_sha256", "attestation_bundle_sha256"):
        value = handoff.get(field)
        if not isinstance(value, str) or not SHA256_RE.fullmatch(value):
            raise AssertionError(f"invalid {field}")
    if handoff.get("attestation_subject_binding_verified") is not True:
        raise AssertionError("attestation subject binding not verified")
    if handoff.get("signatures_reverified") is not True:
        raise AssertionError("production signatures were not reverified")

    subjects = handoff.get("subjects")
    if not isinstance(subjects, list):
        raise AssertionError("subjects missing")
    by_name: dict[str, dict[str, Any]] = {}
    for row in subjects:
        if not isinstance(row, dict):
            raise AssertionError("subject row invalid")
        name = row.get("file_name")
        if not isinstance(name, str) or name in by_name:
            raise AssertionError("invalid/duplicate subject")
        by_name[name] = row
    if not set(EXPECTED_INSTALLERS).issubset(by_name):
        raise AssertionError("required installer subjects missing")
    for name in EXPECTED_INSTALLERS:
        row = by_name[name]
        if not isinstance(row.get("sha256"), str) or not SHA256_RE.fullmatch(row["sha256"]):
            raise AssertionError(f"invalid hash for {name}")
        if row.get("authenticode_status") != "Valid" or row.get("timestamp_verified") is not True:
            raise AssertionError(f"signature/timestamp not valid for {name}")
        if row.get("provenance_subject_verified") is not True:
            raise AssertionError(f"provenance subject binding missing for {name}")
    return {"source_commit": source, "subjects": [by_name[name] for name in EXPECTED_INSTALLERS]}


def self_test() -> dict[str, Any]:
    tracker = {
        "package_id": "PKG-03", "done": 23, "required": 25,
        "active_task": "03.24", "ready_tasks": ["03.24"],
        "tasks": [{"id": "03.23", "status": "DONE"}, {"id": "03.24", "status": "READY"}],
    }
    subjects = [
        {"file_name": name, "sha256": hashlib.sha256(name.encode()).hexdigest(), "authenticode_status": "Valid", "timestamp_verified": True, "provenance_subject_verified": True}
        for name in EXPECTED_INSTALLERS
    ]
    handoff: dict[str, Any] = {
        "schema_version": 1, "package_id": "PKG-03", "task_id": "03.23", "mode": "PROVENANCE_ACCEPTANCE_HANDOFF",
        "source_commit": "c" * 40, "workflow_run_id": 1, "workflow_job_id": 2, "artifact_id": 3,
        "handoff_sha256": "1" * 64, "sbom_sha256": "2" * 64, "provenance_sha256": "3" * 64,
        "attestation_bundle_sha256": "4" * 64, "attestation_subject_binding_verified": True,
        "signatures_reverified": True, "production_accepted": True, "subjects": subjects,
    }
    validate_tracker(tracker)
    valid = validate_handoff(handoff)
    negatives = 0
    candidates: list[tuple[str, dict[str, Any]]] = []
    for label, mutator in (
        ("not-accepted", lambda x: x.__setitem__("production_accepted", False)),
        ("bad-sbom", lambda x: x.__setitem__("sbom_sha256", "bad")),
        ("attestation-unbound", lambda x: x.__setitem__("attestation_subject_binding_verified", False)),
        ("signature-not-reverified", lambda x: x.__setitem__("signatures_reverified", False)),
        ("missing-msi", lambda x: x.__setitem__("subjects", x["subjects"][:-1])),
        ("secret-field", lambda x: x.__setitem__("client_secret", "forbidden")),
    ):
        candidate = json.loads(json.dumps(handoff)); mutator(candidate); candidates.append((label, candidate))
    for label, candidate in candidates:
        try:
            validate_handoff(candidate)
        except AssertionError:
            negatives += 1
        else:
            raise AssertionError(f"negative unexpectedly passed: {label}")
    blocked = json.loads(json.dumps(tracker)); blocked["active_task"] = "03.23"; blocked["ready_tasks"] = ["03.23"]; blocked["done"] = 22; blocked["tasks"][0]["status"] = "READY"
    try:
        validate_tracker(blocked)
    except AssertionError:
        negatives += 1
    else:
        raise AssertionError("blocked tracker unexpectedly activated 03.24")
    return {
        "schema_version": 1, "task_id": "03.24", "mode": "NON_ACCEPTANCE_PREIMPLEMENTATION",
        "production_evidence_consumed": False, "canonical_state_changed": False, "implementation_authority": False,
        "normalized_provenance_handoff_ready": True, "installer_subject_count": len(valid["subjects"]),
        "negative_rejections": negatives, "self_test_pass": True,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--tracker")
    parser.add_argument("--handoff")
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    if args.self_test:
        report = self_test()
    else:
        if not args.tracker or not args.handoff:
            raise SystemExit("real mode requires --tracker and --handoff")
        tracker = read_json(Path(args.tracker)); handoff = read_json(Path(args.handoff))
        validate_tracker(tracker); verified = validate_handoff(handoff)
        report = {
            "schema_version": 1, "task_id": "03.24", "mode": "ACTIVATION_PREFLIGHT_VERIFIED",
            "production_evidence_consumed": True, "canonical_state_changed": False, "implementation_authority": False,
            "source_commit": verified["source_commit"], "installer_subject_count": len(verified["subjects"]),
            "installer_subjects_sha256": hashlib.sha256(canonical(verified["subjects"])).hexdigest(),
            "ready_for_0324_vm_gate": True,
        }
    output = Path(args.output); output.parent.mkdir(parents=True, exist_ok=True); output.write_bytes(canonical(report))
    print(json.dumps(report, sort_keys=True)); print("PKG03_0324_ACTIVATION_PREFLIGHT=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
