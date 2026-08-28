# Engineering Contract Governance

Status: active contract for new materially planned feature/work-package work. Historical accepted v1 manifests and certification evidence remain valid and are not retroactively rewritten.

## Compatibility boundary

- `.ai/templates/feature-manifest.v1.json` remains a valid legacy contract for already approved/accepted work.
- New materially planned work uses `.ai/templates/feature-manifest.v2.json` and, when decomposed, `.ai/templates/work-package.v1.json`.
- Do not rewrite or re-hash accepted v1 evidence merely to add Governance V3 fields.
- If legacy work needs a material scope/permission/data-flow/acceptance change, create an approved v2 addendum/work package and preserve the original accepted artifact.
- Missing Governance V3 fields in historical accepted v1 artifacts mean `LEGACY_NOT_REQUIRED`, not retroactive failure.

## 1. Change classification and implementation gap

Every new feature/work package declares exactly one approved engineering change classification:

- `CORRECTION`
- `COMPLETION`
- `HARDENING`
- `OPTIMIZATION`
- `NEW_PRODUCT_SCOPE`

`NEW_PRODUCT_SCOPE` is never auto-implemented. It requires an explicit approval reference covering the new behavior before development. A generic desire to improve the product is not sufficient authorization.

Change classification is distinct from implementation-gap state. The optional/parallel plan-reality vocabulary (`NO_GAP`, `MISSING_IMPLEMENTATION`, `PARTIAL_IMPLEMENTATION`, `IMPLEMENTED_UNVERIFIED`, `PLAN_REALITY_MISMATCH`, `DOCUMENTATION_GAP`, `UNKNOWN`) may be used to describe what currently exists, but it does not replace the approved change classification. `UNKNOWN` never authorizes guessed mutation.

## 2. Approval scope and inheritance

Approval scope is one of:

- `TASK`
- `MODULE`
- `MILESTONE`
- `PHASE`
- `PROJECT`

Approval records include the approval reference, inherited existing authorization and reapproval triggers. Inherited authority is monotonic: `may_expand=false`. A child work package, delegated agent, branch or task cannot gain authority absent from its source approval.

Clearly authorized existing work does not require retroactive reapproval merely because Governance V3 was adopted. Reapproval is required when any of these occurs:

- `SCOPE_EXPANSION`
- `PRIVILEGE_EXPANSION`
- `DATA_FLOW_CHANGE`
- `SECURITY_ASSUMPTION_CHANGE`
- `ACCEPTANCE_CHANGE`
- `DEPENDENCY_CHANGE`
- `SHARED_SURFACE_COLLISION`
- `ROLLOUT_CHANGE`
- `IRREVERSIBLE_ACTION`

A textual AI statement is not an approval reference. Privileged actions remain subject to the explicit authority rules even when they occur within an approved task/module/milestone/phase/project.

## 3. Module and option specification

Every applicable substantial product module is documented before implementation. A feature-level module contract defines, as applicable:

- **identity** — module name/ID, purpose, business objective, actors, dependencies, scope and non-goals;
- **interfaces** — pages/screens, forms, tables, tabs, filters, search, actions, bulk actions, modals/drawers, empty/loading/error/success/disabled states, responsive behavior and accessibility;
- **permissions** — view/create/update/delete/approve/export/configure/administer plus server-side enforcement where applicable;
- **data** — entities, fields, relationships, constraints, ownership/tenant scope, deletion, retention, auditing, migrations and existing-data impact;
- **workflows** — trigger/actor/preconditions/validation/authorization/processing/state/data/events/jobs/notifications/success/failure/retry/cancel/recovery/concurrency as applicable;
- **integrations** — APIs/webhooks/external services, authentication, timeout/retry/rate-limit/idempotency/failure behavior as applicable;
- **engineering** — security, failure handling, observability, performance, testing, migration, rollback and acceptance.

A not-applicable material section requires rationale; do not fabricate details just to fill a template. Decomposed work packages reference the approved parent module contract and record bounded module changes/option overrides rather than silently redefining it.

Every meaningful option/setting records, where relevant: name, purpose, type, allowed values, default, required/optional, validation, min/max, visibility, required permission, storage, runtime behavior, dependencies, conflicts, side effects, fallback, error behavior, security implications, API representation, UI representation and tests.

Do not opportunistically implement undocumented options.

## 4. Positive and negative requirements

Every new contract carries four first-class requirement sets:

