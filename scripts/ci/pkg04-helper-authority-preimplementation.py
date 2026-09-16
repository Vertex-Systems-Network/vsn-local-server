#!/usr/bin/env python3
"""Fixture-only PKG-04 updater-helper authority/bootstrap contract.

NON_ACCEPTANCE_PREIMPLEMENTATION only. This harness does not modify the real updater
helper, elevate privileges, install a helper, verify a production signer, or activate
PKG-04. It freezes fail-closed requirements for future 04.05/04.11 reconciliation.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

MODE = "NON_ACCEPTANCE_PREIMPLEMENTATION"
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
SAFE_ID_RE = re.compile(r"^[A-Za-z0-9._:-]{1,128}$")
MUTATING_OPERATIONS = {"apply", "rollback", "recover_lock"}
READ_ONLY_OPERATIONS = {"status"}
ALL_OPERATIONS = MUTATING_OPERATIONS | READ_ONLY_OPERATIONS
STABLE_ERROR_CODES = {
    "INPUT_INVALID",
    "CALLER_UNVERIFIED",
    "CALLER_NOT_AUTHORIZED",
    "PRIVILEGE_BOUNDARY_UNVERIFIED",
    "HELPER_IDENTITY_UNVERIFIED",
    "INSTALL_ROOT_UNVERIFIED",
    "CORE_OPERATION_FAILED",
}


class AuthorityError(ValueError):
    pass


def canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha(value: str | bytes) -> str:
    if isinstance(value, str):
        value = value.encode("utf-8")
    return hashlib.sha256(value).hexdigest()


def require_safe_id(value: object, label: str) -> str:
    if not isinstance(value, str) or not SAFE_ID_RE.fullmatch(value):
        raise AuthorityError(f"{label} identity is invalid")
    return value


def validate_bootstrap(record: dict) -> None:
    require_safe_id(record.get("helper_component_id"), "helper component")
    digest = record.get("helper_binary_sha256")
    if not isinstance(digest, str) or not SHA256_RE.fullmatch(digest):
        raise AuthorityError("helper binary digest is invalid")
    if record.get("helper_signature_verified") is not True:
        raise AuthorityError("helper code signature is not verified")
    require_safe_id(record.get("verified_signer_id"), "helper signer")
    if record.get("path_identity_verified") is not True:
        raise AuthorityError("helper path identity is not verified")
    if record.get("parent_writable_by_untrusted_user") is not False:
        raise AuthorityError("helper parent is writable by an untrusted user")
    if record.get("path_reparse_free") is not True:
        raise AuthorityError("helper path crosses a symlink/reparse boundary")
    if record.get("owner_acl_verified") is not True:
        raise AuthorityError("helper owner/ACL contract is not verified")
    if record.get("bootstrap_lineage_verified") is not True:
        raise AuthorityError("helper bootstrap lineage is not verified")
    if record.get("self_replacement_while_running") is not False:
        raise AuthorityError("running helper must not directly self-replace")
    if record.get("replacement_authority") != "external_trusted_bootstrap":
        raise AuthorityError("helper replacement requires external trusted bootstrap authority")


def validate_invocation(request: dict, policy: dict, bootstrap: dict) -> None:
    if request.get("schema_version") != 1:
        raise AuthorityError("request schema is invalid")
    operation = request.get("operation")
    if operation not in ALL_OPERATIONS:
        raise AuthorityError("operation is not recognized")
    require_safe_id(request.get("request_id"), "request")
    require_safe_id(request.get("caller_component_id"), "caller component")
    caller_digest = request.get("caller_binary_sha256")
    if not isinstance(caller_digest, str) or not SHA256_RE.fullmatch(caller_digest):
        raise AuthorityError("caller binary digest is invalid")
    if request.get("caller_identity_verified") is not True:
        raise AuthorityError("caller process identity is not verified")
    if request.get("caller_signature_verified") is not True:
        raise AuthorityError("caller code signature is not verified")
    if request.get("secure_launch_channel_verified") is not True:
        raise AuthorityError("secure launch channel is not verified")
    if request.get("install_root_identity_verified") is not True:
        raise AuthorityError("install root identity is not verified")
    if request["caller_component_id"] not in policy["allowed_callers"]:
        raise AuthorityError("caller component is not authorized")
    if operation not in policy["allowed_operations_by_caller"][request["caller_component_id"]]:
        raise AuthorityError("caller is not authorized for operation")
    if operation in MUTATING_OPERATIONS:
        if request.get("privileged_context_verified") is not True:
            raise AuthorityError("privilege boundary is not verified")
        if request.get("explicit_operator_or_policy_authorization") is not True:
            raise AuthorityError("mutating operation lacks explicit authorization")
    if request.get("helper_binary_sha256") != bootstrap["helper_binary_sha256"]:
        raise AuthorityError("invocation helper identity does not match verified bootstrap")


def validate_response(response: dict, *, expect_success: bool) -> None:
    if response.get("schema_version") != 1:
        raise AuthorityError("response schema is invalid")
    if response.get("ok") is not expect_success:
        raise AuthorityError("response success marker is inconsistent")
    exit_code = response.get("exit_code")
    if not isinstance(exit_code, int):
        raise AuthorityError("response exit code is missing")
    if expect_success:
        if exit_code != 0 or response.get("error_code") is not None:
            raise AuthorityError("successful response must have exit code zero and no error code")
    else:
        if exit_code == 0:
            raise AuthorityError("failed response must retain non-zero process exit status")
        if response.get("error_code") not in STABLE_ERROR_CODES:
            raise AuthorityError("failed response requires a stable error code")
        error_message = response.get("error_message")
        if not isinstance(error_message, str) or len(error_message) > 512:
            raise AuthorityError("failed response error message is invalid or unbounded")
        lowered = error_message.lower()
        for forbidden in ("password=", "token=", "private key", "pfx password"):
            if forbidden in lowered:
                raise AuthorityError("failed response leaks secret-like material")


def assert_rejected(fn, expected: str) -> str:
    try:
        fn()
    except AuthorityError as exc:
        text = str(exc)
        if expected not in text:
            raise AssertionError((expected, text))
        return text
    raise AssertionError(f"expected rejection containing: {expected}")


def run(source_commit: str) -> tuple[dict, dict, list[dict]]:
    if not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        raise AuthorityError("source commit must be lowercase 40-character SHA-1")

    bootstrap = {
        "helper_component_id": "vsn-updater-helper-fixture",
        "helper_binary_sha256": sha(b"fixture updater helper bytes"),
        "helper_signature_verified": True,
        "verified_signer_id": "fixture-code-signing-root",
        "path_identity_verified": True,
        "parent_writable_by_untrusted_user": False,
        "path_reparse_free": True,
        "owner_acl_verified": True,
        "bootstrap_lineage_verified": True,
        "self_replacement_while_running": False,
        "replacement_authority": "external_trusted_bootstrap",
    }
    validate_bootstrap(bootstrap)

    policy = {
        "allowed_callers": {"vsn-agent-fixture"},
        "allowed_operations_by_caller": {
            "vsn-agent-fixture": set(ALL_OPERATIONS),
        },
    }
    serializable_policy = {
        "schema_version": 1,
        "mode": MODE,
        "fixture_only": True,
        "production_policy_frozen": False,
        "production_signer_present": False,
        "production_helper_hash_present": False,
        "raw_stdin_is_authorization_boundary": False,
        "caller_process_identity_required": True,
        "caller_binary_identity_required": True,
        "caller_signature_verification_required": True,
        "secure_launch_channel_required": True,
        "install_root_identity_required": True,
        "mutating_operations_require_verified_privilege": True,
        "mutating_operations_require_explicit_authorization": True,
        "helper_bootstrap_signature_hash_acl_path_lineage_required": True,
        "helper_direct_self_replacement_forbidden": True,
        "structured_failure_response_required": True,
        "nonzero_exit_status_on_failure_required": True,
        "stable_error_code_set": sorted(STABLE_ERROR_CODES),
        "fixture_allowed_callers": sorted(policy["allowed_callers"]),
        "fixture_allowed_operations": sorted(ALL_OPERATIONS),
    }

    request = {
        "schema_version": 1,
        "operation": "apply",
        "request_id": "fixture-request-0001",
        "caller_component_id": "vsn-agent-fixture",
        "caller_binary_sha256": sha(b"fixture agent bytes"),
        "caller_identity_verified": True,
        "caller_signature_verified": True,
        "secure_launch_channel_verified": True,
        "install_root_identity_verified": True,
        "privileged_context_verified": True,
        "explicit_operator_or_policy_authorization": True,
        "helper_binary_sha256": bootstrap["helper_binary_sha256"],
    }
    validate_invocation(request, policy, bootstrap)
    matrix: list[dict] = [{"case": "verified-mutating-invocation", "result": "PASS"}]

    negatives = (
        ("raw-stdin-without-caller-verification", "caller_identity_verified", False, "caller process identity"),
        ("unsigned-caller", "caller_signature_verified", False, "caller code signature"),
        ("unbound-launch-channel", "secure_launch_channel_verified", False, "secure launch channel"),
        ("unverified-install-root", "install_root_identity_verified", False, "install root identity"),
        ("missing-privilege-proof", "privileged_context_verified", False, "privilege boundary"),
        ("missing-operator-policy-authorization", "explicit_operator_or_policy_authorization", False, "explicit authorization"),
    )
    for label, key, value, expected in negatives:
        candidate = dict(request)
        candidate[key] = value
        reason = assert_rejected(lambda c=candidate: validate_invocation(c, policy, bootstrap), expected)
        matrix.append({"case": label, "result": "REJECTED", "reason": reason})

    unauthorized = dict(request)
    unauthorized["caller_component_id"] = "untrusted-local-process"
    reason = assert_rejected(lambda: validate_invocation(unauthorized, policy, bootstrap), "not authorized")
    matrix.append({"case": "arbitrary-local-process-cannot-invoke-helper", "result": "REJECTED", "reason": reason})

    wrong_helper = dict(request)
    wrong_helper["helper_binary_sha256"] = sha(b"substituted helper")
    reason = assert_rejected(lambda: validate_invocation(wrong_helper, policy, bootstrap), "does not match verified bootstrap")
    matrix.append({"case": "helper-substitution-rejected", "result": "REJECTED", "reason": reason})

    status_request = dict(request)
    status_request.update({
        "operation": "status",
        "privileged_context_verified": False,
        "explicit_operator_or_policy_authorization": False,
    })
    validate_invocation(status_request, policy, bootstrap)
    matrix.append({"case": "verified-read-only-status-does-not-require-mutation-privilege", "result": "PASS"})

    for label, key, value, expected in (
        ("unsigned-helper", "helper_signature_verified", False, "code signature"),
        ("untrusted-writable-parent", "parent_writable_by_untrusted_user", True, "writable by an untrusted user"),
        ("reparse-helper-path", "path_reparse_free", False, "symlink/reparse"),
        ("unverified-helper-acl", "owner_acl_verified", False, "owner/ACL"),
        ("missing-bootstrap-lineage", "bootstrap_lineage_verified", False, "bootstrap lineage"),
        ("direct-running-self-replacement", "self_replacement_while_running", True, "must not directly self-replace"),
    ):
        candidate = dict(bootstrap)
        candidate[key] = value
        reason = assert_rejected(lambda c=candidate: validate_bootstrap(c), expected)
        matrix.append({"case": label, "result": "REJECTED", "reason": reason})

    success_response = {
        "schema_version": 1,
        "ok": True,
        "exit_code": 0,
        "result": {"operation": "status", "state": "fixture"},
        "error_code": None,
        "error_message": None,
    }
    validate_response(success_response, expect_success=True)
    matrix.append({"case": "structured-success-response", "result": "PASS"})

    failure_response = {
        "schema_version": 1,
        "ok": False,
        "exit_code": 1,
        "result": None,
        "error_code": "CALLER_UNVERIFIED",
        "error_message": "caller identity verification failed",
    }
    validate_response(failure_response, expect_success=False)
    matrix.append({"case": "structured-failure-response-with-nonzero-exit", "result": "PASS"})

    bad_failure = dict(failure_response)
    bad_failure["exit_code"] = 0
    reason = assert_rejected(lambda: validate_response(bad_failure, expect_success=False), "non-zero")
    matrix.append({"case": "failure-cannot-exit-zero", "result": "REJECTED", "reason": reason})

    leaking_failure = dict(failure_response)
    leaking_failure["error_message"] = "token=fixture-secret-like-value"
    reason = assert_rejected(lambda: validate_response(leaking_failure, expect_success=False), "secret-like")
    matrix.append({"case": "failure-message-secret-redaction", "result": "REJECTED", "reason": reason})

    report = {
        "schema_version": 1,
        "package_id": "PKG-04",
        "task_scope": ["04.05", "04.11"],
        "mode": MODE,
        "source_commit": source_commit,
        "canonical_prerequisite": "PKG-03:COMPLETE",
        "canonical_tasks_active": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "production_evidence_consumed": False,
        "product_helper_mutated": False,
        "helper_installed": False,
        "privilege_elevation_performed": False,
        "production_signer_present": False,
        "fixture_identity": True,
        "raw_stdin_authorization_forbidden": True,
        "caller_identity_and_signature_required": True,
        "secure_launch_channel_required": True,
        "mutating_operation_privilege_and_authorization_required": True,
        "helper_bootstrap_identity_required": True,
        "untrusted_writable_parent_forbidden": True,
        "reparse_helper_path_forbidden": True,
        "direct_helper_self_replacement_forbidden": True,
        "structured_failure_contract_required": True,
        "nonzero_failure_exit_required": True,
        "test_case_count": len(matrix),
        "negative_rejections": sum(row["result"] == "REJECTED" for row in matrix),
        "result": "PASS",
    }
    return report, serializable_policy, matrix


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
    print("PKG04_HELPER_AUTHORITY_PREIMPLEMENTATION=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
