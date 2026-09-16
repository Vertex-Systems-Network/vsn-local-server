#!/usr/bin/env python3
"""Freeze the PKG-04 04.16-04.18 genuine-evidence handoff and final-gate schema.

NON_ACCEPTANCE_PREIMPLEMENTATION. Self-test mode uses explicit synthetic records only
to prove that production validators reject fixture/non-real evidence. It does not run
an installed Windows update, publish release metadata, activate PKG-05, or complete
PKG-04.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from copy import deepcopy
from pathlib import Path

MODE = "NON_ACCEPTANCE_PREIMPLEMENTATION"
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
SHA1_RE = re.compile(r"^[0-9a-f]{40}$")
INSTALLERS = ("nsis-current-user", "nsis-per-machine", "msi")
COMPONENTS = ("agent", "cli", "desktop", "updater-helper")
PRE_FINAL_TASKS = tuple(f"04.{n:02d}" for n in range(2, 18))


class HandoffError(ValueError):
    pass


def canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def sha(value: str | bytes) -> str:
    if isinstance(value, str):
        value = value.encode("utf-8")
    return hashlib.sha256(value).hexdigest()


def require_sha256(value: object, label: str) -> str:
    if not isinstance(value, str) or not SHA256_RE.fullmatch(value):
        raise HandoffError(f"{label} must be lowercase SHA-256")
    return value


def require_sha1(value: object, label: str) -> str:
    if not isinstance(value, str) or not SHA1_RE.fullmatch(value):
        raise HandoffError(f"{label} must be lowercase 40-character git SHA")
    return value


def require_real_common(evidence: dict, label: str) -> None:
    if evidence.get("mode") != "PRODUCTION_ACCEPTANCE_EVIDENCE":
        raise HandoffError(f"{label} is not production acceptance evidence")
    if evidence.get("fixture_identity") is not False:
        raise HandoffError(f"{label} fixture evidence is forbidden")
    if evidence.get("real_execution") is not True:
        raise HandoffError(f"{label} must prove real execution")
    if evidence.get("production_evidence") is not True:
        raise HandoffError(f"{label} production marker missing")
    require_sha1(evidence.get("source_commit"), f"{label} source commit")


def validate_installed_windows_e2e(evidence: dict) -> None:
    require_real_common(evidence, "04.16")
    if evidence.get("task_id") != "04.16":
        raise HandoffError("04.16 task identity mismatch")
    if evidence.get("os_family") != "windows":
        raise HandoffError("04.16 requires real Windows execution")
    if evidence.get("persistent_machine") is not True:
        raise HandoffError("04.16 requires persistent-machine semantics")
    if not isinstance(evidence.get("workflow_run_id"), int) or evidence["workflow_run_id"] <= 0:
        raise HandoffError("04.16 workflow run id missing")
    if not isinstance(evidence.get("workflow_job_id"), int) or evidence["workflow_job_id"] <= 0:
        raise HandoffError("04.16 workflow job id missing")
    if evidence.get("accepted_pkg03_installer_evidence") is not True:
        raise HandoffError("04.16 must consume accepted PKG-03 installer evidence")
    require_sha1(evidence.get("accepted_pkg03_source_commit"), "04.16 accepted PKG-03 source")
    installers = evidence.get("installers")
    if not isinstance(installers, list) or {row.get("id") for row in installers} != set(INSTALLERS):
        raise HandoffError("04.16 installer matrix must cover current-user NSIS, per-machine NSIS and MSI")
    for row in installers:
        require_sha256(row.get("sha256"), f"04.16 {row.get('id')} digest")
        if row.get("signature_verified") is not True:
            raise HandoffError("04.16 installers must have verified production signatures")
        if not isinstance(row.get("signer_subject"), str) or not row["signer_subject"].strip():
            raise HandoffError("04.16 installer signer subject missing")
        if row.get("install_pass") is not True or row.get("uninstall_pass") is not True:
            raise HandoffError("04.16 installer install/uninstall acceptance incomplete")
    if evidence.get("update_apply_pass") is not True or evidence.get("rollback_pass") is not True:
        raise HandoffError("04.16 real update and rollback must both pass")
    if evidence.get("interruption_recovery_pass") is not True:
        raise HandoffError("04.16 interruption recovery proof missing")
    if evidence.get("locked_file_policy_pass") is not True:
        raise HandoffError("04.16 locked-file/restart policy proof missing")
    if evidence.get("component_release_coherence_pass") is not True:
        raise HandoffError("04.16 component release coherence proof missing")
    component_hashes = evidence.get("updated_component_sha256")
    if not isinstance(component_hashes, dict) or set(component_hashes) != set(COMPONENTS):
        raise HandoffError("04.16 updated component identity set incomplete")
    for cid, digest in component_hashes.items():
        require_sha256(digest, f"04.16 updated {cid}")
    before = evidence.get("mutable_state_before_sha256")
    after_update = evidence.get("mutable_state_after_update_sha256")
    after_rollback = evidence.get("mutable_state_after_rollback_sha256")
    if not isinstance(before, dict) or not before or before != after_update or before != after_rollback:
        raise HandoffError("04.16 mutable state was not preserved across update and rollback")
    for name, digest in before.items():
        require_sha256(digest, f"04.16 mutable state {name}")
    if evidence.get("reboot_required") is True:
        if evidence.get("same_machine_after_reboot") is not True:
            raise HandoffError("04.16 reboot proof must retain the same machine")
        if not evidence.get("pre_boot_marker") or evidence.get("pre_boot_marker") == evidence.get("post_boot_marker"):
            raise HandoffError("04.16 reboot proof requires distinct boot markers")
    if evidence.get("cleanup_pass") is not True:
        raise HandoffError("04.16 cleanup proof missing")


def validate_release_handoff(evidence: dict, e2e: dict) -> None:
    require_real_common(evidence, "04.17")
    if evidence.get("task_id") != "04.17":
        raise HandoffError("04.17 task identity mismatch")
    if evidence.get("source_commit") != e2e.get("source_commit"):
        raise HandoffError("04.17 source commit must match accepted 04.16 product source")
    if evidence.get("accepted_0416") is not True:
        raise HandoffError("04.17 must consume accepted 04.16 evidence")
    require_sha256(evidence.get("accepted_0416_evidence_sha256"), "04.17 accepted 04.16 evidence")
    release_id = evidence.get("release_id")
    if not isinstance(release_id, str) or not re.fullmatch(r"[A-Za-z0-9._+-]{1,80}", release_id):
        raise HandoffError("04.17 release id invalid")
    for field in ("release_manifest_sha256", "checksums_sha256", "sbom_sha256", "provenance_sha256"):
        require_sha256(evidence.get(field), f"04.17 {field}")
    if evidence.get("artifact_checksums_cover_all_release_subjects") is not True:
        raise HandoffError("04.17 checksum coverage incomplete")
    if evidence.get("provenance_binds_source_and_artifact_hashes") is not True:
        raise HandoffError("04.17 provenance binding incomplete")
    if evidence.get("release_metadata_published") is not True:
        raise HandoffError("04.17 release/update metadata publication proof missing")
    if evidence.get("pkg05_handoff_generated") is not True:
        raise HandoffError("04.17 PKG-05 handoff generation proof missing")
    if evidence.get("pkg05_activation_authority") is not False:
        raise HandoffError("04.17 must not activate PKG-05")
    handoff = evidence.get("pkg05_handoff")
    if not isinstance(handoff, dict):
        raise HandoffError("04.17 PKG-05 handoff missing")
    require_sha1(handoff.get("pkg04_final_source_commit"), "04.17 PKG-05 handoff source")
    require_sha256(handoff.get("release_evidence_index_sha256"), "04.17 PKG-05 handoff evidence index")
    if handoff.get("prerequisite") != "PKG-04:COMPLETE_AFTER_04.18_ACCEPTANCE":
        raise HandoffError("04.17 PKG-05 prerequisite boundary drifted")


def validate_final_gate(evidence: dict, e2e: dict, release: dict) -> None:
    require_real_common(evidence, "04.18")
    if evidence.get("task_id") != "04.18":
        raise HandoffError("04.18 task identity mismatch")
    tracker = evidence.get("canonical_tracker_snapshot")
    if not isinstance(tracker, dict):
        raise HandoffError("04.18 canonical tracker snapshot missing")
    done_tasks = set(tracker.get("done_tasks", []))
    if not set(PRE_FINAL_TASKS).issubset(done_tasks):
        raise HandoffError("04.18 requires every 04.02-04.17 task DONE")
    if tracker.get("active_task") != "04.18" or tracker.get("ready_tasks") != ["04.18"]:
        raise HandoffError("04.18 requires exact active/ready canonical state")
    if tracker.get("pkg04_complete") is not False:
        raise HandoffError("04.18 pre-acceptance tracker must not already claim PKG-04 complete")
    if evidence.get("source_commit") != e2e.get("source_commit") or evidence.get("source_commit") != release.get("source_commit"):
        raise HandoffError("04.18 exact product source lineage mismatch")
    for field in ("accepted_0416_evidence_sha256", "accepted_0417_evidence_sha256", "final_evidence_index_sha256"):
        require_sha256(evidence.get(field), f"04.18 {field}")
    if evidence.get("accepted_0416") is not True or evidence.get("accepted_0417") is not True:
        raise HandoffError("04.18 requires accepted 04.16 and 04.17 evidence")
    if evidence.get("fresh_state_windows_run") is not True:
        raise HandoffError("04.18 requires genuine fresh-state Windows execution")
    if evidence.get("exact_head_validation") is not True:
        raise HandoffError("04.18 requires exact-head validation")
    if evidence.get("all_required_ci_green") is not True:
        raise HandoffError("04.18 requires all required CI green")
    if evidence.get("acceptance_projection_authority") is not False:
        raise HandoffError("preimplementation validator must not project completion itself")
    if evidence.get("pkg05_activation_authority") is not False:
        raise HandoffError("04.18 must not directly activate PKG-05")


def assert_rejected(fn, expected: str) -> str:
    try:
        fn()
    except HandoffError as exc:
        text = str(exc)
        if expected not in text:
            raise AssertionError((expected, text))
        return text
    raise AssertionError(f"expected rejection containing: {expected}")


def production_shaped_fixture(source_commit: str) -> tuple[dict, dict, dict]:
    installers = []
    for iid in INSTALLERS:
        installers.append({
            "id": iid,
            "sha256": sha(f"installer:{iid}"),
            "signature_verified": True,
            "signer_subject": "CN=Fixture Production Shape Only",
            "install_pass": True,
            "uninstall_pass": True,
        })
    component_hashes = {cid: sha(f"component:{cid}:next") for cid in COMPONENTS}
    mutable = {
        "config": sha("mutable:config"),
        "user-data": sha("mutable:user-data"),
        "machine-data": sha("mutable:machine-data"),
    }
    e2e = {
        "schema_version": 1,
        "task_id": "04.16",
        "mode": "PRODUCTION_ACCEPTANCE_EVIDENCE",
        "fixture_identity": False,
        "real_execution": True,
        "production_evidence": True,
        "source_commit": source_commit,
        "os_family": "windows",
        "persistent_machine": True,
        "workflow_run_id": 160016,
        "workflow_job_id": 160017,
        "accepted_pkg03_installer_evidence": True,
        "accepted_pkg03_source_commit": "1" * 40,
        "installers": installers,
        "update_apply_pass": True,
        "rollback_pass": True,
        "interruption_recovery_pass": True,
        "locked_file_policy_pass": True,
        "component_release_coherence_pass": True,
        "updated_component_sha256": component_hashes,
        "mutable_state_before_sha256": mutable,
        "mutable_state_after_update_sha256": dict(mutable),
        "mutable_state_after_rollback_sha256": dict(mutable),
        "reboot_required": True,
        "same_machine_after_reboot": True,
        "pre_boot_marker": "fixture-boot-a",
        "post_boot_marker": "fixture-boot-b",
        "cleanup_pass": True,
    }
    release = {
        "schema_version": 1,
        "task_id": "04.17",
        "mode": "PRODUCTION_ACCEPTANCE_EVIDENCE",
        "fixture_identity": False,
        "real_execution": True,
        "production_evidence": True,
        "source_commit": source_commit,
        "accepted_0416": True,
        "accepted_0416_evidence_sha256": sha(canonical(e2e)),
        "release_id": "0.39.0-fixture-shape",
        "release_manifest_sha256": sha("manifest"),
        "checksums_sha256": sha("checksums"),
        "sbom_sha256": sha("sbom"),
        "provenance_sha256": sha("provenance"),
        "artifact_checksums_cover_all_release_subjects": True,
        "provenance_binds_source_and_artifact_hashes": True,
        "release_metadata_published": True,
        "pkg05_handoff_generated": True,
        "pkg05_activation_authority": False,
        "pkg05_handoff": {
            "pkg04_final_source_commit": source_commit,
            "release_evidence_index_sha256": sha("release-index"),
            "prerequisite": "PKG-04:COMPLETE_AFTER_04.18_ACCEPTANCE",
        },
    }
    tracker = {
        "active_task": "04.18",
        "ready_tasks": ["04.18"],
        "done_tasks": list(PRE_FINAL_TASKS),
        "pkg04_complete": False,
    }
    final = {
        "schema_version": 1,
        "task_id": "04.18",
        "mode": "PRODUCTION_ACCEPTANCE_EVIDENCE",
        "fixture_identity": False,
        "real_execution": True,
        "production_evidence": True,
        "source_commit": source_commit,
        "canonical_tracker_snapshot": tracker,
        "accepted_0416": True,
        "accepted_0417": True,
        "accepted_0416_evidence_sha256": sha(canonical(e2e)),
        "accepted_0417_evidence_sha256": sha(canonical(release)),
        "final_evidence_index_sha256": sha("final-index"),
        "fresh_state_windows_run": True,
        "exact_head_validation": True,
        "all_required_ci_green": True,
        "acceptance_projection_authority": False,
        "pkg05_activation_authority": False,
    }
    return e2e, release, final


def run(source_commit: str) -> tuple[dict, dict, list[dict]]:
    require_sha1(source_commit, "self-test source commit")
    e2e, release, final = production_shaped_fixture(source_commit)
    matrix: list[dict] = []

    # Validate the production-shaped schema first, then prove any fixture marker is rejected.
    validate_installed_windows_e2e(e2e)
    validate_release_handoff(release, e2e)
    validate_final_gate(final, e2e, release)
    matrix.append({"case": "production-shaped-schema-coherent", "result": "PASS"})

    for label, base, validator, expected in (
        ("0416-fixture-rejected", e2e, lambda x: validate_installed_windows_e2e(x), "fixture evidence"),
        ("0417-fixture-rejected", release, lambda x: validate_release_handoff(x, e2e), "fixture evidence"),
        ("0418-fixture-rejected", final, lambda x: validate_final_gate(x, e2e, release), "fixture evidence"),
    ):
        bad = deepcopy(base)
        bad["fixture_identity"] = True
        reason = assert_rejected(lambda b=bad, v=validator: v(b), expected)
        matrix.append({"case": label, "result": "REJECTED", "reason": reason})

    bad = deepcopy(e2e)
    bad["real_execution"] = False
    matrix.append({"case": "0416-no-real-windows-run", "result": "REJECTED", "reason": assert_rejected(lambda: validate_installed_windows_e2e(bad), "real execution")})
    bad = deepcopy(e2e)
    bad["installers"][0]["signature_verified"] = False
    matrix.append({"case": "0416-unverified-installer", "result": "REJECTED", "reason": assert_rejected(lambda: validate_installed_windows_e2e(bad), "production signatures")})
    bad = deepcopy(e2e)
    bad["mutable_state_after_update_sha256"]["config"] = sha("changed")
    matrix.append({"case": "0416-mutable-state-drift", "result": "REJECTED", "reason": assert_rejected(lambda: validate_installed_windows_e2e(bad), "mutable state")})
    bad = deepcopy(release)
    bad["pkg05_activation_authority"] = True
    matrix.append({"case": "0417-cannot-activate-pkg05", "result": "REJECTED", "reason": assert_rejected(lambda: validate_release_handoff(bad, e2e), "must not activate PKG-05")})
    bad = deepcopy(final)
    bad["canonical_tracker_snapshot"]["done_tasks"].remove("04.15")
    matrix.append({"case": "0418-missing-prior-done-task", "result": "REJECTED", "reason": assert_rejected(lambda: validate_final_gate(bad, e2e, release), "every 04.02-04.17")})
    bad = deepcopy(final)
    bad["acceptance_projection_authority"] = True
    matrix.append({"case": "0418-validator-cannot-self-complete", "result": "REJECTED", "reason": assert_rejected(lambda: validate_final_gate(bad, e2e, release), "must not project completion")})

    schema = {
        "schema_version": 1,
        "mode": MODE,
        "task_scope": ["04.16", "04.17", "04.18"],
        "production_mode_required": "PRODUCTION_ACCEPTANCE_EVIDENCE",
        "fixture_identity_required_in_production": False,
        "real_execution_required": True,
        "0416": {
            "windows_persistent_machine_required": True,
            "accepted_pkg03_installer_evidence_required": True,
            "installer_ids": list(INSTALLERS),
            "verified_production_signatures_required": True,
            "update_and_rollback_required": True,
            "interruption_and_locked_file_required": True,
            "component_hashes_required": list(COMPONENTS),
            "mutable_state_hash_preservation_required": True,
            "same_machine_reboot_proof_if_required": True,
        },
        "0417": {
            "release_manifest_checksums_sbom_provenance_required": True,
            "published_release_metadata_required": True,
            "pkg05_handoff_required": True,
            "pkg05_activation_forbidden": True,
        },
        "0418": {
            "all_0402_through_0417_done_required": True,
            "active_ready_exactly_0418_required": True,
            "fresh_state_windows_required": True,
            "exact_head_and_all_ci_green_required": True,
            "fixture_evidence_forbidden": True,
            "self_completion_projection_forbidden": True,
            "pkg05_activation_forbidden": True,
        },
    }
    report = {
        "schema_version": 1,
        "package_id": "PKG-04",
        "task_scope": ["04.16", "04.17", "04.18"],
        "mode": MODE,
        "source_commit": source_commit,
        "canonical_prerequisite": "04.15:DONE for 04.16; 04.16:DONE for 04.17; 04.17:DONE for 04.18",
        "canonical_tasks_active": False,
        "canonical_state_changed": False,
        "implementation_authority": False,
        "production_evidence_consumed": False,
        "real_windows_e2e_executed": False,
        "release_metadata_published": False,
        "pkg05_handoff_published": False,
        "pkg05_activated": False,
        "pkg04_completion_projected": False,
        "fixture_identity": True,
        "genuine_evidence_schema_frozen": True,
        "fixture_evidence_rejected_by_production_validators": True,
        "exact_lineage_required": True,
        "test_case_count": len(matrix),
        "negative_rejections": sum(row["result"] == "REJECTED" for row in matrix),
        "result": "PASS",
    }
    return report, schema, matrix


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--output-dir", required=True)
    args = parser.parse_args()

    report, schema, matrix = run(args.source_commit)
    out = Path(args.output_dir)
    out.mkdir(parents=True, exist_ok=True)
    (out / "report.json").write_bytes(canonical(report))
    (out / "schema.json").write_bytes(canonical(schema))
    (out / "matrix.json").write_bytes(canonical(matrix))
    print(json.dumps(report, sort_keys=True))
    print("PKG04_FINAL_HANDOFF_PREIMPLEMENTATION=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
