#!/usr/bin/env python3
"""Fail-closed activation validator for the PKG-03 03.24 -> 03.25 handoff."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Any

SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
SHA_RE = re.compile(r"^[0-9a-f]{40}$")
EXPECTED_DONE = tuple(f"03.{index:02d}" for index in range(2, 25))
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


def task_map(tracker: dict[str, Any]) -> dict[str, dict[str, Any]]:
    tasks = tracker.get("tasks")
    if not isinstance(tasks, list):
        raise AssertionError("tracker tasks missing")
    result: dict[str, dict[str, Any]] = {}
    for row in tasks:
        if isinstance(row, dict) and isinstance(row.get("id"), str):
            if row["id"] in result:
                raise AssertionError(f"duplicate task id: {row['id']}")
            result[row["id"]] = row
    return result


def validate_tracker(tracker: dict[str, Any]) -> None:
    if tracker.get("package_id") != "PKG-03":
        raise AssertionError("tracker is not PKG-03")
    if tracker.get("active_task") != "03.25" or tracker.get("ready_tasks") != ["03.25"]:
        raise AssertionError("03.25 is not the sole canonical active/ready task")
    tasks = task_map(tracker)
    missing = [task_id for task_id in EXPECTED_DONE if task_id not in tasks]
    if missing:
        raise AssertionError(f"required task records missing: {missing}")
    not_done = [task_id for task_id in EXPECTED_DONE if tasks[task_id].get("status") != "DONE"]
    if not_done:
        raise AssertionError(f"dependencies not DONE: {not_done}")
    if tasks.get("03.25", {}).get("status") not in {"READY", "IN_PROGRESS"}:
        raise AssertionError("03.25 is not READY/IN_PROGRESS")
    if tracker.get("done") != 24 or tracker.get("required") != 25:
        raise AssertionError("PKG-03 progress is not expected 24/25 activation state")
    if tracker.get("complete") is True:
        raise AssertionError("PKG-03 cannot be complete before 03.25 acceptance")


def validate_handoff(handoff: dict[str, Any]) -> dict[str, Any]:
    scan_no_secrets(handoff)
    if handoff.get("schema_version") != 1 or handoff.get("package_id") != "PKG-03" or handoff.get("task_id") != "03.24":
        raise AssertionError("03.24 handoff identity mismatch")
    if handoff.get("mode") != "VM_ACCEPTANCE_HANDOFF" or handoff.get("production_accepted") is not True:
        raise AssertionError("03.24 VM handoff is not accepted")
    source = handoff.get("source_commit")
    if not isinstance(source, str) or not SHA_RE.fullmatch(source):
        raise AssertionError("invalid source commit")
    for field in ("workflow_run_id", "workflow_job_id", "artifact_id"):
        if not isinstance(handoff.get(field), int) or handoff[field] <= 0:
            raise AssertionError(f"invalid {field}")
    for field in (
        "vm_matrix_sha256", "provenance_handoff_sha256", "signed_subjects_sha256",
        "sbom_sha256", "attestation_bundle_sha256",
    ):
        value = handoff.get(field)
        if not isinstance(value, str) or not SHA256_RE.fullmatch(value):
            raise AssertionError(f"invalid {field}")
    row_count = handoff.get("matrix_row_count")
    if not isinstance(row_count, int) or row_count < 3:
        raise AssertionError("VM matrix row count is insufficient")
    if handoff.get("all_rows_passed") is not True or handoff.get("cleanup_verified") is not True:
        raise AssertionError("VM matrix did not fully pass/cleanup")
    if handoff.get("forbidden_system_mutation_zero") is not True:
        raise AssertionError("forbidden system mutation evidence missing")
    if handoff.get("signatures_reverified") is not True or handoff.get("provenance_reverified") is not True:
        raise AssertionError("signature/provenance continuity was not reverified")
    reboot = handoff.get("real_reboot_rows")
    if not isinstance(reboot, list):
        raise AssertionError("real_reboot_rows missing")
    for row in reboot:
        if not isinstance(row, dict):
            raise AssertionError("reboot row invalid")
        if row.get("required") is True:
            if row.get("executed") is not True:
                raise AssertionError("required real reboot row was not executed")
            if row.get("same_machine_verified") is not True:
                raise AssertionError("same-machine reboot continuity missing")
            if not isinstance(row.get("provider_identity"), str) or not row["provider_identity"].strip():
                raise AssertionError("persistent VM provider identity missing")
            if not isinstance(row.get("pre_boot_marker"), str) or not isinstance(row.get("post_boot_marker"), str):
                raise AssertionError("reboot continuity markers missing")
            if row["pre_boot_marker"] == row["post_boot_marker"]:
                raise AssertionError("reboot markers do not prove a boot boundary")
    return {"source_commit": source, "row_count": row_count, "real_reboot_rows": reboot}


def self_test() -> dict[str, Any]:
    tasks = [{"id": task_id, "status": "DONE"} for task_id in EXPECTED_DONE] + [{"id": "03.25", "status": "READY"}]
    tracker = {"package_id": "PKG-03", "done": 24, "required": 25, "complete": False, "active_task": "03.25", "ready_tasks": ["03.25"], "tasks": tasks}
    handoff: dict[str, Any] = {
        "schema_version": 1, "package_id": "PKG-03", "task_id": "03.24", "mode": "VM_ACCEPTANCE_HANDOFF",
        "source_commit": "d" * 40, "workflow_run_id": 1, "workflow_job_id": 2, "artifact_id": 3,
        "vm_matrix_sha256": "1" * 64, "provenance_handoff_sha256": "2" * 64, "signed_subjects_sha256": "3" * 64,
        "sbom_sha256": "4" * 64, "attestation_bundle_sha256": "5" * 64, "matrix_row_count": 6,
        "all_rows_passed": True, "cleanup_verified": True, "forbidden_system_mutation_zero": True,
        "signatures_reverified": True, "provenance_reverified": True, "production_accepted": True,
        "real_reboot_rows": [{"row_id": "pending-reboot", "required": True, "executed": True, "same_machine_verified": True,
                              "provider_identity": "persistent-vm-fixture", "pre_boot_marker": "boot-a", "post_boot_marker": "boot-b"}],
    }
    validate_tracker(tracker); valid = validate_handoff(handoff)
    negatives = 0
    mutations = (
        ("not-accepted", lambda x: x.__setitem__("production_accepted", False)),
        ("matrix-failed", lambda x: x.__setitem__("all_rows_passed", False)),
        ("bad-matrix-digest", lambda x: x.__setitem__("vm_matrix_sha256", "bad")),
        ("signature-not-reverified", lambda x: x.__setitem__("signatures_reverified", False)),
        ("reboot-not-executed", lambda x: x["real_reboot_rows"][0].__setitem__("executed", False)),
        ("reboot-machine-changed", lambda x: x["real_reboot_rows"][0].__setitem__("same_machine_verified", False)),
        ("secret-field", lambda x: x.__setitem__("oidc_token", "forbidden")),
    )
    for label, mutate in mutations:
        candidate = json.loads(json.dumps(handoff)); mutate(candidate)
        try:
            validate_handoff(candidate)
        except AssertionError:
            negatives += 1
        else:
            raise AssertionError(f"negative unexpectedly passed: {label}")
    blocked = json.loads(json.dumps(tracker)); blocked["active_task"] = "03.24"; blocked["ready_tasks"] = ["03.24"]; blocked["done"] = 23; blocked["tasks"][-2]["status"] = "READY"
    try:
        validate_tracker(blocked)
    except AssertionError:
        negatives += 1
    else:
        raise AssertionError("blocked tracker unexpectedly activated 03.25")
    return {
        "schema_version": 1, "task_id": "03.25", "mode": "NON_ACCEPTANCE_PREIMPLEMENTATION",
        "production_evidence_consumed": False, "canonical_state_changed": False, "implementation_authority": False,
        "normalized_vm_handoff_ready": True, "matrix_row_count": valid["row_count"],
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
            "schema_version": 1, "task_id": "03.25", "mode": "ACTIVATION_PREFLIGHT_VERIFIED",
            "production_evidence_consumed": True, "canonical_state_changed": False, "implementation_authority": False,
            "source_commit": verified["source_commit"], "matrix_row_count": verified["row_count"],
            "reboot_rows_sha256": hashlib.sha256(canonical(verified["real_reboot_rows"])).hexdigest(),
            "ready_for_0325_final_gate": True,
        }
    output = Path(args.output); output.parent.mkdir(parents=True, exist_ok=True); output.write_bytes(canonical(report))
    print(json.dumps(report, sort_keys=True)); print("PKG03_0325_ACTIVATION_PREFLIGHT=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
