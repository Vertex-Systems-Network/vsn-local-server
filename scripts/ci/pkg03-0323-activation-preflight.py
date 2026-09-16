#!/usr/bin/env python3
"""Fail-closed activation validator for the PKG-03 03.22 -> 03.23 handoff.

The self-test path is non-acceptance preimplementation only. Real mode requires a
canonical tracker that has accepted 03.22 and activated 03.23 plus a normalized,
provider-neutral production handoff bound to the exact four release subjects.
This command never mutates canonical state.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import tempfile
from pathlib import Path
from typing import Any

EXPECTED_FILES = (
    "nsis-current-user.exe",
    "nsis-per-machine.exe",
    "vsn-platform.msi",
    "VSN Dev Platform.exe",
)
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
SHA_RE = re.compile(r"^[0-9a-f]{40}$")
SECRET_FRAGMENTS = (
    "password",
    "private_key",
    "private-key",
    "pfx",
    "access_token",
    "api_token",
    "oidc_token",
    "client_secret",
)


def canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


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


def task_by_id(tracker: dict[str, Any], task_id: str) -> dict[str, Any]:
    tasks = tracker.get("tasks")
    if not isinstance(tasks, list):
        raise AssertionError("tracker tasks missing")
    matches = [row for row in tasks if isinstance(row, dict) and row.get("id") == task_id]
    if len(matches) != 1:
        raise AssertionError(f"expected one tracker task {task_id}, found {len(matches)}")
    return matches[0]


def validate_tracker(tracker: dict[str, Any]) -> None:
    if tracker.get("package_id") != "PKG-03":
        raise AssertionError("activation tracker is not PKG-03")
    if tracker.get("active_task") != "03.23":
        raise AssertionError("03.23 is not the canonical active task")
    ready = tracker.get("ready_tasks")
    if ready != ["03.23"]:
        raise AssertionError(f"canonical ready set is not exactly 03.23: {ready!r}")
    if task_by_id(tracker, "03.22").get("status") != "DONE":
        raise AssertionError("03.22 is not canonically DONE")
    task23 = task_by_id(tracker, "03.23")
    if task23.get("status") not in {"READY", "IN_PROGRESS"}:
        raise AssertionError("03.23 is not canonically READY/IN_PROGRESS")
    if tracker.get("done") != 22 or tracker.get("required") != 25:
        raise AssertionError("PKG-03 progress is not the expected 22/25 activation state")


def validate_handoff(handoff: dict[str, Any], artifact_dir: Path) -> dict[str, Any]:
    scan_no_secrets(handoff)
    if handoff.get("schema_version") != 1:
        raise AssertionError("unexpected production handoff schema")
    if handoff.get("package_id") != "PKG-03" or handoff.get("task_id") != "03.22":
        raise AssertionError("production handoff task identity mismatch")
    if handoff.get("mode") != "PRODUCTION_ACCEPTANCE_HANDOFF":
        raise AssertionError("production handoff mode missing")
    source = handoff.get("source_commit")
    if not isinstance(source, str) or not SHA_RE.fullmatch(source):
        raise AssertionError("invalid production source commit")
    for field in ("workflow_run_id", "workflow_job_id", "artifact_id"):
        if not isinstance(handoff.get(field), int) or handoff[field] <= 0:
            raise AssertionError(f"invalid/missing {field}")
    digest = handoff.get("artifact_digest")
    if not isinstance(digest, str) or not re.fullmatch(r"sha256:[0-9a-f]{64}", digest):
        raise AssertionError("invalid production artifact digest")
    if handoff.get("production_accepted") is not True:
        raise AssertionError("upstream production acceptance is not true")
    if handoff.get("test_certificate_used") is not False:
        raise AssertionError("test/self-signed evidence cannot activate 03.23")
    if handoff.get("provider_or_store_acceptance_verified") is not True:
        raise AssertionError("trusted provider/Store acceptance is not verified")

    subjects = handoff.get("subjects")
    if not isinstance(subjects, list) or len(subjects) != len(EXPECTED_FILES):
        raise AssertionError("production subject set must contain exactly four files")
    by_name: dict[str, dict[str, Any]] = {}
    for row in subjects:
        if not isinstance(row, dict):
            raise AssertionError("subject row is not an object")
        name = row.get("file_name")
        if not isinstance(name, str) or name in by_name:
            raise AssertionError("invalid/duplicate subject filename")
        by_name[name] = row
    if set(by_name) != set(EXPECTED_FILES):
        raise AssertionError(f"production subject filenames mismatch: {sorted(by_name)}")

    actual_files = sorted(path.name for path in artifact_dir.iterdir() if path.is_file())
    if actual_files != sorted(EXPECTED_FILES):
        raise AssertionError(f"artifact directory must contain exactly accepted subjects: {actual_files}")

    verified: list[dict[str, Any]] = []
    signer_subjects: set[str] = set()
    for name in EXPECTED_FILES:
        row = by_name[name]
        path = artifact_dir / name
        expected_hash = row.get("sha256")
        if not isinstance(expected_hash, str) or not SHA256_RE.fullmatch(expected_hash):
            raise AssertionError(f"invalid SHA-256 for {name}")
        size = row.get("size_bytes")
        if not isinstance(size, int) or size <= 0:
            raise AssertionError(f"invalid size for {name}")
        if path.stat().st_size != size:
            raise AssertionError(f"size mismatch for {name}")
        actual_hash = sha256_file(path)
        if actual_hash != expected_hash:
            raise AssertionError(f"SHA-256 mismatch for {name}")
        if row.get("authenticode_status") != "Valid":
            raise AssertionError(f"production Authenticode status is not Valid for {name}")
        signer = row.get("signer_subject")
        if not isinstance(signer, str) or not signer.strip():
            raise AssertionError(f"signer subject missing for {name}")
        signer_upper = signer.upper()
        if "SYNTHETIC" in signer_upper or "TEST SIGN" in signer_upper or "CI TEST" in signer_upper:
            raise AssertionError(f"test/synthetic signer forbidden for {name}")
        if row.get("timestamp_verified") is not True:
            raise AssertionError(f"timestamp verification missing for {name}")
        signer_subjects.add(signer)
        verified.append({"file_name": name, "size_bytes": size, "sha256": actual_hash, "signer_subject": signer})
    if len(signer_subjects) != 1:
        raise AssertionError("production subjects do not share one accepted signer identity")
    return {"source_commit": source, "subjects": verified, "signer_subject": next(iter(signer_subjects))}


def self_test() -> dict[str, Any]:
    with tempfile.TemporaryDirectory(prefix="vsn-0323-activation-") as tmp:
        root = Path(tmp)
        artifacts = root / "subjects"
        artifacts.mkdir()
        rows: list[dict[str, Any]] = []
        for index, name in enumerate(EXPECTED_FILES, start=1):
            payload = (f"VSN-ACTIVATION-FIXTURE:{index}:{name}\n" * 32).encode("utf-8")
            path = artifacts / name
            path.write_bytes(payload)
            rows.append(
                {
                    "file_name": name,
                    "size_bytes": len(payload),
                    "sha256": hashlib.sha256(payload).hexdigest(),
                    "authenticode_status": "Valid",
                    "signer_subject": "CN=VSN Production Publisher Fixture",
                    "timestamp_verified": True,
                }
            )
        tracker = {
            "package_id": "PKG-03",
            "done": 22,
            "required": 25,
            "active_task": "03.23",
            "ready_tasks": ["03.23"],
            "tasks": [
                {"id": "03.22", "status": "DONE"},
                {"id": "03.23", "status": "READY"},
            ],
        }
        handoff: dict[str, Any] = {
            "schema_version": 1,
            "package_id": "PKG-03",
            "task_id": "03.22",
            "mode": "PRODUCTION_ACCEPTANCE_HANDOFF",
            "source_commit": "a" * 40,
            "workflow_run_id": 1,
            "workflow_job_id": 2,
            "artifact_id": 3,
            "artifact_digest": "sha256:" + "b" * 64,
            "production_accepted": True,
            "test_certificate_used": False,
            "provider_or_store_acceptance_verified": True,
            "subjects": rows,
        }
        validate_tracker(tracker)
        valid = validate_handoff(handoff, artifacts)
        negatives = 0
        cases: list[tuple[str, Any]] = []

        bad = json.loads(json.dumps(handoff)); bad["production_accepted"] = False
        cases.append(("production-not-accepted", bad))
        bad = json.loads(json.dumps(handoff)); bad["test_certificate_used"] = True
        cases.append(("test-certificate", bad))
        bad = json.loads(json.dumps(handoff)); bad["subjects"][0]["sha256"] = "0" * 64
        cases.append(("subject-hash-mismatch", bad))
        bad = json.loads(json.dumps(handoff)); bad["subjects"][0]["signer_subject"] = "CN=VSN CI Test Signing"
        cases.append(("test-signer", bad))
        bad = json.loads(json.dumps(handoff)); bad["client_secret"] = "forbidden"
        cases.append(("secret-bearing-field", bad))
        for label, candidate in cases:
            try:
                validate_handoff(candidate, artifacts)
            except AssertionError:
                negatives += 1
            else:
                raise AssertionError(f"negative self-test unexpectedly passed: {label}")

        blocked_tracker = json.loads(json.dumps(tracker))
        blocked_tracker["active_task"] = "03.22"
        blocked_tracker["ready_tasks"] = ["03.22"]
        blocked_tracker["done"] = 21
        blocked_tracker["tasks"][0]["status"] = "READY"
        try:
            validate_tracker(blocked_tracker)
        except AssertionError:
            negatives += 1
        else:
            raise AssertionError("blocked canonical tracker unexpectedly activated 03.23")

        return {
            "schema_version": 1,
            "task_id": "03.23",
            "mode": "NON_ACCEPTANCE_PREIMPLEMENTATION",
            "production_evidence_consumed": False,
            "canonical_state_changed": False,
            "implementation_authority": False,
            "normalized_handoff_contract_ready": True,
            "expected_subject_count": len(EXPECTED_FILES),
            "verified_fixture_subject_count": len(valid["subjects"]),
            "negative_rejections": negatives,
            "self_test_pass": True,
        }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--tracker")
    parser.add_argument("--handoff")
    parser.add_argument("--artifact-dir")
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    if args.self_test:
        report = self_test()
    else:
        if not args.tracker or not args.handoff or not args.artifact_dir:
            raise SystemExit("real mode requires --tracker, --handoff and --artifact-dir")
        tracker = read_json(Path(args.tracker))
        handoff = read_json(Path(args.handoff))
        artifact_dir = Path(args.artifact_dir)
        if not artifact_dir.is_dir():
            raise SystemExit("artifact directory missing")
        validate_tracker(tracker)
        verified = validate_handoff(handoff, artifact_dir)
        report = {
            "schema_version": 1,
            "task_id": "03.23",
            "mode": "ACTIVATION_PREFLIGHT_VERIFIED",
            "production_evidence_consumed": True,
            "canonical_state_changed": False,
            "implementation_authority": False,
            "source_commit": verified["source_commit"],
            "subject_count": len(verified["subjects"]),
            "signer_subject": verified["signer_subject"],
            "subjects_sha256": hashlib.sha256(canonical(verified["subjects"])).hexdigest(),
            "ready_for_0323_implementation_gate": True,
        }

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical(report))
    print(json.dumps(report, sort_keys=True))
    print("PKG03_0323_ACTIVATION_PREFLIGHT=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
