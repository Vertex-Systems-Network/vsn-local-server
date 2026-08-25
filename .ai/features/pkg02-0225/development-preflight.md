# PKG-02 02.25 Development Preflight

Feature ID: `pkg02-0225-sqlite-database-studio`  
Canonical base SHA at planning start: `d9b2cd0272c6f1e37119dfa7ea09fbd83dbf1842`  
Status: `READY FOR IMPLEMENTATION ONLY AFTER PLANNING-HEAD GATES PASS`

## Canonical reconciliation

- live `main` was read after PR #97 narrative reconciliation;
- machine state: PKG-02 24/27 = 88.89%;
- active task: 02.25;
- 02.24: DONE;
- 02.26 and 02.27: BLOCKED;
- master execution narrative now matches machine-readable state;
- open PR #59 inspected and classified as stale preparation only.

Any change to live `main`, active task, or task status before product mutation requires a fresh re-read. A mismatch means STOP and reconcile.

## Frozen scope

Only 02.25 may be implemented. The exact outcome is inspect, browse, safe query, indexes/relations/stats and structured insert/update/delete for SQLite through the real authenticated local path.

No 02.26 adapter implementation, no Desktop redesign, no permission widening, no installer/updater work.

## Confirmed acceptance blockers on canonical base

1. Core SQLite endpoints open arbitrary caller paths without registered-workspace containment.
2. SQLite provider permits 16 MiB read results / 8 MiB text cells, incompatible with the 1 MiB authenticated IPC frame.

## Approved implementation direction

- add one shared Core SQLite-path resolver that loads registered roots and calls `vsn_files::resolve_existing`;
- invoke it before every SQLite inspect/query/browse/index/relation/statistics/insert/update/delete provider open;
- lower provider read-result serialized JSON ceiling to 512 KiB;
- lower materialized text-cell ceiling to 256 KiB and return explicit truncation metadata above it;
- preserve BLOB metadata-only behavior;
- preserve current query grammar and structured mutation filter rules;
- preserve local DatabaseView/DatabaseQuery/DatabaseWrite and absence of DatabaseDestructive;
- add focused regressions for containment and result bounds;
- build a fresh current-head GitHub-hosted Windows certification harness/workflow; do not reuse stale port/base assumptions from PR #59.

## Mutation gate

Before the first product-code write:

1. confirm live `main` still equals the expected canonical base or explicitly reconcile a newer canonical base;
2. confirm machine state still says active 02.25;
3. confirm this frozen plan digest matches the manifest;
4. confirm AI Planning Governance, Repository Governance and PKG-02 Acceptance Sequence pass on the planning head;
5. confirm no competing accepted 02.25 PR supersedes this branch.

Only then may product mutation begin.

## Test gate before acceptance claim

Run every command listed in the frozen plan and the full exact-head regression set. Evidence must be GitHub-hosted Windows/X64 and exact-source bound.

Do not update 24/27 -> 25/27 until genuine AC-01..AC-12 evidence is independently verified.
