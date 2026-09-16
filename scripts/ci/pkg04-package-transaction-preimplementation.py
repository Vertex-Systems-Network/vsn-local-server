#!/usr/bin/env python3
"""Fixture-only PKG-04 04.06-04.10 package transaction coordinator model.

NON_ACCEPTANCE_PREIMPLEMENTATION. This does not mutate product updater code, stop real
processes/services, replace installed files, reboot a machine, or activate PKG-04.
It models package-wide coherence, quiesce, apply, crash recovery and rollback invariants.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from copy import deepcopy
from pathlib import Path

MODE = "NON_ACCEPTANCE_PREIMPLEMENTATION"
COMPONENT_ORDER = ("agent", "cli", "desktop", "updater-helper")
MUTABLE_STATE_CLASSES = ("config", "user-data", "machine-data", "logs", "cache")
TX_PHASES = (
    "planned",
    "staged-verified",
    "quiesced",
    "applying",
    "next-verified",
    "committed",
    "restoring",
    "restored",
    "rollback-applying",
    "rollback-committed",
)


class TransactionError(ValueError):
    pass


def canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha(value: str | bytes) -> str:
    if isinstance(value, str):
        value = value.encode("utf-8")
    return hashlib.sha256(value).hexdigest()


def make_component(component_id: str, target: str, kind: str) -> dict:
    previous = f"{component_id}:0.38.1:accepted".encode()
    next_payload = f"{component_id}:0.39.0:fixture".encode()
    return {
        "component_id": component_id,
        "target_relative": target,
        "kind": kind,
        "previous_release": "0.38.1",
        "previous_sha256": sha(previous),
        "previous_bytes": len(previous),
        "next_release": "0.39.0-fixture",
        "next_sha256": sha(next_payload),
        "next_bytes": len(next_payload),
        "staged_verified": True,
        "previous_verified": True,
        "target_containment_verified": True,
        "target_parent_identity_stable": True,
        "security_descriptor_preservation_required": True,
        "mutable_state_owned": False,
        "quiesce_required": component_id in {"agent", "desktop"},
        "quiesce_verified": component_id not in {"agent", "desktop"},
        "replacement_authority": "external_trusted_bootstrap" if component_id == "updater-helper" else "verified_updater_helper",
    }


def fixture_plan() -> dict:
    components = [
        make_component("agent", "bin/vsn-agent.exe", "service"),
        make_component("cli", "bin/vsn.exe", "executable"),
        make_component("desktop", "VSN Dev Platform.exe", "desktop"),
        make_component("updater-helper", "bin/vsn-updater-helper.exe", "helper"),
    ]
    return {
        "schema_version": 1,
        "transaction_id": sha(b"pkg04-package-transaction-fixture")[:32],
        "from_release": "0.38.1",
        "to_release": "0.39.0-fixture",
        "components": components,
        "component_order": list(COMPONENT_ORDER),
        "all_staged_before_quiesce": True,
        "all_staged_verified_before_quiesce": True,
        "mutable_state_policy": {name: "preserve" for name in MUTABLE_STATE_CLASSES},
        "locked_file_policy": "fail_closed_or_explicit_reboot_transaction",
        "implicit_reboot_deferred_replace_forbidden": True,
        "restart_only_after_coherent_terminal_state": True,
        "mixed_version_commit_forbidden": True,
        "helper_self_replacement_forbidden": True,
        "transaction_phases": list(TX_PHASES),
    }


def validate_plan(plan: dict) -> None:
    if plan.get("schema_version") != 1:
        raise TransactionError("transaction schema invalid")
    tx = plan.get("transaction_id")
    if not isinstance(tx, str) or not re.fullmatch(r"[0-9a-f]{32}", tx):
        raise TransactionError("transaction id invalid")
    components = plan.get("components")
    if not isinstance(components, list) or len(components) != len(COMPONENT_ORDER):
        raise TransactionError("transaction must bind all package components")
    ids = [c.get("component_id") for c in components]
    if ids != list(COMPONENT_ORDER) or plan.get("component_order") != list(COMPONENT_ORDER):
        raise TransactionError("component order or identity set drifted")
    targets = [c.get("target_relative") for c in components]
    if len(set(targets)) != len(targets):
        raise TransactionError("component targets collide")
    for component in components:
        if component.get("staged_verified") is not True:
            raise TransactionError("all staged component artifacts must be verified before mutation")
        if component.get("previous_verified") is not True:
            raise TransactionError("previous accepted component identity is unverified")
        if component.get("target_containment_verified") is not True or component.get("target_parent_identity_stable") is not True:
            raise TransactionError("component target containment/identity proof missing")
        if component.get("security_descriptor_preservation_required") is not True:
            raise TransactionError("component security descriptor preservation requirement missing")
        if component.get("mutable_state_owned") is not False:
            raise TransactionError("immutable package transaction must not claim mutable user/machine state")
        if component["component_id"] == "updater-helper" and component.get("replacement_authority") != "external_trusted_bootstrap":
            raise TransactionError("updater helper replacement must use external trusted bootstrap authority")
    if plan.get("all_staged_before_quiesce") is not True or plan.get("all_staged_verified_before_quiesce") is not True:
        raise TransactionError("package must be fully staged and verified before quiesce")
    if set(plan.get("mutable_state_policy", {}).values()) != {"preserve"}:
        raise TransactionError("mutable state preservation policy drifted")
    if plan.get("locked_file_policy") != "fail_closed_or_explicit_reboot_transaction":
        raise TransactionError("locked-file policy is not explicit")
    if plan.get("implicit_reboot_deferred_replace_forbidden") is not True:
        raise TransactionError("implicit reboot-deferred replacement must be forbidden")
    if plan.get("restart_only_after_coherent_terminal_state") is not True:
        raise TransactionError("restart coordination must wait for a coherent terminal state")
    if plan.get("mixed_version_commit_forbidden") is not True:
        raise TransactionError("mixed-version commit must be forbidden")
    if plan.get("helper_self_replacement_forbidden") is not True:
        raise TransactionError("helper self replacement must be forbidden")


def quiesce(plan: dict, acknowledgements: set[str]) -> dict:
    state = deepcopy(plan)
    for component in state["components"]:
        if component["quiesce_required"]:
            if component["component_id"] not in acknowledgements:
                raise TransactionError(f"required quiesce acknowledgement missing for {component['component_id']}")
            component["quiesce_verified"] = True
    if not all((not c["quiesce_required"]) or c["quiesce_verified"] for c in state["components"]):
        raise TransactionError("package quiesce is incomplete")
    state["phase"] = "quiesced"
    return state


def apply_transaction(plan: dict, *, fail_before_component: str | None = None, locked_component: str | None = None, explicit_reboot_transaction: bool = False) -> dict:
    validate_plan(plan)
    state = quiesce(plan, {"agent", "desktop"})
    installed = {c["component_id"]: "previous" for c in state["components"]}
    replaced: list[str] = []
    for component in state["components"]:
        cid = component["component_id"]
        if fail_before_component == cid:
            return recover_partial(state, installed, replaced, reason=f"fault-before-{cid}")
        if locked_component == cid and not explicit_reboot_transaction:
            return recover_partial(state, installed, replaced, reason=f"locked-{cid}")
        if locked_component == cid and explicit_reboot_transaction:
            return {
                "terminal": "reboot-pending",
                "coherent": True,
                "restart_allowed": False,
                "installed": dict(installed),
                "replaced_before_reboot": list(replaced),
                "explicit_reboot_transaction": True,
            }
        installed[cid] = "next"
        replaced.append(cid)
    if set(installed.values()) != {"next"}:
        raise TransactionError("mixed-version state cannot be committed")
    return {
        "terminal": "committed",
        "coherent": True,
        "restart_allowed": True,
        "installed": installed,
        "replaced": replaced,
        "mutable_state_preserved": True,
        "next_components_verified": True,
    }


def recover_partial(plan: dict, installed: dict[str, str], replaced: list[str], *, reason: str) -> dict:
    by_id = {c["component_id"]: c for c in plan["components"]}
    restored: list[str] = []
    for cid in reversed(replaced):
        component = by_id[cid]
        if component.get("previous_verified") is not True:
            raise TransactionError(f"cannot restore unverified previous component: {cid}")
        installed[cid] = "previous"
        restored.append(cid)
    if set(installed.values()) != {"previous"}:
        raise TransactionError("recovery did not restore one coherent previous release")
    return {
        "terminal": "restored",
        "coherent": True,
        "restart_allowed": True,
        "installed": dict(installed),
        "restored": restored,
        "failure_reason": reason,
        "mutable_state_preserved": True,
    }


def rollback_committed(plan: dict, *, tampered_previous_component: str | None = None) -> dict:
    validate_plan(plan)
    installed = {c["component_id"]: "next" for c in plan["components"]}
    for component in reversed(plan["components"]):
        cid = component["component_id"]
        if cid == tampered_previous_component or component.get("previous_verified") is not True:
            raise TransactionError(f"rollback previous identity verification failed for {cid}")
    for component in reversed(plan["components"]):
        installed[component["component_id"]] = "previous"
    if set(installed.values()) != {"previous"}:
        raise TransactionError("rollback did not restore one coherent previous release")
    return {
        "terminal": "rollback-committed",
        "coherent": True,
        "restart_allowed": True,
        "installed": installed,
        "mutable_state_preserved": True,
        "previous_components_verified_before_restore": True,
    }


def assert_rejected(fn, expected: str) -> str:
    try:
        fn()
    except TransactionError as exc:
        text = str(exc)
        if expected not in text:
            raise AssertionError((expected, text))
        return text
    raise AssertionError(f"expected rejection containing: {expected}")


def run(source_commit: str) -> tuple[dict, dict, list[dict]]:
    if not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        raise TransactionError("source commit must be lowercase 40-character SHA-1")
    plan = fixture_plan()
    validate_plan(plan)
    matrix: list[dict] = [{"case": "complete-four-component-plan", "result": "PASS"}]

    committed = apply_transaction(plan)
    assert committed["terminal"] == "committed" and committed["coherent"] and committed["mutable_state_preserved"]
    matrix.append({"case": "atomic-coherent-package-apply", "result": "PASS"})

    for cid in COMPONENT_ORDER:
        recovered = apply_transaction(plan, fail_before_component=cid)
        assert recovered["terminal"] == "restored" and recovered["coherent"] and set(recovered["installed"].values()) == {"previous"}
        matrix.append({"case": f"fault-before-{cid}-restores-previous-release", "result": "PASS"})

    locked = apply_transaction(plan, locked_component="desktop", explicit_reboot_transaction=False)
    assert locked["terminal"] == "restored" and locked["coherent"]
    matrix.append({"case": "locked-file-without-reboot-policy-restores", "result": "PASS"})
    reboot = apply_transaction(plan, locked_component="desktop", explicit_reboot_transaction=True)
    assert reboot["terminal"] == "reboot-pending" and reboot["coherent"] and reboot["restart_allowed"] is False
    matrix.append({"case": "locked-file-explicit-reboot-transaction-stays-noncommitted", "result": "PASS"})

    rolled_back = rollback_committed(plan)
    assert rolled_back["terminal"] == "rollback-committed" and rolled_back["coherent"] and rolled_back["mutable_state_preserved"]
    matrix.append({"case": "verified-package-rollback", "result": "PASS"})
    reason = assert_rejected(lambda: rollback_committed(plan, tampered_previous_component="cli"), "identity verification failed")
    matrix.append({"case": "tampered-previous-component-blocks-rollback", "result": "REJECTED", "reason": reason})

    reason = assert_rejected(lambda: quiesce(plan, {"agent"}), "desktop")
    matrix.append({"case": "missing-desktop-quiesce-ack", "result": "REJECTED", "reason": reason})
    reason = assert_rejected(lambda: quiesce(plan, {"desktop"}), "agent")
    matrix.append({"case": "missing-agent-quiesce-ack", "result": "REJECTED", "reason": reason})

    bad_stage = deepcopy(plan)
    bad_stage["components"][1]["staged_verified"] = False
    reason = assert_rejected(lambda: validate_plan(bad_stage), "staged component artifacts")
    matrix.append({"case": "unverified-staged-component", "result": "REJECTED", "reason": reason})

    bad_mutable = deepcopy(plan)
    bad_mutable["components"][2]["mutable_state_owned"] = True
    reason = assert_rejected(lambda: validate_plan(bad_mutable), "must not claim mutable")
    matrix.append({"case": "package-transaction-cannot-own-user-state", "result": "REJECTED", "reason": reason})

    bad_helper = deepcopy(plan)
    bad_helper["components"][3]["replacement_authority"] = "self"
    reason = assert_rejected(lambda: validate_plan(bad_helper), "external trusted bootstrap")
    matrix.append({"case": "helper-direct-self-replacement", "result": "REJECTED", "reason": reason})

    duplicate_target = deepcopy(plan)
    duplicate_target["components"][1]["target_relative"] = duplicate_target["components"][0]["target_relative"]
    reason = assert_rejected(lambda: validate_plan(duplicate_target), "targets collide")
    matrix.append({"case": "component-target-collision", "result": "REJECTED", "reason": reason})

    report = {
        "schema_version": 1,
        "package_id": "PKG-04",
        "task_scope": ["04.06", "04.07", "04.08", "04.09", "04.10"],
        "mode": MODE,
        "source_commit": source_commit,
        "canonical_prerequisite": "PKG-03:COMPLETE",
        "canonical_tasks_active": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "production_evidence_consumed": False,
        "product_updater_mutated": False,
        "real_process_or_service_quiesce": False,
        "real_file_replacement": False,
        "real_reboot_performed": False,
        "real_rollback_performed": False,
        "fixture_identity": True,
        "package_component_count": len(COMPONENT_ORDER),
        "all_components_staged_and_verified_before_quiesce_required": True,
        "process_service_quiesce_ack_required": True,
        "locked_file_policy_explicit_required": True,
        "implicit_reboot_deferred_replace_forbidden": True,
        "mixed_version_commit_forbidden": True,
        "failure_restores_coherent_previous_release_required": True,
        "rollback_previous_identity_verification_required": True,
        "mutable_state_preservation_required": True,
        "restart_only_after_coherent_terminal_state_required": True,
        "helper_external_replacement_authority_required": True,
        "test_case_count": len(matrix),
        "negative_rejections": sum(row["result"] == "REJECTED" for row in matrix),
        "fault_injection_points": len(COMPONENT_ORDER),
        "result": "PASS",
    }
    contract = {
        "schema_version": 1,
        "mode": MODE,
        "task_scope": report["task_scope"],
        "transaction_id": plan["transaction_id"],
        "component_order": plan["component_order"],
        "transaction_phases": plan["transaction_phases"],
        "mutable_state_policy": plan["mutable_state_policy"],
        "locked_file_policy": plan["locked_file_policy"],
        "terminal_states": ["committed", "restored", "reboot-pending", "rollback-committed"],
        "committed_state_requires_single_release": True,
        "recovery_state_requires_single_release": True,
    }
    return report, contract, matrix


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--output-dir", required=True)
    args = parser.parse_args()

    report, contract, matrix = run(args.source_commit)
    out = Path(args.output_dir)
    out.mkdir(parents=True, exist_ok=True)
    (out / "report.json").write_bytes(canonical(report))
    (out / "contract.json").write_bytes(canonical(contract))
    (out / "matrix.json").write_bytes(canonical(matrix))
    print(json.dumps(report, sort_keys=True))
    print("PKG04_PACKAGE_TRANSACTION_PREIMPLEMENTATION=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
