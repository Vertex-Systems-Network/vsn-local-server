# PKG-02 02.25 Lifecycle Review — Architecture, Data Flow, Security, Design, QA and Performance

Feature ID: `pkg02-0225-sqlite-database-studio`  
Canonical base SHA: `d9b2cd0272c6f1e37119dfa7ea09fbd83dbf1842`

## Architecture

The existing architecture is retained:

`vsn CLI -> authenticated local IPC -> vsn-agent -> vsn-core -> vsn-database-sqlite -> rusqlite -> workspace-contained .db file`

No new database service, privileged helper, remote control path or provider abstraction is required for 02.25.

Core is the correct location for workspace authorization because it already owns the authenticated principal/config boundary and already applies `vsn_files::resolve_existing` to accepted project/container paths. The SQLite provider remains responsible for SQL/provider-local validation and deterministic result bounding.

## Data flow

Read flow:
1. CLI sends SQLite operation and path through authenticated IPC.
2. Agent constructs the normal local authenticated principal.
3. Core checks DatabaseView/DatabaseQuery as appropriate.
4. Core loads registered workspace roots and resolves the existing database path with `vsn_files::resolve_existing`.
5. Provider opens only the resolved contained path in read-only mode.
6. Provider validates query/browse request, creates bounded JSON/value structures, and enforces the frozen 512 KiB serialized read-result budget.
7. Core/Agent serializes the bounded response through the 1 MiB IPC frame.
8. CLI renders JSON.

Mutation flow:
1–4 are the same, with DatabaseWrite.
5. Provider opens the contained database read-write.
6. Insert/update/delete validate identifiers/maps; update/delete require non-empty equality filters.
7. Mutation result is returned and audited.

No provider open occurs before path containment succeeds.

## Security review

Threats and controls:

- arbitrary outside database read/write -> Core existing-path workspace resolver before every SQLite provider open;
- junction/symlink escape -> canonical containment behavior inherited from already-certified `vsn_files::resolve_existing`;
- destructive SQL through query endpoint -> existing read-query allowlist plus read-only provider open;
- mass update/delete -> non-empty equality filter required;
- permission escalation -> retain DatabaseView/DatabaseQuery/DatabaseWrite; DatabaseDestructive stays absent/high-risk;
- oversized row/text causing IPC overflow or memory pressure -> 256 KiB text materialization + 512 KiB serialized result budget, measured after JSON serialization;
- binary amplification -> BLOBs stay metadata-only;
- stale prep authority -> PR #59 is research only; fresh branch is bound to canonical base;
- outside fixture mutation during negative tests -> pre/post SHA-256 equality required.

`PRAGMA query_only` is not used as the sole trust boundary; official SQLite documentation notes it is not equivalent to a fully read-only database connection.

## Design / operator contract

Keep the existing `vsn db sqlite-*` CLI surface. 02.25 is an end-to-end usability/correctness certification, not a UI redesign.

Deterministic fixture:
- table `teams(id, name UNIQUE)`;
- table `users(id, team_id FK, name, email UNIQUE, note)`;
- index `idx_users_name`;
- stable seed rows including Alice;
- one oversized text row above 256 KiB for truncation/frame testing;
- a separate outside-workspace sentinel database.

The acceptance harness must use current IPC port `39731`; stale 49731 assumptions from PR #59 are invalid.

## QA mapping

- AC-01: exact source/toolchain/plan digest precheck.
- AC-02: inspect + missing/invalid negative.
- AC-03: browse page/order/bounds.
- AC-04: query allow/deny matrix.
- AC-05: index/FK/statistics.
- AC-06..08: structured mutation and filter safety.
- AC-09: direct outside + junction escape for reads and writes, outside hash preservation.
- AC-10: oversized text/result + actual CLI serialized payload.
- AC-11: policy assertions, audit, cleanup.
- AC-12: artifact/hash binding.

No green workflow result counts unless the evidence source commit equals the PR head under evaluation.

## Performance / resource review

Frozen budgets:
- authenticated IPC frame: existing 1 MiB contract;
- provider serialized read result: <= 512 KiB;
- materialized text cell: <= 256 KiB;
- browse row limit: preserve existing max 1000 unless acceptance shows a bug;
- read query statement length: preserve existing 1 MiB validation ceiling, while final response remains much smaller;
- SQLite busy timeout: preserve existing bounded 5 seconds;
- certification workflow: GitHub-hosted Windows, bounded by workflow timeout.

Why serialized-byte accounting matters: JSON escaping changes byte size; raw character counts are not a sufficient frame-safety proof.

## Failure behavior

- unresolved/outside path -> nonzero error, no provider open;
- invalid query -> nonzero error, no mutation;
- empty update/delete filter -> nonzero error, no mutation;
- result over total budget -> deterministic bounded error or truncation contract, never IPC frame overflow;
- oversized text cell -> metadata/truncated representation;
- missing fixture -> nonzero error;
- Agent unavailable -> existing CLI failure semantics;
- any cleanup failure -> acceptance failure.

## Rollout review

Only after AC-01..AC-12 plus the frozen final-head regression set pass may branch state be projected to 25/27. No 02.26 implementation is permitted before 02.25 merge and fresh canonical re-read.
