#!/usr/bin/env python3
import json
from pathlib import Path


def require(condition, message):
    if not condition:
        raise SystemExit(f"Operational governance failed: {message}")


def load(path):
    p = Path(path)
    require(p.is_file(), f"missing required artifact: {path}")
    return json.loads(p.read_text(encoding="utf-8"))


release_states = ["BUILT", "DEPLOYED", "RELEASED", "PRODUCTION_VERIFIED"]
recovery_classes = ["SIMPLE_ROLLBACK", "ROLLBACK_WITH_COMPATIBILITY", "FORWARD_FIX_PREFERRED", "IRREVERSIBLE"]
incident_states = ["NONE", "STOP_THE_LINE", "INCIDENT_ACTIVE", "RECOVERED_PENDING_RCA"]

operational = load(".ai/templates/operational-readiness.v1.json")
end_task = load(".ai/templates/end-task-report.v1.json")
tech_debt = load(".ai/templates/tech-debt-item.v1.json")
feature = load(".ai/templates/feature-manifest.v2.json")
sample = load(".ai/examples/feature-manifest-v2.sample.json")
state = load(".ai/state.json")
checkpoint = load(".ai/current-work.json")

for path in [
    ".ai/governance/OPERATIONS-RELEASE.md",
    ".ai/governance/INCIDENT-STOP-LINE.md",
]:
    require(Path(path).is_file(), f"missing governance contract: {path}")

# Operational readiness contract.
require(operational["schema_version"] == 1, "unexpected operational-readiness schema")
require(operational["release"]["allowed_states"] == release_states, "release-state order changed")
require(operational["release"]["state_conflation_allowed"] is False, "release states may be conflated")
require(operational["recovery"]["allowed_classes"] == recovery_classes, "recovery classes changed")
require(operational["sensitive_logging"]["secrets_allowed"] is False, "secrets allowed in operational logs")
require(operational["sensitive_logging"]["authorization_headers_allowed"] is False, "authorization headers allowed in operational logs")
require(operational["sensitive_logging"]["redaction_required"] is True, "operational redaction disabled")

# Feature manifest carries operational/release/incident contracts.
require(feature["schema_version"] == 2, "feature manifest schema changed")
ops = feature["operational_readiness"]
require(ops["template"] == ".ai/templates/operational-readiness.v1.json", "feature operational template binding changed")
rr = feature["release_and_recovery"]
require(rr["allowed_release_states"] == release_states, "feature release states changed")
require(rr["release_state_conflation_allowed"] is False, "feature permits release-state conflation")
require(rr["allowed_recovery_classes"] == recovery_classes, "feature recovery classes changed")
incident = feature["incident_and_stop_line"]
require(incident["allowed_states"] == incident_states, "feature incident states changed")
require(incident["normal_feature_mutation_allowed_when_stopped"] is False, "stop-line permits normal feature mutation")
require(feature["end_task_reporting"]["template"] == ".ai/templates/end-task-report.v1.json", "end-task report binding changed")
require(feature["unrelated_findings"]["template"] == ".ai/templates/tech-debt-item.v1.json", "tech-debt template binding changed")
require(feature["unrelated_findings"]["fix_without_scope_change_allowed"] is False, "unrelated findings may be fixed without scope change")

# Concrete sample proves values are usable, not only placeholders.
require(sample["release_and_recovery"]["release_state"] in release_states, "sample release state invalid")
require(sample["release_and_recovery"]["recovery_classification"] in recovery_classes, "sample recovery class invalid")
require(sample["incident_and_stop_line"]["current_state"] in incident_states, "sample incident state invalid")
require(sample["incident_and_stop_line"]["normal_feature_mutation_allowed_when_stopped"] is False, "sample weakens stop-line")
require(sample["operational_readiness"]["artifact"], "sample operational artifact missing")

# Durable end-task/handoff contract.
require(end_task["schema_version"] == 1, "unexpected end-task schema")
for key in [
    "changed", "why", "research_performed", "tests_and_checks", "security",
    "data_and_migration", "affected_areas", "vcs_and_commits",
    "documentation_and_memory_updated", "known_issues", "not_verified", "next_safe_action"
]:
    require(key in end_task["report"], f"end-task report missing {key}")
require(end_task["release"]["allowed_states"] == release_states, "end-task release states changed")
require(end_task["recovery"]["allowed_classes"] == recovery_classes, "end-task recovery classes changed")
require(end_task["incident"]["allowed_states"] == incident_states, "end-task incident states changed")
require(end_task["redaction"]["secrets_persisted"] is False, "end-task permits secret persistence")
require(end_task["redaction"]["sensitive_values_persisted"] is False, "end-task permits sensitive-value persistence")

