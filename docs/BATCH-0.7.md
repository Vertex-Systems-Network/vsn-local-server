# Batch 0.7.0 — DB CRUD, persistent terminal sessions, transactional control state

## Implemented

- Universal DB SDK extended with browse pages, mutations, indexes, relations, statistics and UI actions.
- SQLite provider moved behind a mutex-backed connection boundary and gained paginated browse, parameterized insert/update/delete, indexes, relations and row statistics.
- SQLite read-query gate is conservative: single SELECT / EXPLAIN SELECT / non-mutating PRAGMA only; broad WITH support was removed.
- Persistent terminal process sessions: start, stdin write, bounded stdout/stderr drain, status, stop, list and remove. This is pipe-based, not a PTY.
- Desktop DB Studio now exposes SQLite inspect/browse/index/relation/stats and structured insert/update/delete controls; Files UI exposes workspace-contained mkdir/move/delete.
- Workspace file mkdir/move/delete while preventing mutation of the configured workspace root itself.
- Existing VPS SSH preflight with key-only BatchMode, strict host-key verification, explicit known_hosts and bounded connection timeout.
- Control Plane state may now use a transactional SQLite/WAL snapshot store (`state.db`) with payload SHA-256 verification and monotonic generations. A verified one-time migration path imports the legacy 0.6 `state.json` when the new store is empty, then archives the legacy source.
- Enterprise auth policy baseline: MFA/step-up/passkey requirements plus validated HTTPS OIDC providers with mandatory PKCE.
- Control Plane exposes scoped auth-policy read/manage endpoints, and the browser dashboard can load/edit/validate the policy JSON.

## Still partial / deliberately not claimed

- PostgreSQL/MySQL/Mongo/Redis are not full native in-process CRUD drivers yet.
- Terminal sessions are not PTY-backed and do not provide resize, job control or full-screen TUI fidelity.
- The secure gateway is still polling-oriented rather than a regional persistent WebSocket/QUIC relay fabric.
- SSH preflight does not create cloud instances or bootstrap an Agent automatically.
- Enterprise auth does not yet implement account login, WebAuthn registration/assertion, TOTP enrollment, recovery flows or OIDC callbacks.
- Transactional SQLite persistence strengthens single-node durability; it is not multi-instance distributed consistency.
