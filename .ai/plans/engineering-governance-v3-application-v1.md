# Engineering Governance V3 — Application Plan v1

Status: **APPROVED — NOT YET APPLIED**
Approval reference: `conversation:user-2026-08-28-engineering-governance-v3-approved`
Audit base: `4f33813bec4254107e6027e98b2a4a8878b9198b`

## Goal

Apply the 22 approved governance findings in a controlled sequence while VSN product development remains paused. Preserve existing exact-source evidence, frozen plans, task denominators, task dependencies and accepted behavior.

## Current pause boundary

At approval time:

- canonical package: `PKG-03 — Windows Installer`
- canonical progress: `10/25 = 40%`
- deterministic cursor: `03.11`
- 03.11 Linear: `ABD-86`
- 03.11 branch: `pkg03/0311-agent-service-install`
- 03.11 draft PR: `#125`
- 03.11 planning head: `b3d0b7d3e7f763ffb5847df1f3301c002a1d5a06`
- 03.11 planning gates had passed; product implementation had NOT started
- 03.15 had genuine green task evidence on its isolated branch but had not been reconciled/merged into current canonical state
- development is intentionally paused for this governance amendment

These values are a checkpoint, not future authority. Refresh all live state before mutation/resume.

## Milestone GOV-V3.0 — Canonical Adoption & Resume Safety

### Scope

Apply findings 1–7.

### Required changes

1. Generalize `.ai/state.json` canonical sources so generic governance resolves the live active package/tracker rather than hardcoding PKG-02.
2. Correct stale live/current-state language in `.ai/README.md` and `docs/MASTER-EXECUTION-PLAN.md`; mark historical-only documents clearly.
3. Expand Repository Governance to validate all designated live state projections against `docs/MASTER-EXECUTION-STATUS.json` and the active tracker.
4. Introduce/normalize `.ai/current-work.json` as a non-authoritative WIP/checkpoint snapshot with mandatory live-refresh semantics.
5. Add adoption-audit schema/template for plan<->repository and documentation-state comparison.
6. Add project-state/capability-ledger schema/template.
7. Define canonical acceptance state vs WIP registry semantics and ensure resume logic reads both without confusing them.

### Acceptance

- no product/runtime behavior changes;
- PKG-03 denominator/order/evidence unchanged;
- active tracker still derives from live canonical status;
- intentionally stale/historical docs are labeled historical, not validated as live projections;
- designated live projections fail Repository Governance when mismatched;
- `.ai/current-work.json` explicitly declares itself non-authoritative and contains a refresh requirement;
- existing AI Planning Governance, Repository Governance and PKG-03 Acceptance Sequence pass on final head;
- diff audit confirms only approved governance/docs/state-support surfaces changed.

### Exit

Merge GOV-V3.0 to `main` before GOV-V3.1.

## Milestone GOV-V3.1 — Engineering Contract Hardening

### Scope

Apply findings 8–17.

### Required changes

1. Extend feature/work-package templates with gap classification.
2. Extend approval metadata with approval scope, inherited authorization and reapproval triggers.
3. Add applicable module/option specification template fields.
4. Add first-class `must_not`/abuse-case/forbidden-boundary requirements.
5. Add expected-change/shared-surface/scope-budget preflight fields and STOP_AND_REASSESS behavior.
6. Add parallel-safety classification and collision rules.
7. Define FAST GATE vs FULL GATE in lifecycle/QA governance.
8. Define `BASELINE_FAILURE` and flaky-test policy.
9. Define universal DoD and `PARTIALLY_COMPLETE`.
10. Define review provenance labels.

### Acceptance

- existing accepted feature manifests remain valid or receive backwards-compatible interpretation;
- no retroactive invalidation of certified PKG-01/02/03 evidence;
- no new product scope;
- governance/templates/docs checks pass;
- a sample new feature manifest can express every added field without ambiguity.

### Exit

Merge GOV-V3.1 to `main` before GOV-V3.2.

## Milestone GOV-V3.2 — Operational & Release Maturity

### Scope

Apply findings 18–22.

### Required changes

1. Add operational-readiness specification block.
2. Add release states and recovery classification.
3. Add incident-mode governance.
4. Add stop-the-line conditions and evidence preservation.
5. Add durable end-of-task report/checkpoint schema and unrelated-finding/tech-debt queue semantics.

### Acceptance

- release states cannot be conflated;
- irreversible actions require explicit authorization;
- incident/stop-line states block normal feature mutation;
- checkpoint schema is sufficient for a new AI to resume without old chat;
- no secrets or sensitive logs are persisted;
- governance checks pass.

## Final V3 audit

After all three milestones are merged:

1. Re-run a read-only audit against all 22 findings.
2. Mark every finding `APPLIED`, `PARTIALLY_APPLIED`, or `BLOCKED` with evidence.
3. Do not call V3 complete while any P0 finding is not APPLIED.
4. Refresh live PKG-03 state/open PRs/Linear.
5. Reconcile the paused 03.11 branch with latest canonical `main`.
6. Re-run 03.11 planning/preflight governance.
7. Resume 03.11 product implementation from its accepted plan, not from chat memory.

## Do-not-forget checklist

- [ ] GOV-V3.0 findings 1–7 applied and merged
- [ ] GOV-V3.0 exact governance checks green
- [ ] GOV-V3.1 findings 8–17 applied and merged
- [ ] GOV-V3.1 exact governance checks green
- [ ] GOV-V3.2 findings 18–22 applied and merged
- [ ] GOV-V3.2 exact governance checks green
- [ ] Final 22-item audit persisted
- [ ] Live canonical PKG-03 state re-read
- [ ] 03.11 branch reconciled from latest main
- [ ] 03.11 planning gates rerun
- [ ] Development pause explicitly lifted
- [ ] 03.11 implementation resumed

No checkbox may be marked complete from chat claims alone.