- `must` — required behavior/invariants;
- `must_not` — forbidden outcomes or regressions;
- `abuse_cases` — hostile/misuse paths that must be handled safely;
- `forbidden_boundaries` — components, permissions, data, networks, repositories or external systems the work may not cross.

Important negative requirements are acceptance obligations and should map to tests/evidence.

## 5. Expected change, shared surfaces and scope budget

Before mutation, the preflight declares expected paths/modules/change types, shared surfaces and a reasonable scope budget. Shared surfaces receive collision keys so parallel work can detect overlap.

If actual work exceeds the approved scope budget, touches an undeclared shared surface, or needs an unexpected mutation class, the required action is `STOP_AND_REASSESS`. Do not normalize scope expansion after the fact. Budgets are guardrails, not incentives to split logically atomic changes or hide files.

## 6. Parallel safety

Every feature/work package uses exactly one approved class:

- `PARALLEL_SAFE` — declared changes have no conflicting mutable ownership and satisfy package-specific concurrency rules;
- `COORDINATED_PARALLEL` — parallel work may continue only under an explicit coordination plan for named shared surfaces/collision keys;
- `SERIALIZE` — affected work must execute/integrate in declared order;
- `BLOCKED` — work must not proceed until the named blocker is removed.

Record shared surfaces, collision keys and, when applicable, coordination plan, serialization order, blocked reason and the package/project concurrency-authority reference. Package-specific concurrency limits remain authoritative and cannot be widened by this generic classification. A new collision requires `STOP_AND_REASSESS` unless the approved contract already defines deterministic coordination/serialization. Branch separation alone does not prove parallel safety.

## 7. FAST GATE and FULL GATE

### FAST GATE

Runs after each meaningful mutation slice. It is narrow and fast: targeted syntax/type/unit/contract checks for the touched surface plus required safety checks. FAST GATE is feedback, not final acceptance.

### FULL GATE

Runs before merge and final acceptance. It includes required regression, integration/E2E/negative/governance/platform checks defined by the approved contract.

A FAST GATE pass cannot substitute for a required FULL GATE.

## 8. Baseline failures and flaky tests

`BASELINE_FAILURE` means a failing check is reproducible on the exact canonical base independent of the candidate change. It requires reproduction evidence from that base. Without base reproduction, do not relabel a candidate failure as baseline.

A proven baseline failure remains subject to the active acceptance contract; proof of pre-existence is not automatic permission to merge.

Flaky tests use `FLAKY_SUSPECTED` or `FLAKY_CONFIRMED`. A retry that happens to pass is not acceptance. Quarantine requires owner, reason, bounded scope and expiry/revisit condition. Never delete, disable or weaken a failing test merely to obtain green status.

## 9. Universal Definition of Done

Completion state is one of `NOT_STARTED`, `IN_PROGRESS`, `PARTIALLY_COMPLETE`, `COMPLETE`, or `BLOCKED`.

`COMPLETE` requires all applicable approved implementation and intended behavior, acceptance criteria and relevant tests, security review, safe error handling, data-integrity/migration consideration, performance review where applicable, integration verification, documentation and durable checkpoint/handoff updates, coherent VCS/history, recorded known limitations/not-verified items, and understood rollback/recovery plus applicable cleanup obligations.

`PARTIALLY_COMPLETE` requires explicit completed criteria with evidence, outstanding criteria, blockers/deferred items with ownership, and a prohibition on any COMPLETE/DONE claim. It is not a softer synonym for complete.

## 10. Review provenance

Material review records use one of the approved provenance labels:

- `SELF_REVIEW`
- `INDEPENDENT_AI_REVIEW`
- `HUMAN_REVIEW`
- `REQUIRED_EXTERNAL_REVIEW`

`SELF_REVIEW` cannot be represented as independent review and cannot satisfy a required external/human approval. `REQUIRED_EXTERNAL_REVIEW` records a review requirement and remains pending until the required reviewer/authority produces evidence.

Automated static/runtime checks are evidence, not review-person provenance; record them separately as automation evidence so they are never represented as human or independent AI review.

## Stop conditions

Stop affected mutation and reassess when approval scope is unclear, inherited authority would expand, `NEW_PRODUCT_SCOPE` lacks explicit approval, implementation gap is materially `UNKNOWN`, a deep required module/option contract is incomplete, scope budget is exceeded, a new shared-surface collision appears, parallel class is `BLOCKED`, a forbidden boundary would be crossed, or a required FULL GATE/external review cannot be produced.
