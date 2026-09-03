# PKG-04..PKG-08 Parallel Planning Resume Protocol

This document exists so work can resume from repository state after a chat/session interruption.

## What is active now

Only the package selected by canonical `.ai/state.json` is an implementation package. At creation of this preplan, that package is PKG-03. PKG-04..PKG-08 are `PREPARED_BLOCKED` planning lanes.

## Resume algorithm

1. Fetch live `main`; never reuse a remembered SHA.
2. Read `.ai/state.json`, `docs/MASTER-EXECUTION-STATUS.json` and `docs/MASTER-EXECUTION-PLAN.md`.
3. If the active package is PKG-03, continue its frozen tracker/plan. Treat this downstream preplan as reference only.
4. If a downstream predecessor is COMPLETE, open this portfolio manifest and that package's latest freeze/reconciliation PR.
5. Re-run fresh delta research for the package before activation.
6. Verify package task count/order against `.ai/manifests/pkg04-pkg08-parallel-preplanning.v1.json` using `python scripts/ci/validate-pkg04-pkg08-preplans.py`.
7. Reconcile Linear package parent and blockers with the GitHub task list.
8. Once package freeze governance is green and merged, allow only `NN.01` to activate it.
9. After activation, compute ready tasks as tasks whose `depends_on` entries are all canonical DONE. Run no more than five concurrently.
10. For every task, inspect its exact-head PR workflows/evidence before deciding DONE/fix/rerun.
11. A product defect gets the smallest acceptance-mapped fix. Infrastructure/harness failure does not become a product change.
12. If a task changes the head SHA, required exact-head gates restart as defined by that package's frozen acceptance contract.
13. Final task must prove the complete package matrix. Completion is projected in a separate state-only PR.

## Failure record template

```text
Package/task:
Canonical base SHA:
Task head SHA:
Branch / PR:
Run / job:
Failing step:
Class: product | harness | runner/infrastructure | stale-base | change-control
Observed evidence:
Minimal next action:
Does head change? yes/no
Regressions that must restart:
```

## Package activation chain

- PKG-04 waits for PKG-03 COMPLETE.
- PKG-05 waits for PKG-04 COMPLETE.
- PKG-06 waits for PKG-05 COMPLETE.
- PKG-07 waits for PKG-06 COMPLETE.
- PKG-08 waits for PKG-07 COMPLETE.

This chain is an activation boundary, not a ban on planning. Research, plan review, Linear setup and non-mutating architecture preparation may proceed concurrently.

## Linear policy

Each package has one portfolio parent while dormant. The parent remains Backlog/blocked by its predecessor package. Full per-task child materialization happens during that package's activation reconciliation so descriptions and acceptance evidence are bound to the then-current canonical product. This avoids creating 108 stale child issues months before their product surfaces stabilize.

When children are materialized, task IDs/titles must match the frozen GitHub package plan, and `blockedBy` relations must mirror the machine-readable DAG rather than merely describing dependencies in prose.

## Non-negotiable boundaries

- Never move `.ai/state.json` to a downstream package early.
- Never count preparatory work as DONE acceptance.
- Never move historical accepted branches to fake same-head evidence.
- Never store signing/notarization/update private keys in repository/Linear comments/evidence.
- Never attack third-party/production infrastructure under PKG-08; pentest targets must be owned/explicitly authorized.
- Never self-approve denominator/order/scope expansion.
