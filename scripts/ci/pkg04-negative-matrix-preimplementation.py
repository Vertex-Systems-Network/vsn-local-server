#!/usr/bin/env python3
"""Fixture-only PKG-04 04.14/04.15 integrated eligibility and failure matrix.

NON_ACCEPTANCE_PREIMPLEMENTATION. No network, product mutation, service control, file
replacement, reboot or rollback occurs. The model composes the previously frozen trust,
download, lock and package-coherence invariants into end-to-end negative outcomes.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

MODE = "NON_ACCEPTANCE_PREIMPLEMENTATION"
COMPONENTS = ("agent", "cli", "desktop", "updater-helper")


class MatrixError(ValueError):
    pass


def canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha(value: str | bytes) -> str:
    if isinstance(value, str):
        value = value.encode("utf-8")
    return hashlib.sha256(value).hexdigest()


def terminal(release: str, *, mutation_started: bool, stage: str, reason: str, recovered: bool = False) -> dict:
    installed = {component: release for component in COMPONENTS}
    return {
        "stage": stage,
        "reason": reason,
        "mutation_started": mutation_started,
        "recovered": recovered,
        "terminal_release_set": sorted(set(installed.values())),
        "installed": installed,
        "mixed_version_terminal": False,
        "mutable_state_preserved": True,
        "restart_allowed": release in {"previous", "next"},
    }


def evaluate(case: str) -> dict:
    pre_mutation = {
        "replay-metadata": ("eligibility", "metadata replay rejected"),
        "downgrade-metadata": ("eligibility", "downgrade rejected"),
        "invalid-metadata-signature": ("eligibility", "metadata signature rejected"),
        "unauthorized-channel-transition": ("eligibility", "channel transition rejected"),
        "offline-discovery": ("discovery", "network unavailable before mutation"),
        "partial-download": ("download", "signed byte count not reached"),
        "corrupt-download": ("download", "artifact SHA-256 mismatch"),
        "tampered-cache": ("cache", "cached artifact identity mismatch"),
        "resume-full-body-200": ("resume", "HTTP 200 cannot append to partial"),
        "resume-range-mismatch": ("resume", "Content-Range identity mismatch"),
        "live-owner-stale-lock": ("lock", "elapsed time cannot recover a live owner lock"),
        "unverified-helper": ("helper-authority", "helper identity not trusted"),
        "unauthorized-caller": ("helper-authority", "caller authority rejected"),
    }
    if case in pre_mutation:
        stage, reason = pre_mutation[case]
        return terminal("previous", mutation_started=False, stage=stage, reason=reason)

    if case.startswith("interrupt-after-"):
        component = case.removeprefix("interrupt-after-")
        if component not in COMPONENTS:
            raise MatrixError("unknown interruption component")
        return terminal(
            "previous",
            mutation_started=True,
            stage="apply-recovery",
            reason=f"interruption after {component}; reverse restoration completed",
            recovered=True,
        )
    if case == "locked-desktop":
        return terminal(
            "previous",
            mutation_started=True,
            stage="apply-recovery",
            reason="locked desktop target caused package restoration",
            recovered=True,
        )
    if case == "crash-after-next-installed-before-commit":
        return terminal(
            "previous",
            mutation_started=True,
            stage="crash-recovery",
            reason="journal recovery restored previous accepted package",
            recovered=True,
        )
    if case == "tampered-rollback-backup":
        return terminal(
            "next",
            mutation_started=False,
            stage="rollback-verification",
            reason="rollback blocked before mutation because previous identity failed",
        )
    if case == "valid-newer-update":
        return terminal(
            "next",
            mutation_started=True,
            stage="commit",
            reason="all synthetic gates satisfied",
        )
    raise MatrixError(f"unknown scenario: {case}")


def validate_outcome(case: str, outcome: dict) -> None:
    release_set = outcome.get("terminal_release_set")
    if release_set not in (["previous"], ["next"]):
        raise MatrixError(f"{case}: terminal package is not one coherent release")
    if outcome.get("mixed_version_terminal") is not False:
        raise MatrixError(f"{case}: mixed-version terminal state projected")
    if outcome.get("mutable_state_preserved") is not True:
        raise MatrixError(f"{case}: mutable state preservation failed")
    if case != "valid-newer-update" and outcome["stage"] in {
        "eligibility", "discovery", "download", "cache", "resume", "lock", "helper-authority", "rollback-verification"
    } and outcome.get("mutation_started") is not False:
        raise MatrixError(f"{case}: pre-mutation failure crossed mutation boundary")
    if outcome.get("mutation_started") and case != "valid-newer-update" and not outcome.get("recovered"):
        raise MatrixError(f"{case}: post-mutation failure did not prove recovery")


def run(source_commit: str) -> tuple[dict, dict, list[dict]]:
    if not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        raise MatrixError("source commit must be lowercase 40-character SHA-1")

    scenarios = [
        "valid-newer-update",
        "replay-metadata",
        "downgrade-metadata",
        "invalid-metadata-signature",
        "unauthorized-channel-transition",
        "offline-discovery",
        "partial-download",
        "corrupt-download",
        "tampered-cache",
        "resume-full-body-200",
        "resume-range-mismatch",
        "live-owner-stale-lock",
        "unverified-helper",
        "unauthorized-caller",
        "locked-desktop",
        "crash-after-next-installed-before-commit",
        "tampered-rollback-backup",
    ] + [f"interrupt-after-{component}" for component in COMPONENTS]

    matrix: list[dict] = []
    for case in scenarios:
        outcome = evaluate(case)
        validate_outcome(case, outcome)
        matrix.append({"case": case, "result": "PASS", "outcome": outcome})

    pre_mutation_rejections = sum(
        row["outcome"]["mutation_started"] is False and row["case"] != "tampered-rollback-backup"
        for row in matrix
        if row["case"] != "valid-newer-update"
    )
    recovered_post_mutation = sum(
        row["outcome"]["recovered"] is True for row in matrix
    )
    coherent_terminal_count = sum(
        row["outcome"]["terminal_release_set"] in (["previous"], ["next"]) for row in matrix
    )

    report = {
        "schema_version": 1,
        "package_id": "PKG-04",
        "task_scope": ["04.14", "04.15"],
        "mode": MODE,
        "source_commit": source_commit,
        "canonical_prerequisite": "04.09:DONE,04.10:DONE,04.11:DONE,04.14:DONE-for-04.15",
        "canonical_tasks_active": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "production_evidence_consumed": False,
        "product_updater_mutated": False,
        "network_access_performed": False,
        "real_update_executed": False,
        "real_rollback_executed": False,
        "fixture_identity": True,
        "scenario_count": len(matrix),
        "coherent_terminal_count": coherent_terminal_count,
        "pre_mutation_rejection_count": pre_mutation_rejections,
        "post_mutation_recovery_count": recovered_post_mutation,
        "replay_downgrade_invalid_metadata_rejected_before_mutation": True,
        "offline_partial_corrupt_tampered_fail_closed": True,
        "resume_append_safety_preserved": True,
        "live_owner_lock_recovery_forbidden": True,
        "helper_authority_required_before_mutation": True,
        "component_interruption_recovery_required": True,
        "rollback_identity_verified_before_mutation": True,
        "mixed_version_terminal_forbidden": True,
        "mutable_state_preservation_required": True,
        "result": "PASS",
    }
    policy = {
        "schema_version": 1,
        "mode": MODE,
        "fixture_only": True,
        "scenario_ids_sha256": sha("\n".join(scenarios)),
        "eligibility_failures_stop_before_download_or_mutation": True,
        "download_failures_stop_before_quiesce_or_mutation": True,
        "authority_failures_stop_before_mutation": True,
        "post_mutation_failures_require_previous-release_recovery": True,
        "rollback_verification_failure_preserves_current-accepted-release": True,
        "terminal_release_cardinality": 1,
    }
    return report, policy, matrix


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--output-dir", required=True)
    args = parser.parse_args()

    report, policy, matrix = run(args.source_commit)
    out = Path(args.output_dir)
    out.mkdir(parents=True, exist_ok=True)
    (out / "report.json").write_bytes(canonical(report))
    (out / "policy.json").write_bytes(canonical(policy))
    (out / "matrix.json").write_bytes(canonical(matrix))
    print(json.dumps(report, sort_keys=True))
    print("PKG04_NEGATIVE_MATRIX_PREIMPLEMENTATION=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
