#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

GAPS = {
    "NO_GAP", "MISSING_IMPLEMENTATION", "PARTIAL_IMPLEMENTATION",
    "IMPLEMENTED_UNVERIFIED", "PLAN_REALITY_MISMATCH",
    "DOCUMENTATION_GAP", "UNKNOWN",
}
APPROVAL_SCOPES = {"TASK", "WORK_PACKAGE", "FEATURE", "PROJECT", "RELEASE", "PRIVILEGED_ACTION"}
REAPPROVAL_TRIGGERS = {
    "SCOPE_EXPANSION", "PRIVILEGE_EXPANSION", "DATA_FLOW_CHANGE",
    "SECURITY_ASSUMPTION_CHANGE", "ACCEPTANCE_CHANGE", "DEPENDENCY_CHANGE",
    "SHARED_SURFACE_COLLISION", "ROLLOUT_CHANGE", "IRREVERSIBLE_ACTION",
}
PARALLEL = {"PARALLEL_SAFE", "SERIALIZE_SHARED_SURFACE", "EXCLUSIVE"}
COMPLETION = {"NOT_STARTED", "IN_PROGRESS", "PARTIALLY_COMPLETE", "COMPLETE", "BLOCKED"}
PROVENANCE = {"HUMAN_REVIEW", "AI_SELF_REVIEW", "AI_INDEPENDENT_REVIEW", "AUTOMATED_STATIC", "AUTOMATED_RUNTIME"}
LIFECYCLE = ["research", "plan", "architecture", "data_flow", "security", "design", "qa", "performance", "development"]


def fail(message: str) -> None:
    raise SystemExit(f"ENGINEERING CONTRACT GOVERNANCE: FAIL\n- {message}")


def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)