# Unrelated findings remain recorded/deferred until separately authorized.
require(tech_debt["schema_version"] == 1, "unexpected tech-debt schema")
require(tech_debt["current_scope"]["authorized_to_fix_now"] is False, "tech-debt template authorizes unrelated cleanup")
require(tech_debt["current_scope"]["scope_change_required_before_mutation"] is True, "tech-debt scope change is not required")
require(tech_debt["security"]["contains_secret_values"] is False, "tech-debt template permits secret values")

# Incident and release prose must retain the critical fail-closed rules.
ops_text = Path(".ai/governance/OPERATIONS-RELEASE.md").read_text(encoding="utf-8")
incident_text = Path(".ai/governance/INCIDENT-STOP-LINE.md").read_text(encoding="utf-8")
for token in ["BUILT", "DEPLOYED", "RELEASED", "PRODUCTION_VERIFIED", "IRREVERSIBLE", "explicit approval"]:
    require(token in ops_text, f"operations/release contract missing token: {token}")
for token in ["STOP_THE_LINE", "INCIDENT_ACTIVE", "STABILIZE", "CONTAIN", "PRESERVE_EVIDENCE", "ROOT_CAUSE"]:
    require(token in incident_text, f"incident contract missing token: {token}")

# Resume must be possible without old chat, while repository evidence still wins.
require(checkpoint["snapshot_semantics"] == "NON_AUTHORITATIVE_CHECKPOINT_REFRESH_LIVE_STATE_BEFORE_ANY_MUTATION", "checkpoint authority semantics weakened")
require(checkpoint["live_refresh"]["required_before_any_mutation"] is True, "checkpoint does not require live refresh")
require(checkpoint["state_semantics"]["conversation_memory_authoritative"] is False, "conversation memory became authoritative")
require(bool(checkpoint.get("exact_next_safe_action")), "checkpoint lacks exact next safe action")

# Before Governance V3 finalization the product-development pause is mandatory.
# After finalization an explicit unpaused checkpoint is permitted only when canonical
# state proves the full final audit/closeout contract. Task-specific approval and gates
# remain separate and are not granted by this validator.
product_paused = checkpoint["pause"]["product_development_paused"]
require(isinstance(product_paused, bool), "product development pause must be boolean")
if product_paused is False:
    addendum = state.get("governance_addendum", {})
    final_audit = state.get("final_audit", {})
    require(addendum.get("id") == "ENG-GOV-V3", "unpaused checkpoint lacks Governance V3 authority")
    require(addendum.get("status") == "APPLIED", "product development resumed before Governance V3 was APPLIED")
    require(addendum.get("target_milestone") == "PRODUCT_RESUME_RECONCILIATION", "product development resumed outside post-finalization reconciliation")
    require(final_audit.get("status") == "APPLIED", "product development resumed before final audit was APPLIED")
    require(final_audit.get("applied") == 22, "product development resumed without all 22 governance findings APPLIED")
    require(final_audit.get("partially_applied") == 0, "product development resumed with partially applied governance findings")
    require(final_audit.get("blocked") == 0, "product development resumed with blocked governance findings")
    require(final_audit.get("remediation_required") == [], "product development resumed with outstanding governance remediation")
    require(final_audit.get("second_read_only_audit_required") is False, "product development resumed before required final read-only audit completed")

# State must bind this validator once P2 is being applied.
planning = state["planning_scope"]
require(planning["operational_readiness_template"] == ".ai/templates/operational-readiness.v1.json", "state operational template binding missing")
require(planning["end_task_report_template"] == ".ai/templates/end-task-report.v1.json", "state end-task template binding missing")
require(planning["tech_debt_template"] == ".ai/templates/tech-debt-item.v1.json", "state tech-debt template binding missing")
require(planning["operational_governance"]["validator"] == "scripts/operational-governance.py", "state operational validator binding changed")
require(planning["operational_governance"]["workflow"] == ".github/workflows/operational-governance.yml", "state operational workflow binding changed")

print(json.dumps({
    "release_states": release_states,
    "recovery_classes": recovery_classes,
    "incident_states": incident_states,
    "operational_readiness": "enforced",
    "stop_line": "fail_closed",
    "durable_handoff": "enforced",
    "unrelated_cleanup": "scope_change_required",
    "product_development_paused": product_paused,
    "post_finalization_resume_policy": "fail_closed",
    "valid": True
}, indent=2))
