#!/usr/bin/env python3
"""Validate dormant PKG-05 Linux/macOS secure-store lifecycle evidence.

NON_ACCEPTANCE_PREIMPLEMENTATION only. The contract consumes ephemeral hosted-runner
credential-loss simulations and current-source structure. It does not prove persistent
same-machine reboot behavior, update/reinstall credential preservation, production
Linux/macOS support, or any canonical PKG-05 task completion.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

MODE = "NON_ACCEPTANCE_PREIMPLEMENTATION"
PACKAGE_ID = "PKG-05"
SCOPE = "PKG-05-DORMANT-SECURE-STORE"
CANONICAL_BLOCKER = "PKG-04:COMPLETE"
ALLOWED_RUNNERS = {
    ("Linux", "X64", "keyring-3.6.3-linux-native"),
    ("macOS", "X64", "keyring-3.6.3-apple-native"),
    ("macOS", "ARM64", "keyring-3.6.3-apple-native"),
}


class SecureStoreError(ValueError):
    pass


def canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def require_patterns(text: str, patterns: tuple[str, ...], label: str) -> None:
    missing = [pattern for pattern in patterns if pattern not in text]
    if missing:
        raise SecureStoreError(f"{label} source baseline drifted; missing patterns: {missing}")


def source_baseline(repo_root: Path) -> dict:
    cargo_path = repo_root / "crates/vsn-security/Cargo.toml"
    security_path = repo_root / "crates/vsn-security/src/lib.rs"
    tracker_path = repo_root / "docs/MASTER-EXECUTION-STATUS.json"
    toolchain_path = repo_root / "rust-toolchain.toml"

    cargo = cargo_path.read_text(encoding="utf-8")
    security = security_path.read_text(encoding="utf-8")
    toolchain = toolchain_path.read_text(encoding="utf-8")

    require_patterns(
        cargo,
        (
            'keyring = { version = "3.6.3", features = ["windows-native"] }',
            'keyring = { version = "3.6.3", features = ["apple-native"] }',
            'keyring = { version = "3.6.3", features = ["linux-native"] }',
        ),
        "secure-store dependency",
    )
    require_patterns(toolchain, ('channel = "1.97.1"',), "Rust toolchain")
    require_patterns(
        security,
        (
            'const KEYRING_SERVICE: &str = "vsn-agent";',
            'const DEVICE_KEY_ENTRY: &str = "device-ed25519-v1";',
            'const IPC_KEY_ENTRY: &str = "local-ipc-hmac-v1";',
            'let secret = load_or_create_secret(DEVICE_KEY_ENTRY, 32)?;',
            'return Err(SecurityError::IdentityMismatch);',
            'load_or_create_secret(IPC_KEY_ENTRY, 32)',
            'Err(keyring::Error::NoEntry) => {',
            'entry\n                .set_password(&B64.encode(&secret))',
            'Ok(data_dir()?.join("security").join("device.json"))',
            'ProjectDirs::from("dev", "VSN", "VSN Platform")',
        ),
        "secure-store implementation",
    )

    tracker = json.loads(tracker_path.read_text(encoding="utf-8"))
    pkg05 = next((row for row in tracker.get("packages", []) if row.get("id") == PACKAGE_ID), None)
    if pkg05 != {
        "id": "PKG-05",
        "name": "Linux + macOS Release",
        "done": 0,
        "required": 23,
        "percent": 0.0,
        "status": "NOT_STARTED",
    }:
        raise SecureStoreError("canonical PKG-05 dormant tracker baseline changed")
    if tracker.get("active_package") != "PKG-03" or tracker.get("active_task") != "03.22":
        raise SecureStoreError("canonical active package/task changed; dormant lane must reconcile")

    return {
        "rust_toolchain": "1.97.1",
        "keyring_version": "3.6.3",
        "linux_backend": "linux-native",
        "macos_backend": "apple-native",
        "windows_backend": "windows-native",
        "keyring_service": "vsn-agent",
        "device_entry": "device-ed25519-v1",
        "ipc_entry_non_windows": "local-ipc-hmac-v1",
        "device_metadata": "ProjectDirs.data_local_dir/security/device.json",
        "missing_device_credential_generates_replacement_before_metadata_check": True,
        "persisted_metadata_mismatch_fails_closed": True,
        "missing_non_windows_ipc_credential_generates_replacement": True,
        "pkg05_required_task_count": 23,
        "pkg05_status": "NOT_STARTED",
        "cargo_sha256": sha256_bytes(cargo_path.read_bytes()),
        "security_source_sha256": sha256_bytes(security_path.read_bytes()),
        "tracker_sha256": sha256_bytes(tracker_path.read_bytes()),
    }


def validate_runtime_evidence(evidence: dict, source_commit: str) -> None:
    if evidence.get("schema_version") != 1:
        raise SecureStoreError("runtime evidence schema mismatch")
    if evidence.get("mode") != MODE or evidence.get("scope") != SCOPE:
        raise SecureStoreError("runtime evidence mode/scope mismatch")
    if evidence.get("source_commit") != source_commit:
        raise SecureStoreError("runtime evidence source commit mismatch")
    identity = (evidence.get("runner_os"), evidence.get("runner_arch"), evidence.get("credential_backend"))
    if identity not in ALLOWED_RUNNERS:
        raise SecureStoreError(f"unexpected runner/backend identity: {identity}")
    required_true = (
        "ephemeral_runner_execution",
        "credential_loss_simulated",
        "initial_device_identity_created",
        "initial_ipc_authenticator_created",
        "identity_mismatch_after_device_credential_loss",
        "ipc_secret_rotated_after_credential_loss",
        "secret_values_logged_false",
        "cleanup_pass",
    )
    for field in required_true:
        if evidence.get(field) is not True:
            raise SecureStoreError(f"runtime evidence missing required proof: {field}")
    required_false = (
        "persistent_same_machine_reboot_proven",
        "update_reinstall_credential_preservation_proven",
        "explicit_identity_recovery_reenrollment_proven",
        "production_support_claimed",
        "production_evidence",
    )
    for field in required_false:
        if evidence.get(field) is not False:
            raise SecureStoreError(f"runtime evidence falsely projected proof: {field}")


def validate_contract(contract: dict) -> None:
    if contract.get("canonical_blocker") != CANONICAL_BLOCKER:
        raise SecureStoreError("canonical blocker drifted")
    if contract.get("canonical_task_ids_assigned") is not False:
        raise SecureStoreError("dormant secure-store lane must not assign canonical task IDs")
    if contract.get("production_support_claimed") is not False:
        raise SecureStoreError("dormant secure-store lane must not claim production support")
    if contract.get("plaintext_fallback_allowed") is not False:
        raise SecureStoreError("plaintext secret fallback must remain forbidden")
    if contract.get("silent_device_identity_reset_allowed") is not False:
        raise SecureStoreError("silent device identity reset must remain forbidden")
    if contract.get("credential_preservation_or_explicit_recovery_required") is not True:
        raise SecureStoreError("release must preserve credentials or prove explicit identity recovery")
    if contract.get("persistent_reboot_acceptance_required") is not True:
        raise SecureStoreError("persistent same-machine reboot acceptance must remain required")
    if contract.get("linux_secret_service_session_acceptance_required") is not True:
        raise SecureStoreError("Linux Secret Service session acceptance must remain required")
    if contract.get("macos_keychain_access_acceptance_required") is not True:
        raise SecureStoreError("macOS Keychain access acceptance must remain required")
    if contract.get("uninstall_credential_policy_must_be_explicit") is not True:
        raise SecureStoreError("uninstall credential policy must be explicit")
    if contract.get("future_canonical_task_reconciliation_required") is not True:
        raise SecureStoreError("future canonical task reconciliation must remain required")


def assert_rejected(fn, expected: str) -> str:
    try:
        fn()
    except SecureStoreError as exc:
        text = str(exc)
        if expected not in text:
            raise AssertionError((expected, text))
        return text
    raise AssertionError(f"expected rejection containing: {expected}")


def run(repo_root: Path, source_commit: str, evidence_path: Path) -> tuple[dict, dict, list[dict]]:
    if not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        raise SecureStoreError("source commit must be lowercase 40-character git SHA")
    baseline = source_baseline(repo_root)
    evidence = json.loads(evidence_path.read_text(encoding="utf-8"))
    validate_runtime_evidence(evidence, source_commit)

    contract = {
        "schema_version": 1,
        "mode": MODE,
        "package_id": PACKAGE_ID,
        "scope": SCOPE,
        "canonical_blocker": CANONICAL_BLOCKER,
        "canonical_task_definitions_frozen": False,
        "canonical_task_ids_assigned": False,
        "future_canonical_task_reconciliation_required": True,
        "production_support_claimed": False,
        "plaintext_fallback_allowed": False,
        "silent_device_identity_reset_allowed": False,
        "credential_preservation_or_explicit_recovery_required": True,
        "persistent_reboot_acceptance_required": True,
        "linux_secret_service_session_acceptance_required": True,
        "macos_keychain_access_acceptance_required": True,
        "uninstall_credential_policy_must_be_explicit": True,
        "update_reinstall_must_not_silently_change_device_identity": True,
        "ipc_credential_rotation_requires_session_reauthentication": True,
        "credential_and_metadata_coherence_required": True,
        "ephemeral_credential_loss_simulation_is_not_release_acceptance": True,
    }
    validate_contract(contract)

    matrix: list[dict] = [
        {"case": "current-source-secure-store-baseline", "result": "PASS"},
        {"case": "ephemeral-runner-credential-loss-simulation", "result": "PASS"},
        {"case": "fail-closed-device-identity-mismatch", "result": "PASS"},
        {"case": "non-windows-ipc-secret-rotation", "result": "PASS"},
    ]
    bad_evidence = dict(evidence)
    bad_evidence["persistent_same_machine_reboot_proven"] = True
    matrix.append({"case": "reject-reboot-projection", "result": "REJECTED", "reason": assert_rejected(lambda: validate_runtime_evidence(bad_evidence, source_commit), "persistent_same_machine_reboot_proven")})
    bad_evidence = dict(evidence)
    bad_evidence["production_support_claimed"] = True
    matrix.append({"case": "reject-production-support-projection", "result": "REJECTED", "reason": assert_rejected(lambda: validate_runtime_evidence(bad_evidence, source_commit), "production_support_claimed")})
    bad_evidence = dict(evidence)
    bad_evidence["secret_values_logged_false"] = False
    matrix.append({"case": "reject-secret-logging", "result": "REJECTED", "reason": assert_rejected(lambda: validate_runtime_evidence(bad_evidence, source_commit), "secret_values_logged_false")})
    bad_contract = dict(contract)
    bad_contract["plaintext_fallback_allowed"] = True
    matrix.append({"case": "reject-plaintext-fallback", "result": "REJECTED", "reason": assert_rejected(lambda: validate_contract(bad_contract), "plaintext")})
    bad_contract = dict(contract)
    bad_contract["silent_device_identity_reset_allowed"] = True
    matrix.append({"case": "reject-silent-identity-reset", "result": "REJECTED", "reason": assert_rejected(lambda: validate_contract(bad_contract), "silent device identity")})
    bad_contract = dict(contract)
    bad_contract["credential_preservation_or_explicit_recovery_required"] = False
    matrix.append({"case": "reject-no-recovery-contract", "result": "REJECTED", "reason": assert_rejected(lambda: validate_contract(bad_contract), "preserve credentials")})
    bad_contract = dict(contract)
    bad_contract["persistent_reboot_acceptance_required"] = False
    matrix.append({"case": "reject-no-persistent-reboot-proof", "result": "REJECTED", "reason": assert_rejected(lambda: validate_contract(bad_contract), "reboot acceptance")})
    bad_contract = dict(contract)
    bad_contract["canonical_task_ids_assigned"] = True
    matrix.append({"case": "reject-fabricated-task-id", "result": "REJECTED", "reason": assert_rejected(lambda: validate_contract(bad_contract), "canonical task IDs")})

    report = {
        "schema_version": 1,
        "mode": MODE,
        "package_id": PACKAGE_ID,
        "scope": SCOPE,
        "source_commit": source_commit,
        "canonical_blocker": CANONICAL_BLOCKER,
        "canonical_tasks_active": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "production_evidence_consumed": False,
        "production_support_claimed": False,
        "fixture_identity": True,
        "ephemeral_runner_execution": True,
        "persistent_same_machine_reboot_proven": False,
        "update_reinstall_credential_preservation_proven": False,
        "explicit_identity_recovery_reenrollment_proven": False,
        "canonical_task_definitions_frozen": False,
        "canonical_task_ids_assigned": False,
        "future_canonical_task_reconciliation_required": True,
        "runner_os": evidence["runner_os"],
        "runner_arch": evidence["runner_arch"],
        "credential_backend": evidence["credential_backend"],
        "credential_loss_simulation_pass": True,
        "identity_mismatch_after_device_credential_loss": True,
        "ipc_secret_rotated_after_credential_loss": True,
        "source_baseline": baseline,
        "test_case_count": len(matrix),
        "negative_rejections": sum(row["result"] == "REJECTED" for row in matrix),
        "result": "PASS",
    }
    return report, contract, matrix


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", default=".")
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--runtime-evidence", required=True)
    parser.add_argument("--output-dir", required=True)
    args = parser.parse_args()

    report, contract, matrix = run(Path(args.repo_root), args.source_commit, Path(args.runtime_evidence))
    out = Path(args.output_dir)
    out.mkdir(parents=True, exist_ok=True)
    (out / "report.json").write_bytes(canonical(report))
    (out / "contract.json").write_bytes(canonical(contract))
    (out / "matrix.json").write_bytes(canonical(matrix))
    print(json.dumps(report, sort_keys=True))
    print("PKG05_SECURE_STORE_PREIMPLEMENTATION=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
