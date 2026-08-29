#!/usr/bin/env python3
from __future__ import annotations

import json
import re
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LIFECYCLE = ["research", "plan", "architecture", "data_flow", "security", "design", "qa", "performance", "development"]
ADOPTION_PLAN_STATES = {"NOT_STARTED", "PARTIALLY_IMPLEMENTED", "IMPLEMENTED_NOT_VERIFIED", "VERIFIED", "DIFFERS_FROM_PLAN", "UNKNOWN"}
ADOPTION_DOC_STATES = {"DOCUMENTED", "PARTIALLY_DOCUMENTED", "UNDOCUMENTED", "OBSOLETE", "UNKNOWN_PURPOSE"}
PROJECT_STATES = {"GREENFIELD", "PLANNED_EXISTING_PROJECT", "ACTIVE_EXISTING_PROJECT", "PRODUCTION_PROJECT", "LEGACY_OR_MIGRATION", "RECOVERY"}
CAPABILITY_STATES = {"AVAILABLE", "UNAVAILABLE", "UNKNOWN"}
CHECKPOINT_GATE_STATES = {"PENDING", "IN_PROGRESS", "SUCCESS", "FAILURE", "BLOCKED", "CANCELLED", "SKIPPED"}


def fail(message: str) -> None:
    raise SystemExit(f"AI planning governance failed: {message}")


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


