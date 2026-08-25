# PKG-02 02.26 Development Preflight

Feature ID: `pkg02-0226-external-native-database-adapters`
Canonical base SHA at planning start: `836feb4171a9eb882208a6d666600cea4abe3f42`
Status: `READY FOR IMPLEMENTATION ONLY AFTER PLANNING-HEAD GATES PASS`

## Canonical reconciliation

- live `main` was read after PR #99 state projection;
- live canonical HEAD at planning start: `836feb4171a9eb882208a6d666600cea4abe3f42`;
- machine state: PKG-02 `25/27 = 92.59%`;
- active task: `02.26`;
- `02.01` through `02.25`: DONE;
- `02.27`: BLOCKED;
- product version: `0.38.1`;
- release candidate: `c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`;
- stale PR #60 is research-only preparation;
- future PR #61 remains blocked.

Any change to live `main`, active task, candidate/product, or competing accepted 02.26 work before product mutation requires a fresh re-read. A contradiction means STOP and reconcile.

## Frozen scope

Only 02.26 may be implemented:

`External/native database beta adapters: client detection plus PostgreSQL/MySQL/MariaDB/MongoDB/Redis declared-capability handling, with loopback/TLS and unsupported-capability fail-closed rules.`

No 02.27 implementation, installer/updater work, Remote Control Plane expansion, permission widening, database server installation, roadmap/denominator/version change.

## Confirmed acceptance blockers on canonical base

1. PostgreSQL plaintext native loopback validation is substring-spoofable.
2. MySQL plaintext native loopback validation is substring-spoofable.
3. External CLI connection specs do not express verified TLS policy.
4. External client detection and synchronous execution use unbounded `Command::output()`.
5. Native provider read results have row limits but no IPC-safe cell/result byte budget.
6. MongoDB SRV accepts explicit insecure/TLS-disable options without policy rejection.
7. Redis remote TLS does not explicitly reject insecure modifiers.
8. Native TLS CA paths are not constrained by the same Core containment rule used for external credential files.
9. Agent/Core expose PostgreSQL/MySQL verified-TLS read commands, but the public CLI lacks a corresponding local operator path.

## Approved implementation direction

- structural endpoint/loopback parsing; no substring host decisions;
- explicit transport/TLS policy in the external CLI adapter;
- strict engine-specific remote TLS modes + trusted CA use;
- reject insecure Mongo/Redis options;
- reuse Core workspace-or-VSN-data containment for credential/CA files;
- bounded client detection/synchronous execution with concurrent pipe drains;
- native 256 KiB materialized-text and 512 KiB serialized-result limits;
- truthful deterministic five-engine capability metadata;
- minimal public CLI commands for existing PostgreSQL/MySQL TLS read paths, using stdin JSON where needed;
- preserve DatabaseView/DatabaseQuery/DatabaseWrite and absence of DatabaseDestructive;
- preserve Mongo/Redis arbitrary-query rejection;
- add no remote database command permissions;
- create a fresh exact-head GitHub-hosted Windows certification harness/workflow.

## Mutation gate

Before the first product-code write:
1. confirm live `main` still equals `836feb4171a9eb882208a6d666600cea4abe3f42` or explicitly reconcile a newer canonical base;
2. confirm machine state still says active `02.26`;
3. recompute and confirm frozen plan SHA-256 equals the manifest;
4. confirm AI Planning Governance, Repository Governance and PKG-02 Acceptance Sequence pass on the planning head;
5. confirm no competing accepted 02.26 PR supersedes this branch.

Only then may product mutation begin.

## Test gate before acceptance claim

Run every command listed in the frozen plan and the entire frozen exact-head regression set. Evidence must be GitHub-hosted Windows/X64 and exact-source bound.

Do not update `25/27 -> 26/27` until genuine AC-01..AC-12 evidence is independently verified and the accepted product PR is merged.
