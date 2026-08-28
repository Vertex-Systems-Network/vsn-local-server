# Adoption & Resume Governance

Status: active governance contract for existing-project adoption and safe resume.

## Purpose

Prevent AI/human work from restarting an existing project, trusting stale chat state, or confusing canonical acceptance with live work-in-progress.

## Project-state classification

Before substantial work, classify the repository as exactly one of:

- `GREENFIELD`
- `PLANNED_EXISTING_PROJECT`
- `ACTIVE_EXISTING_PROJECT`
- `PRODUCTION_PROJECT`
- `LEGACY_OR_MIGRATION`
- `RECOVERY`

The classification must be evidence-backed and stored in the applicable adoption audit/capability ledger or checkpoint. Unknown evidence stays `UNKNOWN`; do not guess.

## Source authority

When state conflicts, use this order:

1. actual repository/code/config/schema;
2. observed execution;
3. executed tests;
4. CI/CD result;
5. VCS history;
6. approved documentation;
7. non-authoritative checkpoint/WIP snapshot;
8. prior conversation.

The active package is resolved from `docs/MASTER-EXECUTION-STATUS.json#active_package`. The active package tracker is the unique `certification/*.json` document whose `package_id` equals that active package. A zero-match or multi-match condition is a stop-and-reconcile failure.

## Existing-project adoption

For existing projects use:

`Inspect -> Baseline -> Audit Existing Plan -> Compare Plan With Reality -> Identify Gaps -> Amend Plan -> Preserve Existing Work -> Continue Safely`

Do not restart the project, replace working architecture without evidence, overwrite approved plans, or discard unknown work.

Use `.ai/templates/adoption-audit.v1.json` to record:

- plan -> repository state as `NOT_STARTED`, `PARTIALLY_IMPLEMENTED`, `IMPLEMENTED_BUT_NOT_VERIFIED`, `VERIFIED`, `DIFFERS_FROM_PLAN`, or `UNKNOWN`;
- repository -> documentation state as `DOCUMENTED`, `PARTIALLY_DOCUMENTED`, `UNDOCUMENTED`, `OBSOLETE`, or `UNKNOWN_PURPOSE`.

## Capability ledger

Use `.ai/templates/capability-ledger.v1.json` when tool/privilege availability materially affects the plan. Record capabilities as `AVAILABLE`, `UNAVAILABLE`, or `UNKNOWN` with evidence. Never convert absence of evidence into capability.

## Canonical acceptance vs WIP

Canonical acceptance state and live work-in-progress are intentionally separate:

- package trackers and `docs/MASTER-EXECUTION-STATUS.json` describe accepted canonical completion/readiness;
- `.ai/current-work.json` describes non-authoritative WIP/checkpoint state such as open branches, PRs, current stage, blockers, and exact next safe action.

An open branch/PR MUST NOT make a task canonically DONE. A READY canonical task MAY have an active WIP lane without changing tracker acceptance state.

## Resume sequence

When the user says `continue` or `resume`, do not resume from chat memory alone. Perform:

1. read `.ai/state.json`;
2. read `.ai/current-work.json` as a checkpoint only;
3. fetch live canonical `main` SHA;
4. read live `docs/MASTER-EXECUTION-STATUS.json`;
5. resolve the unique active tracker;
6. refresh open relevant branches/PRs/issues and compare them with the checkpoint;
7. inspect the current task/feature manifest and approved plan digest;
8. reconcile any mismatch before mutation;
9. continue according to the actual stage (`SPECIFICATION`, `AWAITING_DEVELOPMENT_APPROVAL`, `APPROVED`, `VERIFYING`, or `BLOCKED`).

## Stop conditions

Stop affected mutation and reconcile when:

- active-package tracker resolution is ambiguous;
- the checkpoint conflicts with repository/CI evidence;
- branch/base/head ownership cannot be established;
- approved plan digest does not match;
- unknown work could be overwritten;
- the proposed change would alter a frozen denominator/dependency contract without approved change control.

This file does not replace package-specific lifecycle, evidence, change-control, trust-boundary, or release rules. It defines the adoption/resume layer that all of them inherit.