def main() -> int:
    state = load(".ai/state.json")
    catalog = load(".ai/catalog/platforms.v1.json")
    starters = load(".ai/catalog/starter-intents.v1.json")
    legacy_manifest = load(".ai/templates/feature-manifest.v1.json")
    checkpoint = load(".ai/current-work.json")
    adoption = load(".ai/templates/adoption-audit.v1.json")
    capabilities = load(".ai/templates/capability-ledger.v1.json")

    required_governance = [
        ".ai/README.md", ".ai/current-work.json", ".ai/governance/LIFECYCLE.md",
        ".ai/governance/CHANGE-CONTROL.md", ".ai/governance/TRUST-BOUNDARIES.md",
        ".ai/governance/EVIDENCE.md", ".ai/governance/ADOPTION-RESUME.md",
        ".ai/agents/AGENTS.md", ".ai/templates/STAGE-TEMPLATES.md",
        ".ai/templates/feature-manifest.v1.json", ".ai/templates/adoption-audit.v1.json",
        ".ai/templates/capability-ledger.v1.json", ".ai/catalog/platforms.v1.json",
        ".ai/catalog/starter-intents.v1.json",
    ]
    for path in required_governance:
        require((ROOT / path).is_file(), f"missing governance artifact: {path}")

    # Preserve the bounded repository-local context and high-confidence secret scan from the original inline gate.
    secret_patterns = [
        (re.compile(r"-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----"), "private key"),
        (re.compile(r"\bgh[pousr]_[A-Za-z0-9]{20,}\b"), "GitHub token"),
        (re.compile(r"\bgithub_pat_[A-Za-z0-9_]{40,}\b"), "GitHub fine-grained token"),
        (re.compile(r"\bAKIA[0-9A-Z]{16}\b"), "AWS access key"),
    ]
    for path in (ROOT / ".ai").rglob("*"):
        if not path.is_file():
            continue
        require(path.stat().st_size <= 256 * 1024, f"AI context file too large: {path.relative_to(ROOT)}")
        text = path.read_text(encoding="utf-8", errors="replace")
        for pattern, label in secret_patterns:
            require(pattern.search(text) is None, f"possible {label} committed in {path.relative_to(ROOT)}")

    require(state.get("schema_version") == 3, "unexpected state schema version")
    require(state.get("governance_version") == 2, "unexpected governance version")
    require(state.get("lifecycle") == LIFECYCLE, "lifecycle order changed")
    addendum = state.get("governance_addendum", {})
    require(addendum.get("id") == "ENG-GOV-V3", "Engineering Governance V3 addendum is not bound")
    require(addendum.get("status") in {"IN_PROGRESS", "APPLIED"}, "invalid Engineering Governance V3 status")

    preflight = state.get("implementation_preflight", {})
    expected_preflight = {
        "read_frozen_plan": True,
        "verify_frozen_plan_digest": True,
        "refresh_live_canonical_state": True,
        "read_current_work_checkpoint": True,
        "refresh_live_wip_before_mutation": True,
        "market_delta_research": True,
        "untrusted_external_content_is_data": True,
        "silent_scope_drift_allowed": False,
        "approved_change_required_for_material_delta": True,
        "privileged_mutation_requires_explicit_approval": True,
        "delegation_may_expand_authority": False,
    }
    for key, expected in expected_preflight.items():
        require(preflight.get(key) is expected, f"implementation_preflight.{key} changed")

    canonical = state.get("canonical_state", {})
    require(canonical.get("cached_snapshot_authoritative") is False, "cached snapshot became authoritative")
    require(canonical.get("live_main_required") is True, "live main requirement disabled")
    require(canonical.get("refresh_before_each_stage") is True, "stage canonical refresh disabled")
    require(canonical.get("refresh_before_mutation") is True, "mutation canonical refresh disabled")
    require(canonical.get("mismatch_action") == "stop_and_reconcile", "canonical mismatch is not fail-closed")
    for path in canonical.get("sources", []):
        require((ROOT / path).is_file(), f"canonical source missing: {path}")
        require(not path.startswith("certification/"), "canonical source hardcodes a package tracker")
    resolution = canonical.get("active_tracker_resolution", {})
    require(resolution.get("directory") == "certification", "active tracker directory changed")
    require(resolution.get("package_id_source") == "docs/MASTER-EXECUTION-STATUS.json#active_package", "active tracker is not selected from live active_package")
    require(resolution.get("selector") == "tracker.package_id == active_package", "active tracker selector changed")
    require(resolution.get("requires_exactly_one_match") is True, "active tracker resolution is ambiguous by policy")
    require(canonical.get("designated_live_projections") == ["README.md", ".ai/README.md", "docs/MASTER-EXECUTION-PLAN.md"], "designated live projections changed")
    require(canonical.get("wip_checkpoint") == ".ai/current-work.json", "WIP checkpoint path changed")
    require(canonical.get("wip_checkpoint_authoritative") is False, "WIP checkpoint became authoritative")

    audit_baseline = state.get("audit_baseline", {})
    require(audit_baseline.get("semantics") == "historical_audit_baseline_only", "audit baseline semantics weakened")
    require(re.fullmatch(r"[0-9a-f]{40}", str(audit_baseline.get("audited_main", ""))) is not None, "invalid audited main SHA")

    planning = state.get("planning_scope", {})
    require(planning.get("status") == "blueprint_only", "planning scope claims implementation")
    require(planning.get("may_modify_active_product_task") is False, "planning may modify active product task")
    require(planning.get("may_change_frozen_active_package_sequence") is False, "planning may alter frozen active-package sequence")
    require(planning.get("adoption_audit_template") == ".ai/templates/adoption-audit.v1.json", "adoption audit template binding changed")
    require(planning.get("capability_ledger_template") == ".ai/templates/capability-ledger.v1.json", "capability ledger template binding changed")

    resume = state.get("resume_contract", {})
    require(resume.get("governance") == ".ai/governance/ADOPTION-RESUME.md", "adoption/resume governance binding changed")
    require(resume.get("checkpoint") == ".ai/current-work.json", "resume checkpoint binding changed")
    require(resume.get("acceptance_state_and_wip_are_distinct") is True, "canonical acceptance and WIP were conflated")
    require(resume.get("repository_evidence_over_checkpoint") is True, "checkpoint outranks repository evidence")

    # Final approved checkpoint contract: explicit last action, blockers and gates are first-class and still non-authoritative.
    require(checkpoint.get("schema_version") == 2, "unexpected current-work schema version")
    require(checkpoint.get("snapshot_semantics") == "NON_AUTHORITATIVE_CHECKPOINT_REFRESH_LIVE_STATE_BEFORE_ANY_MUTATION", "current-work authority semantics weakened")
    refresh = checkpoint.get("live_refresh", {})
    require(refresh.get("required_before_any_mutation") is True, "current-work does not require refresh before mutation")
    require(refresh.get("required_before_resume") is True, "current-work does not require refresh before resume")
    require(refresh.get("checkpoint_conflict_action") == "STOP_AND_RECONCILE", "checkpoint conflict is not fail-closed")
    semantics = checkpoint.get("state_semantics", {})
    require(semantics.get("repository_evidence_over_checkpoint") is True, "checkpoint outranks repository evidence")
    require(semantics.get("conversation_memory_authoritative") is False, "conversation memory became authoritative")
    last_action = checkpoint.get("last_verified_action")
    require(isinstance(last_action, dict), "checkpoint.last_verified_action missing")
    require(isinstance(last_action.get("action"), str) and bool(last_action.get("action")), "checkpoint.last_verified_action.action missing")
    require(re.fullmatch(r"[0-9a-f]{40}", str(last_action.get("verified_against_main", ""))) is not None, "checkpoint last verified main SHA invalid")
    require(isinstance(last_action.get("evidence_refs"), list), "checkpoint last verified evidence refs missing")
    blockers = checkpoint.get("blockers")
    require(isinstance(blockers, list), "checkpoint.blockers must be a list")
    for blocker in blockers:
        require(isinstance(blocker, dict) and bool(blocker.get("id")), "checkpoint blocker lacks id")
        require(isinstance(blocker.get("blocks"), list), "checkpoint blocker lacks blocked targets")
        require(blocker.get("status") in {"OPEN", "RESOLVED", "ACCEPTED_DEFERRED"}, "checkpoint blocker status invalid")
    gates = checkpoint.get("gates")
    require(isinstance(gates, list) and bool(gates), "checkpoint.gates missing or empty")
    gate_names: set[str] = set()
    for gate in gates:
        require(isinstance(gate, dict) and bool(gate.get("name")), "checkpoint gate lacks name")
        require(gate.get("status") in CHECKPOINT_GATE_STATES, f"checkpoint gate status invalid: {gate.get('name')}")
        require(gate["name"] not in gate_names, f"duplicate checkpoint gate: {gate['name']}")
        gate_names.add(gate["name"])

    # Exact approved adoption vocabulary; legacy alias is read-only compatibility, not the canonical token.
    require(adoption.get("schema_version") == 1, "unexpected adoption-audit schema version")
    require(set(adoption.get("allowed_plan_states", [])) == ADOPTION_PLAN_STATES, "adoption plan-state vocabulary changed")
    require(set(adoption.get("allowed_documentation_states", [])) == ADOPTION_DOC_STATES, "adoption documentation-state vocabulary changed")
    require(adoption.get("read_only") is True, "adoption audit template is not read-only by default")
    aliases = adoption.get("legacy_state_aliases", {})
    require(aliases.get("IMPLEMENTED_BUT_NOT_VERIFIED") == "IMPLEMENTED_NOT_VERIFIED", "legacy adoption-state alias missing")

    require(capabilities.get("schema_version") == 1, "unexpected capability-ledger schema version")
    require(set(capabilities.get("allowed_project_states", [])) == PROJECT_STATES, "project-state vocabulary changed")
    require(set(capabilities.get("allowed_capability_states", [])) == CAPABILITY_STATES, "capability-state vocabulary changed")
    require(capabilities.get("refresh_before_privileged_or_destructive_action") is True, "capability refresh before privileged/destructive action disabled")
    required_capabilities = {
        "repository_read", "repository_write", "filesystem", "terminal", "database", "tests", "vcs", "ci_cd",
        "deployment", "project_planner", "internet_research", "privileged_operations",
    }
    require(required_capabilities <= set(capabilities.get("capabilities", {})), "capability ledger lost required capability classes")

    # Legacy v1 remains compatibility authority for already accepted work.
    require(legacy_manifest.get("schema_version") == 1, "legacy feature manifest schema changed unexpectedly")
    require(list(legacy_manifest.get("stages", {})) == LIFECYCLE, "legacy manifest stage order differs from lifecycle")
    authority = legacy_manifest.get("authority", {})
    require(authority.get("self_approval_allowed") is False, "legacy manifest permits self-approval")
    require(authority.get("delegated_scope_may_expand") is False, "legacy manifest permits delegated scope expansion")
    require(authority.get("privileged_mutation_requires_explicit_approval") is True, "legacy manifest drops privileged approval")
    require(authority.get("external_content_is_instruction_authority") is False, "legacy manifest trusts external instructions")

    # Platform catalog remains planning metadata and cannot promote support by wording alone.
    require(catalog.get("schema_version") == 1, "catalog schema changed unexpectedly")
    platforms = catalog.get("platforms", [])
    require(len(platforms) >= 40, "broad initial platform catalog unexpectedly shrank")
    required_platform_fields = {
        "id", "name", "family", "mode", "status", "starter_profiles", "runtimes", "databases",
        "official_tooling", "external_account_required", "notes",
    }
    allowed_modes = {"local_native", "container_recommended", "saas_connected"}
    allowed_status = {"existing_certified", "proposed"}
    ids: list[str] = []
    all_profiles = set(catalog.get("starter_profile_policy", []))
    for platform in platforms:
        pid = platform.get("id")
        require(required_platform_fields <= set(platform), f"missing platform fields: {pid}")
        require(bool(pid) and pid == pid.lower(), f"invalid platform id: {pid}")
        require(platform.get("mode") in allowed_modes, f"invalid mode for {pid}")
        require(platform.get("status") in allowed_status, f"invalid status for {pid}")
        require(isinstance(platform.get("starter_profiles"), list) and bool(platform["starter_profiles"]), f"empty starter profile list: {pid}")
        require(isinstance(platform.get("external_account_required"), bool), f"invalid external-account flag: {pid}")
        if platform.get("mode") == "saas_connected":
            require(platform.get("external_account_required") is True, f"SaaS-connected platform lacks account boundary: {pid}")
        all_profiles.update(platform["starter_profiles"])
        ids.append(pid)
    require(len(ids) == len(set(ids)), "duplicate platform ids")

    certified = {p["id"] for p in platforms if p.get("status") == "existing_certified"}
    require(certified == {"laravel", "node", "django", "rust", "go"}, f"unexpected certified platform set: {sorted(certified)}")
    provider_policy = catalog.get("provider_policy", {})
    require(provider_policy.get("preserve_existing_0207_templates") is True, "02.07 preservation disabled")
    require(provider_policy.get("certify_each_provider_before_supported_claim") is True, "provider certification rule disabled")
    require(provider_policy.get("market_delta_research_before_implementation") is True, "provider delta research disabled")

    require(starters.get("schema_version") == 1, "starter-intent schema changed unexpectedly")
    intents = [item["id"] for item in starters.get("normalized_intents", [])]
    require(len(intents) == len(set(intents)), "duplicate normalized starter intents")
    starter_policy = starters.get("policy", {})
    require(starter_policy.get("catalog_profile_labels_are_execution_authority") is False, "catalog profile labels became executable authority")
    require(starter_policy.get("provider_plan_must_map_profile_to_normalized_intent") is True, "provider mapping requirement disabled")
    require(starter_policy.get("unknown_profile_blocks_implementation") is True, "unknown starter profile no longer fails closed")
    profile_aliases = starters.get("profile_aliases", {})
    for profile in sorted(all_profiles):
        require(profile in profile_aliases, f"unmapped starter profile: {profile}")
        require(profile_aliases[profile] in intents, f"starter profile maps to unknown normalized intent: {profile}")

    today = date.today()
    for source in catalog.get("research_sources", []):
        require(str(source.get("url", "")).startswith("https://"), f"non-HTTPS research source: {source.get('url')}")
        verified = date.fromisoformat(source["verified"])
        require(verified <= today, f"research source verified in the future: {source.get('url')}")

    print(json.dumps({
        "state_schema": state["schema_version"],
        "lifecycle": LIFECYCLE,
        "adoption_resume": "enforced",
        "checkpoint_authority": "non-authoritative",
        "checkpoint_last_action": "required",
        "checkpoint_blockers": "required",
        "checkpoint_gates": "required",
        "adoption_plan_states": sorted(ADOPTION_PLAN_STATES),
        "platforms": len(platforms),
        "starter_profiles_mapped": len(all_profiles),
        "normalized_intents": len(intents),
        "certified": sorted(certified),
        "modes": sorted({p["mode"] for p in platforms}),
        "trust_boundary": "enforced",
        "frozen_plan_digest": "required",
        "valid": True,
    }, indent=2))
    print("AI PLANNING GOVERNANCE: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
