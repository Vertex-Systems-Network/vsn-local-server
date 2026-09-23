# Scheduled AI Development Plan

## Purpose and precedence
This file governs scheduled AI development for this repository. It supplements, and never replaces or weakens, the repository's existing AI-Native plan, AGENTS/AUTO-AGENT instructions, governance, roadmap, durable state, security, Issue and PR/MR contracts.

Every scheduled run must continue the repository's existing AI-Native development flow from the last safe durable checkpoint. Do not invent a parallel product roadmap. Live repository, CI/runtime and security evidence outrank stale schedule state or spreadsheet reporting.

## Owner standing consent and execution authority
The repository owner grants standing consent for routine repository-maintenance and development mutations that are reasonably required to advance the existing AI-Native plan/flow. Do not request repeated owner consent for an action already covered by this standing authority.

Within repository policy and available platform capabilities, scheduled AI is authorized to:
- create, edit, update, synchronize/reconcile, review, resolve, merge, close, reopen, supersede, archive or delete PRs/MRs when the platform supports the requested lifecycle action;
- create, edit, update, resolve, close, reopen, supersede, archive or delete Issues when the platform supports the requested lifecycle action;
- update branches, repository files, tests, documentation and durable AI state;
- fix CI, test, security, governance and review failures;
- resolve review threads and apply required review feedback;
- remove obsolete/superseded branches, artifacts or schedule state when evidence proves removal is appropriate and required audit/history is preserved.

If the platform does not support literal deletion of a PR/MR or Issue, use the closest supported lifecycle action such as close, supersede or archive and record the result.

Standing consent authorizes execution of the AI-Native plan. It does not authorize bypassing required exact-head checks, branch protections, review requirements, security controls, migration/data-safety requirements, external-evidence requirements or repository-specific acceptance gates. Missing evidence never becomes PASS merely because owner consent exists.

## Run bootstrap and reconciliation
Before mutation:
1. Reconcile exact default-branch HEAD and repository instructions.
2. Read the current AI-Native plan/roadmap, relevant AGENTS/AUTO-AGENT instructions, durable state/checkpoints, accepted open Issues and PRs/MRs, reviews/threads, CI/runners, security constraints and active milestone.
3. Revalidate the exact PR/MR head and main divergence for work that may merge.
4. Revalidate any active lease/single-writer state before touching a shared mutation surface.
5. Continue accepted existing work before inventing unrelated work unless the repository's own roadmap says otherwise.

## Continuous forward progress — no idle/pause behavior
Scheduled AI must keep advancing authorized actionable work during the run. A pending CI/runner, open review, external dependency, blocked PR/MR, unresolved Issue or one failed action is not completion and must not make the scheduled program idle.

When one surface is blocked:
- preserve its durable checkpoint and exact next safe action;
- immediately continue other non-conflicting actionable work in this repository;
- when appropriate, continue other repository lanes in the same multi-repository run;
- revisit the blocked surface when fresh evidence or a safe mutation makes progress possible.

Do not repeatedly poll unchanged CI/runner state. Use available execution time for implementation, fixes, tests, security work, review resolution, branch reconciliation, PR/MR/Issue lifecycle actions, durable-state reconciliation or preparation of the next evidence-backed mutation.

Do not use no-op/status-only commits as progress. Do not fabricate runtime, security, release or external evidence.

## Single-writer and concurrency safety
Use the repository's durable single-writer/lease/checkpoint semantics. Before each mutation, revalidate live state and ownership of the mutation surface.

If prior scheduled state is stale and safely supersedable, reconcile it against current repository truth and continue from the last safe checkpoint.

If another writer is demonstrably active on the same mutation surface, do not create a conflicting write. Continue non-conflicting work immediately, preserve the exact conflict/checkpoint state, and return to that surface as soon as a safe mutation is possible. A conflict on one surface must not make the repository or multi-repository program idle.

## PR/MR, Issue and merge execution
Actively work the accepted repository queue rather than only reporting it.

For authorized PRs/MRs and Issues:
- inspect actual code/diff/state;
- fix defects and CI/security failures;
- update code, tests, docs and durable state;
- reconcile stale branches with current main when required by repository policy;
- resolve review feedback/threads;
- close or supersede obsolete work when evidence supports it;
- merge completed work when all repository gates are satisfied.

Before merge, verify exact current head, required checks, review/thread requirements, mergeability, current-main divergence, relevant security state, migration/data-safety impact and durable-state consistency. Use expected-head protection when supported. Reconcile resulting default-branch HEAD after merge.

Materially destructive cleanup must be evidence-backed, scoped and preserve required audit/history/evidence. Never weaken or hide a failing gate merely to obtain a merge.

## Security
Security is continuously active. Investigate and fix failed CodeQL, supply-chain, secret, provenance, auth/permission, governance, migration/data-safety and runtime acceptance checks. Preserve fail-closed behavior where authority or evidence is genuinely uncertain.

## Google Drive development ledger
Use the `Dev Schedules` Google Drive folder as the reporting home:
https://drive.google.com/drive/folders/1kmgjOWn6Yo7uThliEKOsZXWzL_kyTzlF

Reuse exactly one repository ledger named `vsn-local-server — Scheduled AI Development Report`, except where an existing repository-specific historical ledger is explicitly designated as canonical. Append history; never overwrite prior report rows.

For every scheduled run append: timestamp, repository, default-branch SHA, active milestone, Issue, PR/MR, PR/MR head SHA, CI/runner state, security state, concrete action taken, result/blocker and exact next safe action.

The ledger is an audit/reporting surface only and never outranks live GitHub/repository evidence. If Drive is unavailable, preserve repository checkpoint truth and backfill reporting when access returns without fabrication.

## Completion standard
A scheduled run must advance every safe actionable surface available within the invocation. The scheduled development program remains active until this repository's own documented completion criteria are genuinely satisfied or the owner cancels it.
