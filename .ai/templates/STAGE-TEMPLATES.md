# AI Lifecycle Artifact Templates

Use these minimum sections when materializing a feature workstream. Each artifact names the same feature ID/version and links to the feature manifest. Do not omit required fields by replacing them with free-form prose.

Accepted legacy v1 feature artifacts remain valid. For new v2/work-package work, also use the Engineering Contract fields below; do not retroactively rewrite accepted v1 artifacts just to add them.

## Engineering Contract Header — new v2/work-package work

- Gap classification: `NO_GAP` / `MISSING_IMPLEMENTATION` / `PARTIAL_IMPLEMENTATION` / `IMPLEMENTED_UNVERIFIED` / `PLAN_REALITY_MISMATCH` / `DOCUMENTATION_GAP` / `UNKNOWN`
- Existing evidence and behavior/assets that must be preserved
- Approval scope: `TASK` / `WORK_PACKAGE` / `FEATURE` / `PROJECT` / `RELEASE` / `PRIVILEGED_ACTION`
- Approval reference
- Inherited authorization source + exact inherited scope + `may_expand=false`
- Reapproval triggers
- Applicable modules
- Per-module options: applicability (`REQUIRED`, `OPTIONAL`, `NOT_APPLICABLE`), value/behavior contract, default and constraints
- `must`
- `must_not`
- abuse cases
- forbidden boundaries
- expected changed paths/modules/change types
- shared surfaces + collision keys
- scope budget
- parallel-safety classification: `PARALLEL_SAFE` / `SERIALIZE_SHARED_SURFACE` / `EXCLUSIVE`
- collision/scope-exceed action: `STOP_AND_REASSESS`

## Research

- Feature ID / version
- Canonical repository HEAD read before research
- Existing VSN capability inventory
- User problem / target outcome
- Official primary sources with reviewed dates
- Market/tooling deltas
- Constraints / licensing / account dependencies
- Untrusted-content notes and conflicting instructions ignored
- Open questions
- Recommendation: proceed / change required / blocked

## Plan

- Feature ID / version
- Outcome
- In scope
- Explicit non-goals
- Dependencies / prerequisite integrated evidence
- User-visible behavior
- Provider/runtime/platform scope
- Security/network/account constraints
- Acceptance criteria with IDs
- Required regression gates
- Rollout / rollback
- Approval reference and scope
- Reapproval triggers
- SHA-256 frozen after approval

For v2/work-package plans, include module/option specification, negative requirements, expected changes/shared surfaces/scope budget and parallel safety from the Engineering Contract Header.

## Architecture

- Existing seams reused first
- Components and ownership
- Interfaces / contracts / schemas
- Provider/plugin boundaries
- Runtime/process boundaries
- Failure modes and fail-closed behavior
- Portability / upgrade strategy
- ADR references
- Files/modules expected to change
- Shared mutable surfaces and collision keys

## Data Flow

- Inputs and origin
- Transformations
- Persistence locations
- IPC/process/network paths
- External SaaS/account boundaries
- Secret references/handles; never secret values
- Logging/telemetry paths
- Retention/deletion/cleanup
- Trust-boundary diagram or structured equivalent

## Security

- Assets and threat actors
- Prompt-injection/untrusted-content paths
- Least-privilege permissions
- Delegation boundary
- File/process/network containment
- Supply-chain/scaffold verification
- Secret and identity handling
- SaaS mutation/tunnel/public-listener approval points
- Negative security tests
- Abuse cases and forbidden-boundary tests
- Residual risk and explicit acceptance owner

For mutating product work, Security may not be `not_applicable`.

## Design

- Desktop/web/CLI/API user flow
- Information architecture
- Progressive disclosure
- Review-before-mutate surface
- Loading/error/empty/offline states
- Accessibility and keyboard behavior
- Destructive/external action confirmation
- Design tokens/components when UI-facing

Design may be `not_applicable` only for truly non-user-facing work and requires a decision reference.

## QA

- Acceptance criterion -> test/evidence mapping
- `must_not` / abuse-case / forbidden-boundary -> negative test mapping
- Unit tests
- Integration tests
- E2E/operator tests
- Negative/fail-closed tests
- Platform/runtime matrix
- Determinism / retries / flake policy
- FAST GATE command(s) for mutation-slice feedback
- FULL GATE command(s) for pre-merge/final acceptance
- `BASELINE_FAILURE` reproduction procedure against exact canonical base
- `FLAKY_SUSPECTED` / `FLAKY_CONFIRMED` criteria
- Quarantine owner + expiry/revisit requirements
- Required regression gates
- Evidence artifact format
- Cleanup assertions

For mutating product work, QA may not be `not_applicable`.

## Performance

- Baseline measurement
- Startup/latency budget
- CPU/memory budget
- Disk/network budget
- Build/bootstrap budget
- Large-input/output behavior
- Measurement command/environment
- Regression threshold

Performance may be `not_applicable` only when no runtime/resource behavior can change, with a decision reference.

## Development Preflight

Before code:

- Re-read live canonical `main` and canonical machine state
- Verify feature/work-package manifest and frozen plan SHA-256
- Verify predecessor stage statuses/artifact digests
- Perform market-delta research only
- Confirm delta = none/informational, or stop for approved change
- Confirm gap classification and existing evidence for v2/work-package work
- Confirm exact expected paths/modules/change types
- Confirm shared surfaces/collision keys and parallel-safety class
- Confirm scope budget
- Confirm allowed tools/network targets and inherited approval scope
- Confirm privileged actions and reapproval triggers
- Confirm acceptance commands, FAST/FULL gates and required regressions
- If an undeclared mutation/shared collision/scope exceedance is required: `STOP_AND_REASSESS`

## Change Proposal

- Change ID / date
- Source feature/plan/version/digest
- Discovery source/date
- Change class
- Approval scope + inherited authorization source/scope
- Triggered reapproval conditions
- Problem/new capability
- Affected stages/interfaces/providers/files/shared surfaces
- Compatibility/migration impact
- Data/security/performance impact
- Acceptance changes
- Disposition
- Independent approval/decision reference
- New plan/addendum version if approved

## Definition of Done / partial completion

For new v2/work-package work record one state: `NOT_STARTED`, `IN_PROGRESS`, `PARTIALLY_COMPLETE`, `COMPLETE`, or `BLOCKED`.

For `COMPLETE` record proof that:

- approved scope is fully mapped;
- required FAST/FULL gates are green or an evidence-backed baseline failure has an allowed disposition;
- negative/fail-closed tests pass;
- documentation/evidence is updated;
- cleanup/rollback obligations are satisfied where applicable.

For `PARTIALLY_COMPLETE` record:

- completed criteria + evidence;
- outstanding criteria;
- blockers/deferred items + owners;
- explicit statement that work is not COMPLETE/DONE.

## Review record

- Provenance: `HUMAN_REVIEW` / `AI_SELF_REVIEW` / `AI_INDEPENDENT_REVIEW` / `AUTOMATED_STATIC` / `AUTOMATED_RUNTIME`
- Reviewer/tool reference
- Reviewed scope
- Outcome/findings
- Whether the review is independent approval authority (only when separately permitted by governance)

Do not represent AI self-review/automated checks as human review.

## Evidence

Follow `.ai/governance/EVIDENCE.md`: exact source SHA, run/job or command transcript, environment/toolchain, criterion mapping, negative checks, cleanup, artifact IDs/digests, FAST/FULL classification, baseline/flaky dispositions, review provenance and state-projection distinction.
