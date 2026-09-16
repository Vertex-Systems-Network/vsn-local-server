#!/usr/bin/env python3
"""Fixture-only PKG-04 04.12/04.13 Desktop/CLI update UX contract.

NON_ACCEPTANCE_PREIMPLEMENTATION. The harness records the current absence of update
commands in the Desktop AgentCommand union and CLI dispatcher, then freezes a future
state/action/result contract without modifying product UI, CLI or Agent code.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

MODE = "NON_ACCEPTANCE_PREIMPLEMENTATION"
STATES = (
    "idle",
    "checking",
    "up-to-date",
    "available",
    "downloading",
    "downloaded",
    "awaiting-authorization",
    "applying",
    "restart-required",
    "recovering",
    "rollback-available",
    "rolling-back",
    "completed",
    "error",
)
ACTIONS = ("check", "status", "download", "apply", "rollback")
MUTATING_ACTIONS = {"apply", "rollback"}
CLI_COMMANDS = ("update check", "update status", "update apply", "update rollback")


class UxError(ValueError):
    pass


def canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def source_baseline(repo_root: Path) -> dict:
    cli_path = repo_root / "apps/cli/src/main.rs"
    desktop_contract_path = repo_root / "apps/desktop/src/contracts.ts"
    cli = cli_path.read_text(encoding="utf-8")
    desktop = desktop_contract_path.read_text(encoding="utf-8")
    cli_patterns = (
        'cmd == "update"',
        '"update.check"',
        '"update.status"',
        '"update.apply"',
        '"update.rollback"',
    )
    desktop_patterns = (
        "'update.check'",
        "'update.status'",
        "'update.apply'",
        "'update.rollback'",
    )
    cli_present = [pattern for pattern in cli_patterns if pattern in cli]
    desktop_present = [pattern for pattern in desktop_patterns if pattern in desktop]
    if cli_present or desktop_present:
        raise UxError(
            "current product update command surface already exists; reconcile this preimplementation contract before continuing: "
            + json.dumps({"cli": cli_present, "desktop": desktop_present}, sort_keys=True)
        )
    if "fn dispatch(args: &[String])" not in cli or "call(" not in cli:
        raise UxError("current CLI dispatch-to-Agent baseline not recognized")
    if "export type AgentCommand" not in desktop:
        raise UxError("current Desktop AgentCommand baseline not recognized")
    return {
        "cli_source_sha256": sha(cli.encode("utf-8")),
        "desktop_contract_source_sha256": sha(desktop.encode("utf-8")),
        "cli_update_surface_present": False,
        "desktop_update_agent_commands_present": False,
        "cli_dispatch_to_agent_pattern_present": True,
        "desktop_agent_command_union_present": True,
    }


def transition(state: str, action: str, outcome: str) -> str:
    if state not in STATES or action not in ACTIONS:
        raise UxError("unknown update state/action")
    if action == "status":
        return state
    table = {
        ("idle", "check", "start"): "checking",
        ("checking", "check", "current"): "up-to-date",
        ("checking", "check", "available"): "available",
        ("checking", "check", "failure"): "error",
        ("up-to-date", "check", "start"): "checking",
        ("available", "download", "start"): "downloading",
        ("downloading", "download", "complete"): "downloaded",
        ("downloading", "download", "failure"): "error",
        ("downloaded", "apply", "authorization-required"): "awaiting-authorization",
        ("awaiting-authorization", "apply", "authorized"): "applying",
        ("awaiting-authorization", "apply", "denied"): "downloaded",
        ("applying", "apply", "restart-required"): "restart-required",
        ("applying", "apply", "complete"): "completed",
        ("applying", "apply", "recovering"): "recovering",
        ("recovering", "apply", "restored"): "rollback-available",
        ("recovering", "apply", "failure"): "error",
        ("restart-required", "check", "start"): "checking",
        ("completed", "check", "start"): "checking",
        ("rollback-available", "rollback", "authorization-required"): "awaiting-authorization",
        ("completed", "rollback", "authorization-required"): "awaiting-authorization",
        ("awaiting-authorization", "rollback", "authorized"): "rolling-back",
        ("awaiting-authorization", "rollback", "denied"): "rollback-available",
        ("rolling-back", "rollback", "complete"): "completed",
        ("rolling-back", "rollback", "recovering"): "recovering",
        ("error", "check", "start"): "checking",
    }
    key = (state, action, outcome)
    if key not in table:
        raise UxError(f"invalid update transition: {key}")
    return table[key]


def validate_action_authority(state: str, action: str, authority: dict) -> None:
    if action not in ACTIONS:
        raise UxError("unknown action")
    if action in MUTATING_ACTIONS:
        if authority.get("caller_identity_verified") is not True:
            raise UxError("mutating UX action requires verified caller identity")
        if authority.get("explicit_user_or_policy_authorization") is not True:
            raise UxError("mutating UX action requires explicit authorization")
        if authority.get("helper_authority_verified") is not True:
            raise UxError("mutating UX action requires verified helper authority")
    if action == "apply" and state not in {"downloaded", "awaiting-authorization", "applying", "recovering"}:
        raise UxError("apply action is unavailable in current state")
    if action == "rollback" and state not in {"rollback-available", "completed", "awaiting-authorization", "rolling-back"}:
        raise UxError("rollback action is unavailable in current state")


def cli_result(command: str, *, ok: bool, state: str, error_code: str | None = None) -> tuple[dict, int]:
    if command not in CLI_COMMANDS:
        raise UxError("unknown CLI update command")
    if state not in STATES:
        raise UxError("CLI result state invalid")
    if ok:
        if error_code is not None:
            raise UxError("successful CLI result cannot carry error code")
        code = 0
    else:
        if not isinstance(error_code, str) or not re.fullmatch(r"UPDATE_[A-Z0-9_]{3,64}", error_code):
            raise UxError("failed CLI result requires stable UPDATE_* error code")
        code = 1
    return {
        "schema_version": 1,
        "command": command,
        "ok": ok,
        "state": state,
        "error_code": error_code,
    }, code


def assert_rejected(fn, expected: str) -> str:
    try:
        fn()
    except UxError as exc:
        text = str(exc)
        if expected not in text:
            raise AssertionError((expected, text))
        return text
    raise AssertionError(f"expected rejection containing: {expected}")


def run(repo_root: Path, source_commit: str) -> tuple[dict, dict, list[dict]]:
    if not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        raise UxError("source commit must be lowercase 40-character SHA-1")
    baseline = source_baseline(repo_root)
    matrix: list[dict] = [{"case": "current-product-update-surface-absent", "result": "PASS"}]

    state = "idle"
    state = transition(state, "check", "start")
    state = transition(state, "check", "available")
    state = transition(state, "download", "start")
    state = transition(state, "download", "complete")
    authority = {
        "caller_identity_verified": True,
        "explicit_user_or_policy_authorization": True,
        "helper_authority_verified": True,
    }
    validate_action_authority(state, "apply", authority)
    state = transition(state, "apply", "authorization-required")
    state = transition(state, "apply", "authorized")
    state = transition(state, "apply", "restart-required")
    if state != "restart-required":
        raise AssertionError(state)
    matrix.append({"case": "deterministic-check-download-authorized-apply", "result": "PASS"})

    for label, bad in (
        ("unverified-caller", {**authority, "caller_identity_verified": False}),
        ("missing-explicit-authorization", {**authority, "explicit_user_or_policy_authorization": False}),
        ("unverified-helper-authority", {**authority, "helper_authority_verified": False}),
    ):
        reason = assert_rejected(lambda b=bad: validate_action_authority("downloaded", "apply", b), "requires")
        matrix.append({"case": label, "result": "REJECTED", "reason": reason})

    reason = assert_rejected(lambda: validate_action_authority("idle", "apply", authority), "unavailable")
    matrix.append({"case": "apply-disabled-before-download", "result": "REJECTED", "reason": reason})
    reason = assert_rejected(lambda: validate_action_authority("available", "rollback", authority), "unavailable")
    matrix.append({"case": "rollback-disabled-without-rollback-state", "result": "REJECTED", "reason": reason})
    reason = assert_rejected(lambda: transition("idle", "apply", "complete"), "invalid update transition")
    matrix.append({"case": "invalid-state-transition", "result": "REJECTED", "reason": reason})

    for command in CLI_COMMANDS:
        result, exit_code = cli_result(command, ok=True, state="idle")
        if exit_code != 0 or result["schema_version"] != 1:
            raise AssertionError(command)
    matrix.append({"case": "cli-success-envelope-and-exit-zero", "result": "PASS"})
    failed, exit_code = cli_result("update apply", ok=False, state="error", error_code="UPDATE_AUTHORITY_DENIED")
    if exit_code != 1 or failed["error_code"] != "UPDATE_AUTHORITY_DENIED":
        raise AssertionError(failed)
    matrix.append({"case": "cli-failure-envelope-and-nonzero-exit", "result": "PASS"})
    reason = assert_rejected(
        lambda: cli_result("update apply", ok=False, state="error", error_code="permission denied"),
        "stable UPDATE_*",
    )
    matrix.append({"case": "cli-unstable-error-code", "result": "REJECTED", "reason": reason})

    contract = {
        "schema_version": 1,
        "mode": MODE,
        "task_scope": ["04.12", "04.13"],
        "states": list(STATES),
        "actions": list(ACTIONS),
        "desktop_future_agent_commands": ["update.check", "update.status", "update.download", "update.apply", "update.rollback"],
        "cli_commands": list(CLI_COMMANDS),
        "status_is_read_only": True,
        "check_is_read_only": True,
        "download_does_not_mutate_installed_product": True,
        "apply_and_rollback_require_authority": True,
        "ui_must_not_claim_success_before_coherent_terminal_state": True,
        "ui_must_surface_restart_required": True,
        "ui_must_surface_recovery_state": True,
        "cli_machine_readable_output_required": True,
        "cli_nonzero_exit_on_failure_required": True,
        "product_surface_absent_requires_activation_time_reconciliation": True,
    }
    report = {
        "schema_version": 1,
        "package_id": "PKG-04",
        "task_scope": ["04.12", "04.13"],
        "mode": MODE,
        "source_commit": source_commit,
        "canonical_prerequisite": "04.09:DONE,04.10:DONE,04.11:DONE",
        "canonical_tasks_active": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "production_evidence_consumed": False,
        "product_desktop_mutated": False,
        "product_cli_mutated": False,
        "product_agent_mutated": False,
        "fixture_identity": True,
        "current_baseline": baseline,
        "deterministic_state_machine_frozen": True,
        "mutating_action_authority_required": True,
        "stable_cli_result_contract_required": True,
        "activation_time_product_reconciliation_required": True,
        "test_case_count": len(matrix),
        "negative_rejections": sum(row["result"] == "REJECTED" for row in matrix),
        "result": "PASS",
    }
    return report, contract, matrix


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true", required=True)
    parser.add_argument("--repo-root", default=".")
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--output-dir", required=True)
    args = parser.parse_args()

    report, contract, matrix = run(Path(args.repo_root), args.source_commit)
    out = Path(args.output_dir)
    out.mkdir(parents=True, exist_ok=True)
    (out / "report.json").write_bytes(canonical(report))
    (out / "contract.json").write_bytes(canonical(contract))
    (out / "matrix.json").write_bytes(canonical(matrix))
    print(json.dumps(report, sort_keys=True))
    print("PKG04_UPDATE_UX_PREIMPLEMENTATION=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
