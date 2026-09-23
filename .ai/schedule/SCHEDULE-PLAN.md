# Scheduled AI Development Plan

## Purpose
This file governs scheduled ChatGPT development runs only. Interactive/chat development continues to use the repository's existing AI, agent, governance, roadmap, issue and development-plan contracts. This schedule plan supplements those contracts; it does not replace or weaken them.

## Source of truth and recovery
At the start of every scheduled run:
1. Reconcile the exact default-branch HEAD and repository instructions.
2. Read the repository's existing AI/development plan, roadmap, AGENTS/AUTO-AGENT instructions, durable state, accepted open Issues and open PRs before choosing new work.
3. Treat GitHub repository/runtime evidence as authoritative over stale schedule state or a spreadsheet.
4. Continue the existing development plan from the last safe durable checkpoint. Do not create a parallel product roadmap.

## Google Drive progress ledger
Each repository must use its own Google Sheet named `<repo> — Scheduled AI Development Report`.
- On the first scheduled run, use the connected Google Drive app to locate the exact sheet name; create it if absent.
- Persist the Sheet URL/ID in scheduled-run context when available and reuse the same Sheet on later runs.
- Append, never overwrite, one report row per run with: timestamp, repository, default-branch SHA, active milestone, Issue, PR, PR head SHA, CI/runner state, security state, action taken, result/blocker, and next safe action.
- The Sheet is an audit/report ledger, never development authority.
- If Google Drive is temporarily unavailable, do not fabricate a write. Preserve the GitHub checkpoint, record the reporting blocker when possible, and resume ledger updates on a later run.

## Continuous scheduled development
Scheduled development must keep advancing the repository's accepted development plan while actionable authorized work remains.
- Continue accepted open Issue/PR work before claiming unrelated new work.
- Review, implement, test and merge only within repository authorization and security policy.
- Pending CI/runners are handoff boundaries, not project completion.
- Never busy-wait or repeatedly poll unchanged runner state within one run.
- Failed checks must be inspected and safely fixed; do not bypass or weaken gates.
- Never fabricate runtime, external, security or release evidence.
- Do not use status-only/no-op commits to simulate progress.

## Single-writer lease and handoff
Scheduled runs use a durable single-writer lease/checkpoint model to prevent overlapping mutations.
- A run records its run identity, start time, exact base/head, active Issue/PR and next safe action in the repository's existing durable state when that state supports schedule metadata; otherwise the run report acts only as a non-authoritative hint and live GitHub state must be reconciled.
- Before mutating, a new scheduled run must revalidate live GitHub state and detect whether prior scheduled work is still active.
- If a predecessor is safely supersedable/stale, the new run resumes from the last durable checkpoint after revalidation.
- If a predecessor may still be performing a mutation and cannot be safely terminated by the platform, do not create a competing writer. Fail closed, checkpoint the handoff, and resume on the next run.
- Never assume that starting a new schedule invocation can forcibly kill another process. Safe supersession is implemented through lease/checkpoint/handoff semantics.

## Pull requests and merge safety
Use the repository's existing PR workflow. Before merge, verify the exact PR head, required checks, review/thread requirements, mergeability and main divergence. Merge only when the repository policy and granted authority permit it. Reconcile resulting default-branch HEAD after merge.

## Completion
A run may end because its bounded iteration is complete, a runner/external dependency is pending, or a safe handoff is required. The scheduled development program itself remains active until the repository's documented roadmap/completion criteria are genuinely satisfied or the owner explicitly cancels it.
