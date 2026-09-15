# FAST-DEVELOPMENT-GOVERNANCE-V1

Date: 2026-09-16
Disposition: APPROVED
Engineering change classification: OPTIMIZATION
Plan-delta mechanic: CONTRACT CHANGE (execution timing/coordination only)
Approval scope: PROJECT
Approval reference: `conversation:user-2026-09-16-fast-development-implementation`
Source canonical main: `48b9ad1cb74b9e2e90606b3c6dd3b6e77a15e2ad`

## Problem

The repository's acceptance model is intentionally evidence-driven, but current implementation throughput is constrained by two independent effects:

1. canonically blocked downstream tasks cannot perform useful implementation even when their interfaces can be built safely against explicit non-production fixtures;
2. completed-package whole-project regressions are repeatedly triggered by task-local governance/certification changes, creating exact-head churn without adding proportional risk coverage.

The result is idle implementation capacity while external signing/Store/provider/review gates remain pending.

## Approved change

Adopt `.ai/governance/FAST-DEVELOPMENT.md` as a project-level execution addendum.

The addendum introduces a non-canonical `PREIMPLEMENTATION` lane, short integration epochs and a FAST / INTEGRATION FULL / RELEASE-CERTIFICATION CI tier model. It explicitly preserves the existing canonical dependency DAG and final acceptance requirements.

## What does not change

- no product/runtime/installer/updater/network/security behavior is changed by this governance change;
- no target-system permission, privilege, file path, service, trust-store, hosts/resolver, firewall or IPC behavior changes;
- no package denominator/order/dependency/status changes;
- no task can become DONE from fixture/synthetic/test-signed evidence;
- no external provider, Store, signing, certificate, notarization or human review is fabricated;
- no security or QA stage is skipped;
- no accepted historical evidence is rewritten;
- PKG-03 remains 21/25 with 03.22 as the canonical READY task until real acceptance changes it.

## Approval / reapproval analysis

The change affects execution timing and coordination, so it is treated conservatively as a contract change. The user explicitly approved implementation provided project/target-system behavior and flow are not harmed. This proposal constrains the change to orchestration/CI behavior and keeps acceptance semantics invariant.

Reapproval is required if implementation later attempts any of:

- canonical dependency/denominator/status changes;
- product scope expansion;
- privilege or data-flow expansion;
- weaker security/QA acceptance;
- treating PREIMPLEMENTATION evidence as final acceptance;
- changing production release/signing/provider authority;
- removing a required final/release gate rather than routing irrelevant PR execution.

## Initial bounded implementation surfaces

Phase 1:

- `.ai/governance/FAST-DEVELOPMENT.md`
- `.ai/changes/FAST-DEVELOPMENT-GOVERNANCE-V1.md`
- `scripts/ci/live-work-state.py`
- `scripts/ci/validate-fast-development.py`
- `.github/workflows/fast-development-governance.yml`

No `apps/**`, `crates/**`, `cloud/**`, `packaging/**`, canonical tracker/status, installer/signing implementation, release state or secret-bearing surface is authorized in this phase.

## CI optimization rollout rule

Do not bulk-edit completed-package workflow triggers blindly. First establish the live-state/fast-governance contract and validate a conservative changed-path classifier. Then route only PRs whose changed paths are proven unrelated to the completed package. Shared product/toolchain/lockfile changes continue to trigger impacted completed-package regression, and release/certification gates remain available.

## Rollback

If this execution profile creates ambiguity or unsafe integration behavior, stop PREIMPLEMENTATION creation and fall back to the prior serialized lifecycle. Because canonical tracker/status and product behavior are unchanged, rollback is governance-only and does not require target-system migration.
