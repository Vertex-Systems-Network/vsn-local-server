# PKG-02 02.25 Frozen Plan — SQLite Database Studio End-to-End

Feature ID: `pkg02-0225-sqlite-database-studio`  
Version: `1.0.0`  
Canonical base SHA: `d9b2cd0272c6f1e37119dfa7ea09fbd83dbf1842`  
Approval reference: `docs/MASTER-EXECUTION-PLAN.md — frozen PKG-02 task 02.25`  
Approved date: `2026-08-25`

## Outcome

Genuinely certify the frozen task:

`02.25 — SQLite Database Studio end-to-end: inspect, browse, safe query, indexes/relations/stats and structured insert/update/delete.`

## In scope

- authenticated SQLite inspect/introspection for an existing database inside a registered workspace;
- deterministic browse with columns, row counts, limit/offset and ordering behavior;
- safe read-query execution through the existing SQLite query grammar;
- indexes, foreign-key relations and statistics;
- structured insert/update/delete using the existing `MutationRequest` model;
- non-empty equality-filter enforcement for update/delete;
- registered-workspace containment for every SQLite operation, including Windows junction/symlink escape rejection;
- frame-safe/bounded read results through the real Agent IPC channel;
- preservation of the existing DatabaseView/DatabaseQuery/DatabaseWrite policy split;
- exact-source GitHub-hosted Windows evidence, audit-chain verification and cleanup;
- only bug fixes directly required by these acceptance criteria.

## Explicit non-goals

- no PostgreSQL/MySQL/MariaDB/MongoDB/Redis implementation or certification (`02.26`);
- no database import/export/backup/restore acceptance;
- no arbitrary destructive SQL surface;
- no grant of `DatabaseDestructive` to the normal local principal;
- no widening of the current read-query grammar to CTEs or arbitrary PRAGMA statements unless a separately approved plan revision requires it;
- no Desktop redesign;
- no installer/updater work;
- no remote database production acceptance;
- no `02.26+` product work;
- no task denominator/order changes.

## Dependencies

- canonical `02.01`–`02.24` integrated DONE;
- canonical PKG-02 state `24/27 = 88.89%`, active `02.25`;
- canonical base `d9b2cd0272c6f1e37119dfa7ea09fbd83dbf1842`;
- Rust/cargo exact `1.97.1`;
- authenticated IPC on `127.0.0.1:39731`;
- release candidate `c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`, product `0.38.1`;
- existing certified workspace containment primitive `vsn_files::resolve_existing`.

## User-visible / operator behavior

- SQLite operations accept only an existing database path contained by a registered workspace.
- `inspect` returns deterministic entity/field metadata for the fixture.
- `browse` returns a bounded page with stable columns/row-count/limit/offset semantics.
- safe query accepts the existing SELECT / EXPLAIN SELECT / non-mutating PRAGMA contract and rejects mutation/multiple-statement/unsupported forms.
- indexes, relations and statistics return the fixture's deterministic index, foreign-key and row-count metadata.
- structured insert/update/delete operate through JSON mutation requests; update/delete without a non-empty equality filter fail.
- an outside-workspace or junction-escaped database path fails before SQLite is opened/mutated.
- oversized read data is bounded before authenticated IPC framing; the CLI receives a truthful bounded/truncated representation or a deliberate bounded-result error, never an oversized frame failure.

## Security / data constraints

- `DatabaseView`, `DatabaseQuery`, and `DatabaseWrite` remain the ordinary local permissions used by this task.
- `DatabaseDestructive` remains high-risk and absent from `Principal::local_authenticated()`.
- Before every SQLite provider open, Core resolves the database path through registered workspace roots using the already-certified existing-path resolver.
- Direct outside-workspace paths, canonical aliases escaping the workspace, and Windows junction/symlink escapes fail closed.
- Structured update/delete require non-empty equality filters at the provider boundary.
- Read-query validation remains application-level allowlist logic in addition to opening read operations read-only.
- Provider read results use a maximum **512 KiB serialized JSON result budget**.
- Materialized text cells use a maximum **256 KiB cell budget**; larger cells return explicit metadata with `type`, `bytes`, and `truncated=true`. BLOB cells remain metadata-only.
- Bounds are checked on serialized JSON bytes so escaping overhead is included.
- The certification must measure the final CLI response size and prove it stays below the 1 MiB IPC frame contract.
- Normal certification uses disposable fixture databases only and preserves an outside-workspace sentinel database byte-for-byte.

## Acceptance criteria

