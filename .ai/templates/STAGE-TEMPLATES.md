# AI Lifecycle Artifact Templates

Use these minimum sections when materializing a feature workstream. Each artifact names the same feature ID/version and links to the feature manifest. Do not omit required fields by replacing them with free-form prose.

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
- Approval reference
- SHA-256 frozen after approval

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
- Unit tests
- Integration tests
- E2E/operator tests
- Negative/fail-closed tests
- Platform/runtime matrix
- Determinism / retries / flake policy
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
- Verify feature manifest and frozen plan SHA-256
- Verify predecessor stage statuses/artifact digests
- Perform market-delta research only
- Confirm delta = none/informational, or stop for approved change
- Confirm exact files/modules and allowed tools/network targets
- Confirm privileged actions requiring approval
- Confirm acceptance commands and required regressions

## Change Proposal

- Change ID / date
- Source feature/plan/version/digest
- Discovery source/date
- Change class
- Problem/new capability
- Affected stages/interfaces/providers/files
- Compatibility/migration impact
- Data/security/performance impact
- Acceptance changes
- Disposition
- Independent approval/decision reference
- New plan/addendum version if approved

## Evidence

Follow `.ai/governance/EVIDENCE.md`: exact source SHA, run/job or command transcript, environment/toolchain, criterion mapping, negative checks, cleanup, artifact IDs/digests and state-projection distinction.
