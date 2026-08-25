# PKG-02 02.25 Research — SQLite Database Studio

Feature ID: `pkg02-0225-sqlite-database-studio`  
Canonical base SHA: `d9b2cd0272c6f1e37119dfa7ea09fbd83dbf1842`  
Reviewed: `2026-08-25`

## Canonical scope

Frozen task:

`02.25 — SQLite Database Studio end-to-end: inspect, browse, safe query, indexes/relations/stats and structured insert/update/delete.`

Canonical machine state is PKG-02 `24/27 = 88.89%`, active `02.25`. `02.26+` remain blocked.

## Current repository audit

The current SQLite provider already exposes the required capability surface: inspect/introspection, browse, read query, indexes, relations, statistics, and structured insert/update/delete. The local authenticated principal already has `DatabaseView`, `DatabaseQuery`, and `DatabaseWrite`; high-risk `DatabaseDestructive` remains absent.

Two acceptance-blocking defects are present on canonical `main`:

1. **Database path containment is missing at the Core SQLite boundary.** `sqlite_inspect`, query, browse, indexes, relations, statistics, insert, update and delete accept a caller-supplied path and open it directly. Existing certified Core flows already use `vsn_files::resolve_existing(&workspace_roots, path)` for registered-workspace containment. SQLite should reuse that boundary rather than introduce a new path-security mechanism.
2. **Provider read bounds exceed the authenticated transport budget.** `MAX_READ_RESULT_BYTES` is 16 MiB and `MAX_TEXT_CELL_BYTES` is 8 MiB while authenticated IPC frames are 1 MiB. A provider result can therefore be valid locally but impossible to return through the actual Agent/CLI channel.

Additional current behavior to preserve unless acceptance proves a defect:

- read query validation accepts SELECT, EXPLAIN SELECT / EXPLAIN QUERY PLAN SELECT, and selected non-mutating PRAGMA forms; multiple statements and mutation statements fail closed;
- `WITH ... SELECT` is currently rejected and is not required by the frozen task;
- structured update/delete require non-empty equality filters;
- insert requires at least one value;
- SQLite BLOB cells are represented as bounded metadata rather than materialized into JSON;
- browse clamps row count and uses deterministic typed metadata.

## Stale preparation review

Open PR #59 was inspected only as historical preparation. It is stacked on an obsolete 02.24 preparation branch, was authored when canonical PKG-02 was 7/27, uses stale IPC port 49731, and provides no current exact-head lifecycle binding. It is not an implementation baseline or acceptance authority.

Useful audit leads from PR #59 were revalidated against current `main`: deterministic fixture coverage, direct outside-workspace rejection, frame-safe large-result handling, structured mutation filters, audit verification and cleanup.

## Primary-source delta research

Official SQLite documentation reviewed on 2026-08-25:

- https://www.sqlite.org/limits.html — SQLite explicitly supports lowering run-time limits for applications that process externally influenced databases/queries and recommends tighter limits as a resource-safety measure.
- https://www.sqlite.org/c3ref/c_limit_attached.html — run-time limit categories include string/BLOB/row length and SQL length.
- https://www.sqlite.org/pragma.html — `PRAGMA query_only` blocks ordinary data-changing statements but is not a complete read-only guarantee; it does not replace opening a database read-only or an application-level query policy.

Market/API delta: **none that changes the frozen VSN scope.** The correct 02.25 design remains an embedded SQLite provider behind authenticated Agent/Core boundaries, with explicit application-level containment, query grammar, permissions and transport/resource limits.

## Planning conclusions

- Reuse `vsn_files::resolve_existing` against registered workspace roots before every SQLite open.
- Keep read operations under `DatabaseView`/`DatabaseQuery`; structured mutations remain under `DatabaseWrite`; do not grant or consume `DatabaseDestructive`.
- Preserve the current safe-query grammar; do not widen it merely for convenience.
- Freeze a provider-level **512 KiB maximum serialized JSON read-result budget** and a **256 KiB maximum materialized text-cell budget**. Oversized text cells become explicit metadata/truncation objects; BLOBs remain metadata. This leaves deterministic margin below the 1 MiB authenticated IPC frame after Core/IPC response wrapping.
- Measure the actual final CLI/IPC payload in certification, not just provider-internal counters.
- Use a deterministic disposable SQLite fixture under a registered workspace and a byte-hashed outside-workspace fixture to prove containment/non-mutation.
- GitHub-hosted Windows/X64 exact-head evidence remains the acceptance authority.
