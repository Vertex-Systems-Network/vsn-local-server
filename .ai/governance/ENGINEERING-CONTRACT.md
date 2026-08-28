# Engineering Contract Governance

Status: active contract for new feature/work-package planning after GOV-V3.1 becomes canonical. It does not retroactively invalidate accepted v1 manifests or certification evidence.

## Compatibility boundary

- `.ai/templates/feature-manifest.v1.json` remains a valid legacy contract for already approved/accepted work.
- New materially planned work uses `.ai/templates/feature-manifest.v2.json` and, when decomposed, `.ai/templates/work-package.v1.json`.
- Do not rewrite or re-hash an accepted v1 manifest merely to add V3.1 fields.
- If legacy work needs a material scope/permission/data-flow/acceptance change, create an approved v2 addendum/work package; preserve the original accepted artifact.
- Missing V3.1 fields in historical accepted v1 artifacts mean `LEGACY_NOT_REQUIRED`, not failure or `PARTIALLY_COMPLETE`.

## 1. Gap classification

Every new feature/work package declares exactly one current gap classification:

- `NO_GAP` — requested outcome already exists and is verified; avoid duplicate implementation.
- `MISSING_IMPLEMENTATION` — planned capability is absent.
- `PARTIAL_IMPLEMENTATION` — some required behavior exists but the approved outcome is incomplete.
- `IMPLEMENTED_UNVERIFIED` — implementation exists but acceptance evidence is insufficient.
- `PLAN_REALITY_MISMATCH` — repository behavior differs materially from the approved plan.
- `DOCUMENTATION_GAP` — implementation is adequate but documentation/plan representation is incomplete or stale.
- `UNKNOWN` — evidence is insufficient; inspect before mutation.

The contract records existing evidence, work that must be preserved, and plan amendments required. `UNKNOWN` never authorizes guessed mutation.

## 2. Approval scope and inheritance

Approval scope is explicit: `TASK`, `WORK_PACKAGE`, `FEATURE`, `PROJECT`, `RELEASE`, or `PRIVILEGED_ACTION`.

Inherited authorization records its source and inherited scope. Inheritance is monotonic: `may_expand=false`. A child work package, delegated agent, branch or task cannot gain authority not present in its source approval.

Independent reapproval is required when any of these triggers occurs:

- `SCOPE_EXPANSION`
- `PRIVILEGE_EXPANSION`
- `DATA_FLOW_CHANGE`
- `SECURITY_ASSUMPTION_CHANGE`
- `ACCEPTANCE_CHANGE`
- `DEPENDENCY_CHANGE`
- `SHARED_SURFACE_COLLISION`
- `ROLLOUT_CHANGE`
- `IRREVERSIBLE_ACTION`

A textual AI statement is not an approval reference.

## 3. Module and option specification

Applicable work defines modules and options before implementation. Each module/option declares applicability and a value/behavior contract. Use `REQUIRED`, `OPTIONAL`, or `NOT_APPLICABLE`; a not-applicable item needs a defensible rationale in the plan/stage artifact when material.

Do not implement unspecified options opportunistically. Unknown option behavior is a planning gap, not permission to invent defaults.

## 4. Positive and negative requirements

Every new contract carries four first-class requirement sets:

- `must` — required behavior/invariants;
- `must_not` — forbidden outcomes or regressions;
- `abuse_cases` — hostile/misuse paths that must be handled safely;
- `forbidden_boundaries` — components, permissions, data, networks, repositories or external systems the work may not cross.

Negative requirements are acceptance criteria, not prose-only warnings.

## 5. Expected change, shared surfaces and scope budget

Before mutation, the preflight declares expected paths/modules/change types, shared surfaces and a scope budget. Shared surfaces receive collision keys so parallel work can detect overlap.

If actual work exceeds the approved scope budget, touches an undeclared shared surface, or needs an unexpected mutation class, the required action is `STOP_AND_REASSESS`. Do not normalize scope expansion after the fact.

Budgets are guardrails, not incentives to split logically atomic changes or hide files.

## 6. Parallel safety

Every feature/work package is classified as:

- `PARALLEL_SAFE` — declared changes do not share mutable ownership/collision keys with concurrent work;
- `SERIALIZE_SHARED_SURFACE` — independent work may proceed, but named shared surfaces must be serialized/reconciled;
- `EXCLUSIVE` — mutation must not run concurrently with another overlapping lane.

A collision requires `STOP_AND_REASSESS` unless the approved contract already defines deterministic serialization. Branch separation alone does not prove parallel safety.

## 7. FAST GATE and FULL GATE

### FAST GATE

Runs after each meaningful mutation slice. It is narrow and fast: targeted syntax/type/unit/contract checks for the touched surface plus required safety checks. FAST GATE is feedback, not final acceptance.

### FULL GATE

Runs before merge and final acceptance. It includes the required regression suite, integration/E2E/negative checks, governance/evidence checks and any platform matrix required by the plan.

A FAST GATE pass cannot substitute for a required FULL GATE.

## 8. Baseline failures and flaky tests

`BASELINE_FAILURE` means a failing check is reproducible on the exact canonical base independent of the candidate change. It requires reproduction evidence from that base. Without base reproduction, do not relabel a candidate failure as baseline.

A baseline failure does not automatically permit merge: the active acceptance contract decides whether it blocks, requires remediation, or can be explicitly dispositioned.

Flaky tests use `FLAKY_SUSPECTED` or `FLAKY_CONFIRMED`. A retry that happens to pass is not acceptance. Quarantine requires an owner, reason, bounded scope and expiry/revisit condition. Never delete/disable a failing test merely to obtain green status.

## 9. Universal Definition of Done

Completion state is one of `NOT_STARTED`, `IN_PROGRESS`, `PARTIALLY_COMPLETE`, `COMPLETE`, or `BLOCKED`.

`COMPLETE` requires all applicable approved scope mapped, required gates green or validly dispositioned baseline failures, required negative/fail-closed tests, documentation/evidence updates, and cleanup/rollback obligations satisfied.

`PARTIALLY_COMPLETE` requires explicit completed criteria with evidence, outstanding criteria, blockers/deferred items with ownership, and a prohibition on any COMPLETE/DONE claim. It is not a softer synonym for complete.

## 10. Review provenance

Every material review record declares provenance:

- `HUMAN_REVIEW`
- `AI_SELF_REVIEW`
- `AI_INDEPENDENT_REVIEW`
- `AUTOMATED_STATIC`
- `AUTOMATED_RUNTIME`

Provenance does not imply authority. AI self-review cannot satisfy an independent-human approval requirement; automated checks cannot be represented as human review.

## Stop conditions

Stop affected mutation and reassess when approval scope is unclear, inherited authority would expand, gap state is `UNKNOWN` on a material surface, scope budget is exceeded, a new shared-surface collision appears, a forbidden boundary would be crossed, or a required FULL GATE cannot be produced.
