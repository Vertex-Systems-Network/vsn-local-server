#!/usr/bin/env python3
"""Adversarial filesystem contract for future PKG-04 updater implementation.

NON_ACCEPTANCE_PREIMPLEMENTATION only. This models secure-design requirements derived
from isolated PKG-04 audit findings. It does not modify or certify the product updater.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import tempfile
from pathlib import Path, PurePosixPath

MODE = "NON_ACCEPTANCE_PREIMPLEMENTATION"


class ContractError(ValueError):
    pass


def canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha(value: str | bytes) -> str:
    if isinstance(value, str):
        value = value.encode("utf-8")
    return hashlib.sha256(value).hexdigest()


def normalize_target(value: str) -> str:
    if not value or value.startswith(("/", "\\")) or re.match(r"^[A-Za-z]:", value):
        raise ContractError("target must be relative")
    path = PurePosixPath(value.replace("\\", "/"))
    if any(part in {"", ".", ".."} for part in path.parts):
        raise ContractError("unsafe target component")
    return str(path)


def component_slot(transaction_id: str, target: str, kind: str) -> str:
    if kind not in {"pending", "previous", "failed-current"}:
        raise ContractError("unknown component slot kind")
    if not re.fullmatch(r"[0-9a-f]{32}", transaction_id):
        raise ContractError("transaction id must be a 128-bit lowercase hex identifier")
    normalized = normalize_target(target)
    target_digest = sha(normalized)[:24]
    return f"{kind}/{transaction_id}/{target_digest}.bin"


def bind_parent_identity(validated_identity: str, mutation_identity: str) -> bool:
    if not validated_identity or not mutation_identity:
        raise ContractError("parent identity proof missing")
    if validated_identity != mutation_identity:
        raise ContractError("target parent identity changed after validation")
    return True


def require_control_root(*, contained: bool, reparse_free: bool, identity_stable: bool) -> bool:
    if not contained:
        raise ContractError("updater control root escapes canonical install root")
    if not reparse_free:
        raise ContractError("updater control root is a symlink/reparse redirection")
    if not identity_stable:
        raise ContractError("updater control root identity changed during transaction")
    return True


def create_state_temp_exclusive(path: Path, payload: bytes) -> None:
    # "xb" maps to O_CREAT|O_EXCL semantics: any pre-existing filesystem entry,
    # including a planted hardlink/symlink path, must fail before bytes are written.
    with path.open("xb") as handle:
        handle.write(payload)
        handle.flush()
        os.fsync(handle.fileno())


def assert_rejected(fn, expected: str = "") -> str:
    try:
        fn()
    except (ContractError, FileExistsError) as exc:
        text = str(exc)
        if expected and expected not in text:
            raise AssertionError((expected, text))
        return text
    raise AssertionError(f"expected rejection containing {expected!r}")


def run(source_commit: str) -> tuple[dict, list[dict]]:
    if not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        raise ContractError("source commit must be a lowercase 40-character SHA-1")

    matrix: list[dict] = []
    tx = sha(b"pkg04-adversarial-fixture")[:32]

    assert bind_parent_identity("volume42:file100", "volume42:file100")
    matrix.append({"case": "mutation-time-parent-identity-stable", "result": "PASS"})
    reason = assert_rejected(
        lambda: bind_parent_identity("volume42:file100", "volume42:file999"),
        "changed after validation",
    )
    matrix.append({"case": "parent-swap-rejected", "result": "REJECTED", "reason": reason})

    assert require_control_root(contained=True, reparse_free=True, identity_stable=True)
    matrix.append({"case": "contained-stable-control-root", "result": "PASS"})
    for case, kwargs, expected in (
        ("control-root-outside", dict(contained=False, reparse_free=True, identity_stable=True), "escapes canonical install root"),
        ("control-root-reparse", dict(contained=True, reparse_free=False, identity_stable=True), "symlink/reparse"),
        ("control-root-swapped", dict(contained=True, reparse_free=True, identity_stable=False), "identity changed"),
    ):
        reason = assert_rejected(lambda kwargs=kwargs: require_control_root(**kwargs), expected)
        matrix.append({"case": case, "result": "REJECTED", "reason": reason})

    first = "bin/vsn-agent.exe"
    second = "alt/vsn-agent.exe"
    slots = {}
    for kind in ("pending", "previous", "failed-current"):
        slot_a = component_slot(tx, first, kind)
        slot_b = component_slot(tx, second, kind)
        assert slot_a != slot_b
        assert Path(slot_a).name != Path(slot_b).name
        slots[kind] = {"first": slot_a, "second": slot_b}
        matrix.append({"case": f"{kind}-same-basename-targets-do-not-collide", "result": "PASS"})

    previous_tx = sha(b"pkg04-previous-transaction")[:32]
    assert component_slot(previous_tx, first, "previous") != component_slot(tx, first, "previous")
    matrix.append({"case": "transaction-namespace-does-not-collide", "result": "PASS"})

    with tempfile.TemporaryDirectory(prefix="vsn-pkg04-adversarial-") as temp:
        root = Path(temp)
        control = root / ".vsn-update"
        control.mkdir()
        outside = root / "outside-sentinel.json"
        outside_original = b"OUTSIDE-SENTINEL"
        outside.write_bytes(outside_original)
        state_tmp = control / "state.json.tmp"

        try:
            os.link(outside, state_tmp)
        except OSError as exc:
            raise ContractError(f"hardlink fixture unavailable on this runner: {exc}") from exc
        reason = assert_rejected(
            lambda: create_state_temp_exclusive(state_tmp, b'{"release":"2.0.0"}\n')
        )
        assert outside.read_bytes() == outside_original
        assert state_tmp.read_bytes() == outside_original
        matrix.append({
            "case": "preplanted-state-temp-hardlink-rejected-without-outside-mutation",
            "result": "REJECTED",
            "reason": reason,
            "outside_sha256": sha(outside_original),
        })
        state_tmp.unlink()

        safe_state_tmp = control / "state.safe.tmp"
        payload = b'{"release":"2.0.0"}\n'
        create_state_temp_exclusive(safe_state_tmp, payload)
        assert safe_state_tmp.read_bytes() == payload
        matrix.append({"case": "exclusive-state-temp-create", "result": "PASS"})

        reason = assert_rejected(
            lambda: create_state_temp_exclusive(safe_state_tmp, b"overwrite-forbidden")
        )
        assert safe_state_tmp.read_bytes() == payload
        matrix.append({"case": "existing-state-temp-never-overwritten", "result": "REJECTED", "reason": reason})

    report = {
        "schema_version": 1,
        "package_id": "PKG-04",
        "contract": "adversarial-filesystem",
        "mode": MODE,
        "source_commit": source_commit,
        "canonical_prerequisite": "PKG-03:COMPLETE",
        "canonical_package_active": False,
        "canonical_task_0401_active": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "production_evidence_consumed": False,
        "product_updater_mutated": False,
        "real_update_executed": False,
        "real_rollback_executed": False,
        "audit_findings_used_as_requirements": [
            "post-validation-parent-swap",
            "control-root-redirection",
            "cross-target-backup-lineage-collision",
            "failed-current-lineage-collision",
            "state-temp-hardlink-composition",
        ],
        "mutation_time_parent_identity_required": True,
        "control_root_containment_required": True,
        "control_root_reparse_rejection_required": True,
        "transaction_and_target_digest_namespacing_required": True,
        "exclusive_state_temp_creation_required": True,
        "test_case_count": len(matrix),
        "rejected_negative_case_count": sum(row["result"] == "REJECTED" for row in matrix),
        "result": "PASS",
    }
    contract = {
        "schema_version": 1,
        "transaction_id": tx,
        "component_slots": slots,
        "invariants": {
            "revalidate_parent_identity_at_mutation": True,
            "control_root_must_be_contained_and_non_reparse": True,
            "component_slots_include_transaction_and_target_digest": True,
            "state_temp_must_use_exclusive_creation": True,
            "preexisting_state_temp_entry_is_fatal": True,
        },
    }
    return {"report": report, "contract": contract}, matrix


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--output-dir", required=True)
    args = parser.parse_args()

    bundle, matrix = run(args.source_commit)
    out = Path(args.output_dir)
    out.mkdir(parents=True, exist_ok=True)
    (out / "contract.json").write_bytes(canonical(bundle["contract"]))
    (out / "matrix.json").write_bytes(canonical(matrix))
    (out / "report.json").write_bytes(canonical(bundle["report"]))
    print(json.dumps(bundle["report"], sort_keys=True))
    print("PKG04_UPDATER_ADVERSARIAL_PREIMPLEMENTATION=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
