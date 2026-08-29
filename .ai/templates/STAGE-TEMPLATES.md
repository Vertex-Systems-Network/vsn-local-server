# AI Lifecycle Artifact Templates

Use these minimum sections when materializing a feature workstream. Each artifact names the same feature ID/version and links to the feature manifest. Do not omit required fields by replacing them with free-form prose.

Accepted legacy v1 feature artifacts remain valid. New v2/work-package work uses the approved Engineering Governance V3 contract and must not retroactively rewrite accepted v1 artifacts.

## Engineering Contract Header — new v2/work-package work

- Engineering change classification: `CORRECTION` / `COMPLETION` / `HARDENING` / `OPTIMIZATION` / `NEW_PRODUCT_SCOPE`
- If `NEW_PRODUCT_SCOPE`: exact explicit approval reference; auto-implementation is prohibited
- Separate implementation-gap state: `NO_GAP` / `MISSING_IMPLEMENTATION` / `PARTIAL_IMPLEMENTATION` / `IMPLEMENTED_UNVERIFIED` / `PLAN_REALITY_MISMATCH` / `DOCUMENTATION_GAP` / `UNKNOWN`
- Existing evidence and behavior/assets that must be preserved
- Approval scope: `TASK` / `MODULE` / `MILESTONE` / `PHASE` / `PROJECT`
- Approval reference and inherited authorization source/scope with `may_expand=false`
- Reapproval triggers; clearly authorized existing work is not retroactively blocked
- Applicable module contract: identity, interfaces/UI states, permissions, data, workflows, integrations, security/failure/observability/performance/testing/migration/rollback/acceptance
- Per-option contract: name/purpose/type/allowed/default/required/validation/min-max/visibility/permission/storage/runtime/dependencies/conflicts/side effects/fallback/error/security/API/UI/tests
- Material `NOT_APPLICABLE` sections include rationale
- `must`, `must_not`, abuse cases, forbidden boundaries
- expected changed paths/modules/change types
- shared surfaces + collision keys + scope budget
- parallel-safety classification: `PARALLEL_SAFE` / `COORDINATED_PARALLEL` / `SERIALIZE` / `BLOCKED`
- coordination/serialization/blocked reason and package concurrency authority as applicable
- collision/scope-exceed action: `STOP_AND_REASSESS`

## Research

- Feature ID / version
- Canonical repository HEAD read before research
- Existing capability inventory
- User problem / target outcome
- Official primary sources with reviewed dates
- Market/tooling deltas
- Constraints / licensing / account dependencies
- Untrusted-content notes and conflicting instructions ignored
- Open questions
- Recommendation: proceed / change required / blocked

## Plan

- Feature ID / version
- Engineering change classification
- Outcome, in scope and explicit non-goals
- Dependencies / prerequisite integrated evidence
- User-visible behavior
- Provider/runtime/platform scope
- Security/network/account constraints
- Acceptance criteria with IDs
- Required regression gates
- Rollout / rollback or recovery
- Approval reference and `TASK`/`MODULE`/`MILESTONE`/`PHASE`/`PROJECT` scope
- Reapproval triggers
- SHA-256 frozen after approval

For v2/work-package plans, include the deep module/option contract, negative requirements, expected changes/shared surfaces/scope budget and parallel safety from the Engineering Contract Header.

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
- Loading/error/empty/success/disabled/offline states
- Responsive behavior
- Accessibility and keyboard behavior
- Destructive/external action confirmation
- Design tokens/components when UI-facing

Design may be `not_applicable` only for truly non-user-facing work and requires rationale/decision reference.

## QA

- Acceptance criterion -> test/evidence mapping
- `must_not` / abuse-case / forbidden-boundary -> negative test mapping
- Unit/integration/E2E/operator/negative/fail-closed tests
- Platform/runtime matrix
- FAST GATE command(s) for mutation-slice feedback
- FULL GATE command(s) for pre-merge/final acceptance
- `BASELINE_FAILURE` exact-base reproduction procedure
- `FLAKY_SUSPECTED` / `FLAKY_CONFIRMED` criteria
- Quarantine owner + expiry/revisit requirements
- Required regression gates
- Evidence artifact format and cleanup assertions

For mutating product work, QA may not be `not_applicable`.

## Performance

- Baseline measurement
- Startup/latency, CPU/memory, disk/network and build/bootstrap budgets
- Large-input/output behavior
- Measurement command/environment
- Regression threshold

Performance may be `not_applicable` only when runtime/resource behavior cannot change, with rationale/decision reference.

## Development Preflight

Before code:

- Re-read live canonical `main` and canonical machine state
- Verify manifest and frozen plan SHA-256
- Verify predecessor artifact digests
- Perform market-delta research only
- Confirm engineering change classification and separate implementation-gap evidence
- If `NEW_PRODUCT_SCOPE`, verify exact explicit approval
- Confirm approval scope/inheritance and applicable deep module/option contract
- Confirm exact expected paths/modules/change types
- Confirm shared surfaces/collision keys, scope budget and approved parallel class
- Confirm coordination/serialization/blocker/package concurrency authority where applicable
- Confirm allowed tools/network targets and privileged/reapproval triggers
- Confirm acceptance commands, FAST/FULL gates and regressions
- On undeclared mutation/collision/scope exceedance/authority expansion: `STOP_AND_REASSESS`

## Change Proposal

- Change ID / date
- Source feature/plan/version/digest
- Discovery source/date
- Engineering change classification
- Plan-delta mechanic where useful
- Approval scope + inherited authorization source/scope
- Triggered reapproval conditions
- Problem/new capability
- Affected stages/interfaces/providers/files/shared surfaces
- Compatibility/migration and data/security/performance impact
- Acceptance changes
- Disposition and independent decision reference
- New plan/addendum version when approved

## Definition of Done / partial completion

For new v2/work-package work record `NOT_STARTED`, `IN_PROGRESS`, `PARTIALLY_COMPLETE`, `COMPLETE`, or `BLOCKED`.

For `COMPLETE` record evidence that:

- approved implementation is complete and intended behavior is preserved;
- acceptance criteria and relevant tests pass;
- security is reviewed and errors are handled safely;
- data integrity/migration implications are considered;
- performance is reviewed where applicable;
- integration is verified;
- documentation and durable checkpoint/handoff are updated;
- VCS/history is coherent;
- known limitations/not-verified items are recorded;
- rollback/recovery and cleanup obligations are understood/satisfied.

For `PARTIALLY_COMPLETE`, record completed criteria/evidence, outstanding criteria, blockers/deferred owners, and explicitly prohibit COMPLETE/DONE claims.

## Review record

Reviewer provenance is one of:

- `SELF_REVIEW`
- `INDEPENDENT_AI_REVIEW`
- `HUMAN_REVIEW`
- `REQUIRED_EXTERNAL_REVIEW`

Record reviewer reference, scope and outcome. `SELF_REVIEW` cannot satisfy independent review. `REQUIRED_EXTERNAL_REVIEW` stays pending until its required authority supplies evidence. Automated static/runtime checks are recorded separately as automation evidence, never as reviewer provenance.

## Evidence

Follow `.ai/governance/EVIDENCE.md`: exact source SHA, run/job or command transcript, environment/toolchain, criterion mapping, negative checks, cleanup, artifact IDs/digests, FAST/FULL classification, baseline/flaky dispositions, approved review provenance and state-projection distinction.