- `AC-01 Exact source/toolchain`: GitHub-hosted Windows/X64 verifies checkout source equals `EXPECTED_SHA`; rustc/cargo are exactly 1.97.1; evidence binds canonical base, feature/plan IDs and frozen plan digest.
- `AC-02 Inspect`: a deterministic registered-workspace SQLite fixture containing `teams` and `users` is inspected through authenticated Agent/CLI; expected fields/types and both entities are present; missing/invalid database fails.
- `AC-03 Browse`: authenticated browse of `users` returns stable columns, correct total rows, requested bounded limit/offset, deterministic ordering when requested, and no response exceeds the frozen read-result budget.
- `AC-04 Safe query`: SELECT and supported read-only EXPLAIN/non-mutating PRAGMA forms succeed; DELETE/UPDATE/INSERT/DDL, multiple statements, unsupported CTE form and mutating/risky PRAGMA forms fail through the read-query boundary.
- `AC-05 Metadata`: indexes include deterministic `idx_users_name`; relations include `users.team_id -> teams.id`; statistics report the fixture row count and truthful storage metadata.
- `AC-06 Structured insert`: authenticated `DatabaseWrite` insert creates exactly one intended row; empty values/unsafe identifiers or invalid fields fail without unintended mutation.
- `AC-07 Structured update`: a non-empty equality filter updates exactly the intended row; empty filter fails; unrelated rows remain unchanged.
- `AC-08 Structured delete`: a non-empty equality filter deletes exactly the intended row; empty filter fails; unrelated rows remain unchanged.
- `AC-09 Workspace containment`: inspect/query/browse/indexes/relations/stats/insert/update/delete all reject a direct outside-workspace database and a Windows junction/symlink escape before provider open. Outside sentinel database pre/post SHA-256 is identical.
- `AC-10 Frame/resource safety`: provider query/browse enforce 512 KiB serialized JSON result ceiling and 256 KiB materialized text-cell ceiling. Oversized cells/results produce truthful truncation metadata or bounded rejection; final authenticated CLI response is measured below 1 MiB and does not fail at IPC framing.
- `AC-11 Permission/audit/cleanup`: `local_authenticated` retains DatabaseView/DatabaseQuery/DatabaseWrite and lacks DatabaseDestructive; operations produce a valid nonzero audit chain; Agent stops; IPC key and LOCALAPPDATA are restored; sandbox/junction/fixtures are removed.
- `AC-12 Evidence integrity`: evidence binds exact source/base/plan, candidate/product, runner/toolchain, Agent/CLI hashes, AC-01..AC-12, measured payload sizes, outside sentinel hashes, audit validity, cleanup and artifact/evidence SHA-256 values that are independently recomputable.

## Required implementation / certification files

Primary planned product files:

- `crates/vsn-core/src/lib.rs`
- `crates/vsn-database-sqlite/src/lib.rs`

Focused tests may be added under:

- `crates/vsn-core/tests/`
- `crates/vsn-database-sqlite/tests/` (create only if needed for provider-bound tests)

Certification files:

- `crates/vsn-database-sqlite/examples/pkg02_fixture.rs`
- `scripts/self-hosted/pkg02-0225.ps1`
- `.github/workflows/pkg02-0225-sqlite-database-studio.yml`

Conditional files only when a mapped AC requires them:

- `apps/agent/src/main.rs`
- `apps/cli/src/main.rs`
- relevant Cargo manifests / `Cargo.lock` if dependency structure genuinely changes.

No other product file may change without mapping to an AC or approved plan addendum.

## Required commands

- `cargo fmt --all -- --check`
- `cargo clippy --locked --package vsn-database-sqlite --package vsn-database --package vsn-core --package vsn-policy --package vsn-agent --all-targets --no-deps -- -D warnings`
- `cargo test --locked --package vsn-database-sqlite --package vsn-database --package vsn-core --package vsn-policy`
- `cargo build --locked --release --package vsn-agent --package vsn --package vsn-database-sqlite --example pkg02_fixture`
- `pwsh -NoProfile -File scripts/self-hosted/pkg02-0225.ps1`
- `git diff --check`

## Required regression gates on final exact head

- AI Planning Governance
- Repository Governance
- PKG-02 Acceptance Sequence
- PKG-02 02.02 Authenticated IPC
- PKG-02 02.08 Windows GitHub-Hosted Certification
- PKG-02 02.14 Local Diagnostics
- PKG-02 02.16 Workspace Text Files
- PKG-02 02.17 Resumable Binary Workspace Transfer
- PKG-02 02.18 Bounded Direct Terminal Execution
- PKG-02 02.19 Persistent Pipe Terminal Sessions
- PKG-02 02.20 PTY ConPTY Lifecycle
- PKG-02 02.21 Loopback Preview Fetch
- PKG-02 02.22 Advanced Preview Requests
- PKG-02 02.23 `.test` DNS Responder
- PKG-02 02.24 Local Domain/HTTPS Boundary
- PKG-02 02.25 SQLite Database Studio

## Evidence artifact

`pkg02-0225-sqlite-database-studio-github-hosted`

Expected contents include:

- `evidence.json` plus independently recomputable SHA-256;
- exact source/base/feature/plan binding;
- deterministic fixture definition and database hashes;
- inspect/browse/query/index/relation/statistics outputs;
- structured insert/update/delete and empty-filter negative outputs;
- direct outside and junction-escape rejection evidence plus outside sentinel pre/post SHA-256;
- large-cell/result and final CLI payload measurements;
- permission proof;
- audit output;
- cleanup JSON with every required field;
- Agent/CLI binary hashes.

## Rollout / rollback

Rollout is merge of a genuinely accepted 02.25 PR, followed by machine-state projection from `24/27`, active 02.25 to `25/27 = 92.59%`, active 02.26. Until final exact-head evidence and authorized merge, canonical state remains 24/27 active 02.25.

Rollback is PR closure/revert. Normal certification uses disposable databases and must leave outside-workspace state unchanged.

## Change control

This plan is frozen by its SHA-256 in the feature manifest. Do not edit it in place after the manifest records its digest. Material scope, permission, acceptance, resource-budget, or rollout changes require an approved addendum or new plan version.
