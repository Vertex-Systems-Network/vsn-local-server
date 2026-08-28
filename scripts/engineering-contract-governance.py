#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

IMPLEMENTATION_GAPS = {
    "NO_GAP", "MISSING_IMPLEMENTATION", "PARTIAL_IMPLEMENTATION",
    "IMPLEMENTED_UNVERIFIED", "PLAN_REALITY_MISMATCH",
    "DOCUMENTATION_GAP", "UNKNOWN",
}
CHANGE_CLASSES = {"CORRECTION", "COMPLETION", "HARDENING", "OPTIMIZATION", "NEW_PRODUCT_SCOPE"}
APPROVAL_SCOPES = {"TASK", "MODULE", "MILESTONE", "PHASE", "PROJECT"}
REAPPROVAL_TRIGGERS = {
    "SCOPE_EXPANSION", "PRIVILEGE_EXPANSION", "DATA_FLOW_CHANGE",
    "SECURITY_ASSUMPTION_CHANGE", "ACCEPTANCE_CHANGE", "DEPENDENCY_CHANGE",
    "SHARED_SURFACE_COLLISION", "ROLLOUT_CHANGE", "IRREVERSIBLE_ACTION",
}
PARALLEL = {"PARALLEL_SAFE", "COORDINATED_PARALLEL", "SERIALIZE", "BLOCKED"}
COMPLETION = {"NOT_STARTED", "IN_PROGRESS", "PARTIALLY_COMPLETE", "COMPLETE", "BLOCKED"}
PROVENANCE = {"SELF_REVIEW", "INDEPENDENT_AI_REVIEW", "HUMAN_REVIEW", "REQUIRED_EXTERNAL_REVIEW"}
AUTOMATION = {"AUTOMATED_STATIC", "AUTOMATED_RUNTIME"}
LIFECYCLE = ["research", "plan", "architecture", "data_flow", "security", "design", "qa", "performance", "development"]
DOD = {
    "approved implementation is complete and intended behavior is preserved",
    "acceptance criteria are satisfied and relevant tests are executed",
    "security is reviewed and errors are handled safely",
    "data integrity and migration implications are considered",
    "performance is reviewed where applicable",
    "integration is verified",
    "documentation and durable checkpoint or handoff are updated",
    "VCS/history is updated with coherent changes",
    "known limitations and not-verified items are recorded",
    "rollback or recovery is understood and applicable cleanup obligations are satisfied",
}
MODULE_IDENTITY = {"purpose", "business_objective", "actors", "dependencies", "scope", "non_goals"}
MODULE_INTERFACES = {
    "pages_or_screens", "forms", "tables", "tabs", "filters", "search", "actions", "bulk_actions",
    "modals_or_drawers", "empty_states", "loading_states", "error_states", "success_states", "disabled_states",
    "responsive_behavior", "accessibility",
}
MODULE_PERMISSIONS = {"view", "create", "update", "delete", "approve", "export", "configure", "administer", "server_side_enforcement_required"}
MODULE_DATA = {"entities", "fields", "relationships", "constraints", "ownership", "tenant_or_workspace_scope", "deletion", "retention", "auditing", "migrations", "existing_data_impact"}
MODULE_ENGINEERING = {"security", "failure_handling", "observability", "performance", "testing", "migration", "rollback", "acceptance"}
OPTION_KEYS = {
    "id", "name", "purpose", "applicability", "non_applicable_rationale", "type", "allowed_values", "default",
    "required", "validation", "min", "max", "visibility", "required_permission", "storage", "runtime_behavior",
    "dependencies", "conflicts", "side_effects", "fallback", "error_behavior", "security_implications",
    "api_representation", "ui_representation", "tests",
}


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


def require_keys(obj: dict, keys: set[str], label: str) -> None:
    require(keys <= set(obj), f"{label}: missing keys {sorted(keys - set(obj))}")


def validate_module(module: dict, label: str) -> None:
    require_keys(module, {"id", "name", "applicability", "non_applicable_rationale", "identity", "interfaces", "permissions", "data", "workflows", "integrations", "engineering", "options"}, label)
    require(module["applicability"] in {"REQUIRED", "OPTIONAL", "NOT_APPLICABLE"}, f"{label}: invalid applicability")
    if module["applicability"] == "NOT_APPLICABLE":
        require(bool(module["non_applicable_rationale"]), f"{label}: not-applicable module lacks rationale")
        return
    require_keys(module["identity"], MODULE_IDENTITY, f"{label}.identity")
    require_keys(module["interfaces"], MODULE_INTERFACES, f"{label}.interfaces")
    require_keys(module["permissions"], MODULE_PERMISSIONS, f"{label}.permissions")
    require_keys(module["data"], MODULE_DATA, f"{label}.data")
    require(isinstance(module["workflows"], list), f"{label}.workflows must be a list")
    require(isinstance(module["integrations"], list), f"{label}.integrations must be a list")
    require_keys(module["engineering"], MODULE_ENGINEERING, f"{label}.engineering")
    require(isinstance(module["options"], list), f"{label}.options must be a list")
    for index, option in enumerate(module["options"]):
        olabel = f"{label}.options[{index}]"
        require_keys(option, OPTION_KEYS, olabel)
        require(option["applicability"] in {"REQUIRED", "OPTIONAL", "NOT_APPLICABLE"}, f"{olabel}: invalid applicability")
        if option["applicability"] == "NOT_APPLICABLE":
            require(bool(option["non_applicable_rationale"]), f"{olabel}: not-applicable option lacks rationale")


