# PKG-03 Windows Installer Execution Workflow

Canonical package plan: `.ai/plans/pkg03-windows-installer-v1.md`
Tracker: `certification/pkg03-windows-installer-v1.json`
Fixed denominator: 25 tasks.

## Source of truth

1. live canonical GitHub `main`;
2. machine-readable master status + PKG-03 tracker;
3. frozen package manifest/plan digests;
4. exact-head GitHub PR/run/artifact evidence;
5. Linear mirrors execution state and never overrides GitHub.

## Resume algorithm

- Read live `main`.
- Read master status + tracker.
- Verify plan/manifest digests.
- Read open PKG-03 PRs and Linear parent/children.
- For each `IN_PROGRESS` task, locate its authoritative PR and exact-head run state.
- If none are active, compute `ready_tasks` from dependency closure and start up to five non-conflicting lanes.
- Never start a BLOCKED task.
- Merge only accepted exact-head task PRs with expected-head protection.
- After every merge, recompute DAG readiness from canonical main and update Linear.

## Failure algorithm

Record failure at both PR and Linear task with exact run/job, failing step, source SHA and classification. Fix only the minimum approved scope. A changed PR head invalidates old exact-head acceptance and reruns required gates. Stale/duplicate branches are superseded and closed rather than merged.

## Parallel limit

Maximum active task PRs: 5.
`active_task` remains the lowest-ID active/ready deterministic resume cursor; `active_tasks` and `ready_tasks` in the tracker carry the full DAG state.
