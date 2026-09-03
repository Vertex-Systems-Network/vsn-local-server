# PKG-04..PKG-08 lifecycle review

Status: PREPARED / BLOCKED. No downstream package is activated by this document.

## Authority

Canonical package order and denominators come from live `main` `docs/MASTER-EXECUTION-PLAN.md`:

`PKG-03 -> PKG-04 -> PKG-05 -> PKG-06 -> PKG-07 -> PKG-08`.

The current canonical package remains PKG-03. The portfolio preplan may merge as planning-only metadata, but `.ai/state.json`, master progress and package completion counters must not move forward.

## Lifecycle per downstream package

Each PKG-04..PKG-08 package follows the same resumable lifecycle:

1. **Prepared preplan** — task IDs, denominator, high-level DAG, predecessor handoff and scope boundaries exist but are dormant.
2. **Activation preflight** — after predecessor COMPLETE, re-read fresh `main`, predecessor evidence/artifacts and current external requirements.
3. **Freeze PR** — reconcile/update research, lifecycle, security/data-flow/design/QA/performance artifacts as required; freeze exact task acceptance/evidence and hashes.
4. **Activation task** — first task projects the package into canonical state only after freeze governance is green/merged.
5. **Implementation DAG** — at most five ready tasks run concurrently; each task uses its own branch/PR/evidence and becomes DONE only after accepted merge.
6. **Final package gate** — exact-head full package regression/evidence proves all prior tasks.
7. **State-only completion projection** — a separate PR updates canonical package COMPLETE and selects the next prepared package.

## Cross-package gates

- PKG-04 activation requires PKG-03 COMPLETE and accepted installer artifact/layout/signing boundary.
- PKG-05 activation requires PKG-04 COMPLETE and accepted updater/recovery/update-manifest handoff.
- PKG-06 activation requires PKG-05 COMPLETE and accepted Windows/Linux/macOS release artifacts and platform matrix.
- PKG-07 activation requires PKG-06 COMPLETE and accepted security certification/remediation baseline.
- PKG-08 activation requires PKG-07 COMPLETE and accepted resilience/soak/fault-injection baseline.

No downstream child task may be counted DONE merely because preparatory code/tests happen to exist earlier.

## Concurrency policy

Maximum concurrency is five active implementation tasks inside the currently activated package. Prepared downstream packages consume planning lanes, not canonical implementation slots.

A task is runnable only when:

- its package is activated;
- every declared task dependency is DONE on canonical `main`;
- no open/superseding change invalidates its frozen plan;
- its branch starts from the required canonical predecessor state.

`active_task` remains a deterministic lowest-ID resume cursor. A package tracker may additionally carry `ready_tasks`/`active_tasks`, but canonical completion remains based on merged DONE evidence.

## Failure/resume protocol

Every interrupted or failed task records:

- package/task ID;
- source/head SHA;
- PR number/branch;
- workflow run/job and failing step when applicable;
- failure class: product defect, certification harness, runner/infrastructure, stale base or scope/change-control;
- exact next action.

On resume, do not trust chat memory. Re-read live main -> master state -> current package frozen tracker/manifest -> open task PRs -> exact-head CI/evidence -> Linear blockers. Stale/superseded branches close unmerged.

## Change control

Prepared downstream wording can be refreshed at activation because upstream packages may change concrete implementation details. Denominator/order/task IDs are treated as portfolio authority and require explicit master change-control to alter. Scope expansion is never self-approved by an implementation task.
