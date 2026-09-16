#!/usr/bin/env python3
"""Deterministic non-acceptance PKG-04 updater/recovery contract model.

This is intentionally a preimplementation harness. It does not mutate product updater
code, perform a real update, activate PKG-04, or project any canonical acceptance.
It machine-checks the P0 transaction, lock ownership, rollback identity, containment,
and Windows durability invariants that activation must preserve.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path, PurePosixPath

MODE = "NON_ACCEPTANCE_PREIMPLEMENTATION"
PHASES = (
    "prepared",
    "old-target-staged",
    "new-target-installed",
    "state-committed",
    "rollback-started",
    "rollback-committed",
)


class ContractError(ValueError):
    pass


def canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def digest(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def safe_relative_target(value: str) -> str:
    if not value or value.startswith(("/", "\\")) or re.match(r"^[A-Za-z]:", value):
        raise ContractError("target must be a non-empty relative path")
    normalized = value.replace("\\", "/")
    path = PurePosixPath(normalized)
    if any(part in {"", ".", ".."} for part in path.parts):
        raise ContractError("target contains an unsafe path component")
    return str(path)


def require_containment(target: str, *, parent_verified: bool, reparse_verified: bool) -> str:
    normalized = safe_relative_target(target)
    if not parent_verified:
        raise ContractError("canonical parent containment proof is required")
    if not reparse_verified:
        raise ContractError("symlink/reparse containment proof is required")
    return normalized


def make_lock(owner_token: str, *, lease_expired: bool, owner_liveness: str) -> dict:
    if len(owner_token) < 32:
        raise ContractError("owner token must be collision resistant")
    if owner_liveness not in {"live", "dead", "unknown"}:
        raise ContractError("invalid owner liveness")
    return {
        "owner_token": owner_token,
        "lease_expired": lease_expired,
        "owner_liveness": owner_liveness,
    }


def release_lock(lock: dict, presented_owner_token: str) -> bool:
    if lock["owner_token"] != presented_owner_token:
        raise ContractError("lock release denied: owner token mismatch")
    return True


def recover_lock(lock: dict, *, explicit_confirmation: bool) -> bool:
    if not explicit_confirmation:
        raise ContractError("stale lock recovery requires explicit confirmation")
    if not lock["lease_expired"]:
        raise ContractError("lock lease is not expired")
    if lock["owner_liveness"] != "dead":
        raise ContractError("elapsed time alone is insufficient; dead-owner proof is required")
    return True


def verify_backup(payload: bytes, *, expected_sha256: str, expected_size: int) -> bool:
    if len(payload) != expected_size:
        raise ContractError("rollback backup byte count mismatch")
    if digest(payload) != expected_sha256:
        raise ContractError("rollback backup digest mismatch")
    return True


def recovery_action(
    phase: str,
    *,
    previous_verified: bool,
    next_verified: bool,
    state_matches_next: bool,
    containment_verified: bool,
) -> str:
    if phase not in PHASES:
        raise ContractError("unknown journal phase")
    if not containment_verified:
        raise ContractError("recovery requires target containment proof")
    if phase == "prepared":
        return "discard-pending"
    if phase == "old-target-staged":
        if not previous_verified:
            raise ContractError("cannot restore unverified previous target")
        return "restore-previous"
    if phase == "new-target-installed":
        if next_verified and previous_verified:
            return "commit-state"
        if previous_verified:
            return "rollback-to-previous"
        raise ContractError("neither a verified next state nor verified rollback point is available")
    if phase == "state-committed":
        if not next_verified or not state_matches_next:
            raise ContractError("committed state does not match installed target")
        return "no-op-committed"
    if phase == "rollback-started":
        if not previous_verified:
            raise ContractError("rollback continuation requires verified previous target")
        return "complete-rollback"
    if phase == "rollback-committed":
        if not previous_verified:
            raise ContractError("rollback-committed target is not verified")
        return "no-op-rollback-committed"
    raise AssertionError("unreachable")


def assert_raises(fn, contains: str) -> str:
    try:
        fn()
    except ContractError as exc:
        text = str(exc)
        assert contains in text, (contains, text)
        return text
    raise AssertionError(f"expected ContractError containing: {contains}")


def run_self_test(source_commit: str) -> tuple[dict, list[dict]]:
    if not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        raise ContractError("source commit must be a lowercase 40-character SHA-1")

    previous = b"vsn-agent previous accepted bytes\n"
    next_payload = b"vsn-agent next candidate bytes\n"
    previous_sha = digest(previous)
    next_sha = digest(next_payload)
    owner_a = digest(b"synthetic-owner-A")
    owner_b = digest(b"synthetic-owner-B")

    transaction = {
        "schema_version": 1,
        "transaction_id": digest(b"pkg04-fast-epoch-transaction")[:32],
        "component_id": "agent",
        "target_relative": "bin/vsn-agent.exe",
        "previous": {
            "release": "0.38.1",
            "sha256": previous_sha,
            "bytes": len(previous),
        },
        "next": {
            "release": "0.38.2-fixture",
            "sha256": next_sha,
            "bytes": len(next_payload),
        },
        "journal_phases": list(PHASES),
        "owner_token_required": True,
        "backup_identity_required_before_restore": True,
        "canonical_parent_containment_required": True,
        "symlink_reparse_containment_required": True,
        "windows_metadata_durability_proof_required": True,
        "windows_acl_owner_inheritance_proof_required": True,
    }

    matrix: list[dict] = []

    target = require_containment(
        transaction["target_relative"], parent_verified=True, reparse_verified=True
    )
    matrix.append({"case": "safe-target", "result": "PASS", "target": target})
    for case, bad in (
        ("parent-traversal", "../vsn-agent.exe"),
        ("windows-drive", "C:\\Program Files\\VSN\\vsn-agent.exe"),
        ("rooted", "/opt/vsn/vsn-agent"),
        ("unc", "\\\\server\\share\\vsn-agent.exe"),
    ):
        error = assert_raises(lambda bad=bad: safe_relative_target(bad), "relative path")
        matrix.append({"case": case, "result": "REJECTED", "reason": error})
    error = assert_raises(
        lambda: require_containment("bin/vsn-agent.exe", parent_verified=True, reparse_verified=False),
        "reparse containment proof",
    )
    matrix.append({"case": "reparse-proof-missing", "result": "REJECTED", "reason": error})

    live_lock = make_lock(owner_a, lease_expired=True, owner_liveness="live")
    assert release_lock(live_lock, owner_a)
    matrix.append({"case": "owner-release", "result": "PASS"})
    replacement_lock = make_lock(owner_b, lease_expired=False, owner_liveness="live")
    error = assert_raises(lambda: release_lock(replacement_lock, owner_a), "owner token mismatch")
    matrix.append({"case": "old-guard-cannot-delete-replacement", "result": "REJECTED", "reason": error})
    error = assert_raises(
        lambda: recover_lock(live_lock, explicit_confirmation=True), "dead-owner proof"
    )
    matrix.append({"case": "age-alone-cannot-recover-live-lock", "result": "REJECTED", "reason": error})
    unknown_lock = make_lock(owner_a, lease_expired=True, owner_liveness="unknown")
    error = assert_raises(
        lambda: recover_lock(unknown_lock, explicit_confirmation=True), "dead-owner proof"
    )
    matrix.append({"case": "unknown-owner-cannot-be-recovered", "result": "REJECTED", "reason": error})
    dead_lock = make_lock(owner_a, lease_expired=True, owner_liveness="dead")
    assert recover_lock(dead_lock, explicit_confirmation=True)
    matrix.append({"case": "expired-dead-owner-recovery", "result": "PASS"})

    assert verify_backup(previous, expected_sha256=previous_sha, expected_size=len(previous))
    matrix.append({"case": "rollback-backup-identity", "result": "PASS"})
    tampered = previous + b"tamper"
    error = assert_raises(
        lambda: verify_backup(tampered, expected_sha256=previous_sha, expected_size=len(previous)),
        "byte count mismatch",
    )
    matrix.append({"case": "tampered-rollback-backup", "result": "REJECTED", "reason": error})

    expected_actions = {
        "prepared": "discard-pending",
        "old-target-staged": "restore-previous",
        "new-target-installed": "commit-state",
        "state-committed": "no-op-committed",
        "rollback-started": "complete-rollback",
        "rollback-committed": "no-op-rollback-committed",
    }
    for phase, expected in expected_actions.items():
        action = recovery_action(
            phase,
            previous_verified=True,
            next_verified=True,
            state_matches_next=True,
            containment_verified=True,
        )
        assert action == expected
        matrix.append({"case": f"journal-{phase}", "result": "PASS", "action": action})

    error = assert_raises(
        lambda: recovery_action(
            "old-target-staged",
            previous_verified=False,
            next_verified=True,
            state_matches_next=False,
            containment_verified=True,
        ),
        "unverified previous target",
    )
    matrix.append({"case": "journal-rejects-unverified-rollback", "result": "REJECTED", "reason": error})
    error = assert_raises(
        lambda: recovery_action(
            "state-committed",
            previous_verified=True,
            next_verified=False,
            state_matches_next=False,
            containment_verified=True,
        ),
        "does not match installed target",
    )
    matrix.append({"case": "journal-rejects-state-target-divergence", "result": "REJECTED", "reason": error})

    assert transaction["windows_metadata_durability_proof_required"] is True
    assert transaction["windows_acl_owner_inheritance_proof_required"] is True
    matrix.append({"case": "windows-durability-contract", "result": "PASS", "real_windows_io_executed": False})

    report = {
        "schema_version": 1,
        "package_id": "PKG-04",
        "mode": MODE,
        "source_commit": source_commit,
        "canonical_prerequisite": "PKG-03:COMPLETE",
        "canonical_package_active": False,
        "canonical_task_0401_active": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "production_evidence_consumed": False,
        "real_update_executed": False,
        "real_rollback_executed": False,
        "real_windows_durability_certified": False,
        "transaction_phase_count": len(PHASES),
        "test_case_count": len(matrix),
        "rejected_negative_case_count": sum(1 for row in matrix if row["result"] == "REJECTED"),
        "owner_checked_lock_release": True,
        "elapsed_time_alone_recovery_forbidden": True,
        "rollback_backup_identity_bound": True,
        "reparse_containment_required": True,
        "windows_durability_proof_required": True,
        "deterministic_contract": True,
        "result": "PASS",
    }
    return {"transaction": transaction, "report": report}, matrix


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--output-dir", required=True)
    args = parser.parse_args()

    bundle, matrix = run_self_test(args.source_commit)
    out = Path(args.output_dir)
    out.mkdir(parents=True, exist_ok=True)
    (out / "contract.json").write_bytes(canonical(bundle["transaction"]))
    (out / "matrix.json").write_bytes(canonical(matrix))
    (out / "report.json").write_bytes(canonical(bundle["report"]))
    print(json.dumps(bundle["report"], sort_keys=True))
    print("PKG04_UPDATER_RECOVERY_PREIMPLEMENTATION=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