def validate_contract(doc: dict, label: str, *, feature: bool) -> None:
    change = doc.get("change_classification")
    require(isinstance(change, dict), f"{label}: missing change_classification block")
    require(set(change.get("allowed_classifications", [])) == CHANGE_CLASSES, f"{label}: change classification vocabulary mismatch")
    require(change.get("classification") in CHANGE_CLASSES, f"{label}: invalid change classification")
    require(change.get("auto_implement_new_product_scope") is False, f"{label}: NEW_PRODUCT_SCOPE auto implementation enabled")
    require("explicit_approval_ref_if_new_product_scope" in change, f"{label}: new-product approval field missing")
    if change.get("classification") == "NEW_PRODUCT_SCOPE":
        require(bool(change.get("explicit_approval_ref_if_new_product_scope")), f"{label}: NEW_PRODUCT_SCOPE lacks explicit approval")

    gap = doc.get("gap")
    require(isinstance(gap, dict), f"{label}: missing implementation gap block")
    require(set(gap.get("allowed_classifications", [])) == IMPLEMENTATION_GAPS, f"{label}: implementation gap vocabulary mismatch")
    require(gap.get("classification") in IMPLEMENTATION_GAPS, f"{label}: invalid implementation gap")

    approval = doc.get("approval")
    require(isinstance(approval, dict), f"{label}: missing approval block")
    require(set(approval.get("allowed_scopes", [])) == APPROVAL_SCOPES, f"{label}: approval scope vocabulary mismatch")
    require(approval.get("scope") in APPROVAL_SCOPES, f"{label}: invalid approval scope")
    inherited = approval.get("inherited_authorization")
    require(isinstance(inherited, dict), f"{label}: missing inherited authorization")
    require(inherited.get("may_expand") is False, f"{label}: inherited authorization may expand")
    require(approval.get("clearly_authorized_existing_work_requires_retroactive_reapproval") is False, f"{label}: clearly authorized existing work was retroactively blocked")
    require(set(approval.get("reapproval_triggers", [])) == REAPPROVAL_TRIGGERS, f"{label}: reapproval trigger set mismatch")

    spec = doc.get("specification")
    require(isinstance(spec, dict) and isinstance(spec.get("modules"), list), f"{label}: module specification missing")
    if feature:
        require(bool(spec["modules"]), f"{label}: feature module specification is empty")
        for index, module in enumerate(spec["modules"]):
            require(isinstance(module, dict), f"{label}: invalid module entry")
            validate_module(module, f"{label}.modules[{index}]")
    else:
        for index, module in enumerate(spec["modules"]):
            mlabel = f"{label}.modules[{index}]"
            require_keys(module, {"id", "applicability", "non_applicable_rationale", "module_contract_ref", "changes", "option_overrides"}, mlabel)
            require(module["applicability"] in {"REQUIRED", "OPTIONAL", "NOT_APPLICABLE"}, f"{mlabel}: invalid applicability")
            if module["applicability"] == "NOT_APPLICABLE":
                require(bool(module["non_applicable_rationale"]), f"{mlabel}: not-applicable module lacks rationale")
            else:
                require(isinstance(module["module_contract_ref"], str) and bool(module["module_contract_ref"]), f"{mlabel}: parent deep module contract reference missing")
            require(isinstance(module["changes"], list), f"{mlabel}.changes must be a list")
            require(isinstance(module["option_overrides"], list), f"{mlabel}.option_overrides must be a list")

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
    require_keys(budget, {"max_changed_files", "max_new_files", "max_shared_surfaces", "notes"}, f"{label}.scope_budget")
    require(preflight.get("scope_exceeded_action") == "STOP_AND_REASSESS", f"{label}: scope budget does not fail closed")

    parallel = doc.get("parallel_safety")
    require(isinstance(parallel, dict), f"{label}: parallel_safety block missing")
    require(set(parallel.get("allowed_classifications", [])) == PARALLEL, f"{label}: parallel classification vocabulary mismatch")
    require(parallel.get("classification") in PARALLEL, f"{label}: invalid parallel classification")
    for key in ("shared_surfaces", "collision_keys", "serialization_order"):
        require(isinstance(parallel.get(key), list), f"{label}: parallel_safety.{key} must be a list")
    for key in ("coordination_plan", "blocked_reason", "package_concurrency_authority_ref"):
        require(key in parallel, f"{label}: parallel_safety.{key} missing")
    require(parallel.get("on_collision") == "STOP_AND_REASSESS", f"{label}: collision does not stop/reassess")

    gates = doc.get("quality_gates")
    require(isinstance(gates, dict), f"{label}: quality_gates missing")
    for gate_name in ("fast_gate", "full_gate"):
        gate = gates.get(gate_name)
        require(isinstance(gate, dict), f"{label}: {gate_name} missing")
        for key in ("required_on", "commands", "evidence"):
            require(isinstance(gate.get(key), list), f"{label}: {gate_name}.{key} must be a list")
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
    require(set(dod.get("universal_criteria", [])) == DOD, f"{label}: universal DoD is incomplete or changed")
    require(dod.get("complete_requires_all_criteria") is True, f"{label}: COMPLETE no longer requires all criteria")
    require(isinstance(dod.get("partially_complete_requires"), list) and dod["partially_complete_requires"], f"{label}: PARTIALLY_COMPLETE contract missing")

    review = doc.get("review")
    require(isinstance(review, dict), f"{label}: review block missing")
    require(set(review.get("allowed_provenance", [])) == PROVENANCE, f"{label}: review provenance vocabulary mismatch")
    require(review.get("self_review_may_satisfy_independent_review") is False, f"{label}: self-review may satisfy independent review")
    require(isinstance(review.get("records"), list), f"{label}: review records must be a list")
    for record in review["records"]:
        require(record.get("provenance") in PROVENANCE, f"{label}: invalid review provenance record")
    require(isinstance(review.get("automation_evidence"), list), f"{label}: automation_evidence must be a list")
    for record in review["automation_evidence"]:
        require(record.get("kind") in AUTOMATION, f"{label}: invalid automation evidence kind")

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
    require(sample["change_classification"]["classification"] != "NEW_PRODUCT_SCOPE", "sample unexpectedly demonstrates unapproved new product scope")
    require(sample["gap"]["classification"] != "UNKNOWN", "sample does not demonstrate a concrete implementation gap")
    module = sample["specification"]["modules"][0]
    require(bool(module["identity"]["purpose"]), "sample module identity is not concrete")
    require(bool(module["interfaces"]["error_states"]), "sample module error states are not concrete")
    require(bool(module["permissions"]["view"]), "sample module permissions are not concrete")
    require(bool(module["data"]["entities"]), "sample module data contract is not concrete")
    require(bool(module["workflows"]), "sample module workflows are not concrete")
    require(bool(module["integrations"]), "sample module integrations are not concrete")
    require(bool(module["engineering"]["security"]), "sample module security is not concrete")
    require(bool(module["engineering"]["testing"]), "sample module testing is not concrete")
    require(bool(module["engineering"]["rollback"]), "sample module rollback is not concrete")
    require(bool(module["engineering"]["acceptance"]), "sample module acceptance is not concrete")
    require(bool(module["options"]), "sample does not demonstrate a deep option")
    option = module["options"][0]
    require(bool(option["purpose"]) and bool(option["tests"]), "sample option contract is not concrete")
    for key in ("must", "must_not", "abuse_cases", "forbidden_boundaries"):
        require(bool(sample["requirements"][key]), f"sample does not demonstrate requirements.{key}")
    require(bool(sample["preflight"]["expected_changes"]["paths"]), "sample expected paths are empty")
    require(bool(sample["preflight"]["shared_surfaces"]), "sample shared surface is empty")
    budget = sample["preflight"]["scope_budget"]
    require(all(isinstance(budget[k], int) and budget[k] >= 0 for k in ("max_changed_files", "max_new_files", "max_shared_surfaces")), "sample scope budget is not concrete")
    require(sample["parallel_safety"]["classification"] == "COORDINATED_PARALLEL", "sample does not demonstrate coordinated parallel work")
    require(bool(sample["parallel_safety"]["coordination_plan"]), "sample coordination plan is empty")
    require(bool(sample["parallel_safety"]["package_concurrency_authority_ref"]), "sample package concurrency authority is empty")
    require(bool(sample["quality_gates"]["fast_gate"]["commands"]), "sample FAST GATE has no command")
    require(bool(sample["quality_gates"]["full_gate"]["commands"]), "sample FULL GATE has no command")
    sample_review = {r["provenance"] for r in sample["review"]["records"]}
    require({"SELF_REVIEW", "HUMAN_REVIEW", "REQUIRED_EXTERNAL_REVIEW"} <= sample_review, "sample review provenance is incomplete")
    require(bool(sample["review"]["automation_evidence"]), "sample automation evidence is empty")
    require(bool(sample["acceptance"]["criteria"]), "sample acceptance criteria are empty")

    print(json.dumps({
        "legacy_v1_compatible": True,
        "new_work_schema": 2,
        "change_classifications": sorted(CHANGE_CLASSES),
        "implementation_gap_states": sorted(IMPLEMENTATION_GAPS),
        "approval_scopes": sorted(APPROVAL_SCOPES),
        "parallel_classifications": sorted(PARALLEL),
        "completion_states": sorted(COMPLETION),
        "review_provenance": sorted(PROVENANCE),
        "deep_module_contract": True,
        "sample_unambiguous": True,
        "valid": True
    }, indent=2))
    print("ENGINEERING CONTRACT GOVERNANCE: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