def load(path: str) -> dict:
    p = ROOT / path
    require(p.is_file(), f"missing required artifact: {path}")
    try:
        return json.loads(p.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        fail(f"invalid JSON {path}: {exc}")


def validate_contract(doc: dict, label: str, *, feature: bool) -> None:
    gap = doc.get("gap")
    require(isinstance(gap, dict), f"{label}: missing gap block")
    require(set(gap.get("allowed_classifications", [])) == GAPS, f"{label}: gap vocabulary mismatch")
    require(gap.get("classification") in GAPS, f"{label}: invalid gap classification")

    approval = doc.get("approval")
    require(isinstance(approval, dict), f"{label}: missing approval block")
    require(set(approval.get("allowed_scopes", [])) == APPROVAL_SCOPES, f"{label}: approval scope vocabulary mismatch")
    require(approval.get("scope") in APPROVAL_SCOPES, f"{label}: invalid approval scope")
    inherited = approval.get("inherited_authorization")
    require(isinstance(inherited, dict), f"{label}: missing inherited authorization")
    require(inherited.get("may_expand") is False, f"{label}: inherited authorization may expand")
    require(set(approval.get("reapproval_triggers", [])) == REAPPROVAL_TRIGGERS, f"{label}: reapproval trigger set mismatch")

    spec = doc.get("specification")
    require(isinstance(spec, dict) and isinstance(spec.get("modules"), list), f"{label}: module specification missing")
    for module in spec["modules"]:
        require(isinstance(module, dict), f"{label}: invalid module entry")
        if "applicability" in module:
            require(module["applicability"] in {"REQUIRED", "OPTIONAL", "NOT_APPLICABLE"}, f"{label}: invalid module applicability")
        require(isinstance(module.get("options", []), list), f"{label}: module options must be a list")
        for option in module.get("options", []):
            require(option.get("applicability") in {"REQUIRED", "OPTIONAL", "NOT_APPLICABLE"}, f"{label}: invalid option applicability")
            require(isinstance(option.get("value_contract"), str) and option["value_contract"], f"{label}: option value contract missing")
            require(isinstance(option.get("constraints"), list), f"{label}: option constraints must be a list")

    requirements = doc.get("requirements")
    require(isinstance(requirements, dict), f"{label}: requirements block missing")
    for key in ("must", "must_not", "abuse_cases", "forbidden_boundaries"):
        require(isinstance(requirements.get(key), list), f"{label}: requirements.{key} must be a list")

    preflight = doc.get("preflight")
    require(isinstance(preflight, dict), f"{label}: preflight block missing")
    expected = preflight.get("expected_changes")
    require(isinstance(expected, dict), f"{label}: expected_changes missing")
    for key in ("paths", "modules", "change_types"):
        require(isinstance(expected.get(key), list), f"{label}: expected_changes.{key} must be a list")
    require(isinstance(preflight.get("shared_surfaces"), list), f"{label}: shared_surfaces must be a list")
    budget = preflight.get("scope_budget")
    require(isinstance(budget, dict), f"{label}: scope_budget missing")
    for key in ("max_changed_files", "max_new_files", "max_shared_surfaces", "notes"):
        require(key in budget, f"{label}: scope_budget.{key} missing")
    require(preflight.get("scope_exceeded_action") == "STOP_AND_REASSESS", f"{label}: scope budget does not fail closed")

    parallel = doc.get("parallel_safety")
    require(isinstance(parallel, dict), f"{label}: parallel_safety block missing")
    require(set(parallel.get("allowed_classifications", [])) == PARALLEL, f"{label}: parallel classification vocabulary mismatch")
    require(parallel.get("classification") in PARALLEL, f"{label}: invalid parallel classification")
    require(isinstance(parallel.get("collision_keys"), list), f"{label}: collision_keys must be a list")
    require(parallel.get("on_collision") == "STOP_AND_REASSESS", f"{label}: collision does not stop/reassess")

    gates = doc.get("quality_gates")
    require(isinstance(gates, dict), f"{label}: quality_gates missing")
    for gate_name in ("fast_gate", "full_gate"):
        gate = gates.get(gate_name)
        require(isinstance(gate, dict), f"{label}: {gate_name} missing")
        require(isinstance(gate.get("required_on"), list), f"{label}: {gate_name}.required_on must be a list")
        require(isinstance(gate.get("commands"), list), f"{label}: {gate_name}.commands must be a list")
        require(isinstance(gate.get("evidence"), list), f"{label}: {gate_name}.evidence must be a list")
    baseline = gates.get("baseline_failure_policy")
    require(isinstance(baseline, dict), f"{label}: baseline failure policy missing")
    require(baseline.get("label") == "BASELINE_FAILURE", f"{label}: baseline failure label changed")
    require(baseline.get("must_reproduce_on_canonical_base") is True, f"{label}: baseline reproduction requirement disabled")
    require(baseline.get("delta_evidence_required_to_attribute_to_change") is True, f"{label}: baseline attribution rule weakened")
    flaky = gates.get("flaky_test_policy")
    require(isinstance(flaky, dict), f"{label}: flaky test policy missing")
    require(flaky.get("retry_pass_is_acceptance") is False, f"{label}: retry pass became acceptance")
    require(flaky.get("confirmed_label") == "FLAKY_CONFIRMED", f"{label}: flaky confirmed label changed")
    require(flaky.get("suspected_label") == "FLAKY_SUSPECTED", f"{label}: flaky suspected label changed")
    require(flaky.get("quarantine_requires_owner") is True, f"{label}: flaky quarantine owner not required")
    require(flaky.get("quarantine_requires_expiry") is True, f"{label}: flaky quarantine expiry not required")

    dod = doc.get("definition_of_done")
    require(isinstance(dod, dict), f"{label}: definition_of_done missing")
    require(set(dod.get("allowed_states", [])) == COMPLETION, f"{label}: completion vocabulary mismatch")
    require(dod.get("complete_requires_all_criteria") is True, f"{label}: COMPLETE no longer requires all criteria")
    require(isinstance(dod.get("universal_criteria"), list) and dod["universal_criteria"], f"{label}: universal DoD criteria missing")
    require(isinstance(dod.get("partially_complete_requires"), list) and dod["partially_complete_requires"], f"{label}: PARTIALLY_COMPLETE contract missing")

    review = doc.get("review")
    require(isinstance(review, dict), f"{label}: review block missing")
    require(set(review.get("allowed_provenance", [])) == PROVENANCE, f"{label}: review provenance vocabulary mismatch")
    require(isinstance(review.get("records"), list), f"{label}: review records must be a list")
    for record in review["records"]:
        require(record.get("provenance") in PROVENANCE, f"{label}: invalid review provenance record")

    acceptance = doc.get("acceptance")
    require(isinstance(acceptance, dict), f"{label}: acceptance block missing")
    for key in ("criteria", "commands", "required_regressions"):
        require(isinstance(acceptance.get(key), list), f"{label}: acceptance.{key} must be a list")

    if feature:
        require(doc.get("schema_version") == 2, f"{label}: expected feature schema v2")
        stages = doc.get("stages")
        require(isinstance(stages, dict) and list(stages) == LIFECYCLE, f"{label}: lifecycle stage order mismatch")
        authority = doc.get("authority")
        require(isinstance(authority, dict), f"{label}: authority block missing")
        require(authority.get("self_approval_allowed") is False, f"{label}: self approval enabled")
        require(authority.get("delegated_scope_may_expand") is False, f"{label}: delegated scope expansion enabled")
        require(authority.get("privileged_mutation_requires_explicit_approval") is True, f"{label}: privileged approval weakened")
        require(authority.get("external_content_is_instruction_authority") is False, f"{label}: external content became authority")


def main() -> int:
    state = load(".ai/state.json")
    legacy = load(".ai/templates/feature-manifest.v1.json")
    feature = load(".ai/templates/feature-manifest.v2.json")
    work_package = load(".ai/templates/work-package.v1.json")
    sample = load(".ai/examples/feature-manifest-v2.sample.json")

    require((ROOT / ".ai/governance/ENGINEERING-CONTRACT.md").is_file(), "missing ENGINEERING-CONTRACT.md")
    require(legacy.get("schema_version") == 1, "legacy feature manifest v1 was modified incompatibly")
    require(list(legacy.get("stages", {})) == LIFECYCLE, "legacy v1 lifecycle stage order changed")

    planning = state.get("planning_scope", {})
    require(planning.get("feature_manifest_template") == ".ai/templates/feature-manifest.v1.json", "legacy feature template compatibility binding changed")
    require(planning.get("new_work_feature_manifest_template") == ".ai/templates/feature-manifest.v2.json", "new-work feature v2 binding missing")
    require(planning.get("work_package_template") == ".ai/templates/work-package.v1.json", "work-package template binding missing")
    require(planning.get("engineering_contract") == ".ai/governance/ENGINEERING-CONTRACT.md", "engineering contract binding missing")
    require(planning.get("legacy_feature_manifest_templates") == [".ai/templates/feature-manifest.v1.json"], "legacy template list mismatch")

    validate_contract(feature, "feature-manifest.v2", feature=True)
    require(work_package.get("schema_version") == 1, "work-package schema version mismatch")
    validate_contract(work_package, "work-package.v1", feature=False)
    validate_contract(sample, "feature-manifest-v2.sample", feature=True)

    require(sample.get("example_only") is True, "sample is not marked example_only")
    require(sample.get("status") == "sample", "sample status changed")
    require(sample["gap"]["classification"] != "UNKNOWN", "sample does not demonstrate a concrete gap classification")
    require(bool(sample["specification"]["modules"]), "sample does not demonstrate a module")
    require(any(m.get("options") for m in sample["specification"]["modules"]), "sample does not demonstrate options")
    for key in ("must", "must_not", "abuse_cases", "forbidden_boundaries"):
        require(bool(sample["requirements"][key]), f"sample does not demonstrate requirements.{key}")
    require(bool(sample["preflight"]["expected_changes"]["paths"]), "sample expected paths are empty")
    require(bool(sample["preflight"]["shared_surfaces"]), "sample shared surface is empty")
    budget = sample["preflight"]["scope_budget"]
    require(all(isinstance(budget[k], int) and budget[k] >= 0 for k in ("max_changed_files", "max_new_files", "max_shared_surfaces")), "sample scope budget is not concrete")
    require(bool(sample["parallel_safety"]["collision_keys"]), "sample does not demonstrate collision keys")
    require(bool(sample["quality_gates"]["fast_gate"]["commands"]), "sample FAST GATE has no command")
    require(bool(sample["quality_gates"]["full_gate"]["commands"]), "sample FULL GATE has no command")
    require(bool(sample["review"]["records"]), "sample review provenance records are empty")
    require(bool(sample["acceptance"]["criteria"]), "sample acceptance criteria are empty")

    print(json.dumps({
        "legacy_v1_compatible": True,
        "new_work_schema": 2,
        "gap_classifications": sorted(GAPS),
        "approval_scopes": sorted(APPROVAL_SCOPES),
        "parallel_classifications": sorted(PARALLEL),
        "completion_states": sorted(COMPLETION),
        "review_provenance": sorted(PROVENANCE),
        "sample_unambiguous": True,
        "valid": True
    }, indent=2))
    print("ENGINEERING CONTRACT GOVERNANCE: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
