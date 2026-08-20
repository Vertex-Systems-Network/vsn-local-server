# Batch 0.10.0 — native MongoDB, passkeys, stream protocol, shared PostgreSQL state

Date: 2026-08-18

## Implemented in this batch

### Generic bounded stream foundation

A new `vsn-stream` crate provides versioned multiplexed stream state for terminal, file upload/download, database, preview, logs and custom resources. It enforces:

- 128 active streams maximum per process baseline
- 256 KiB maximum frame payload
- 4 MiB aggregate buffered payload per stream
- sequence-number enforcement
- direction enforcement
- bounded pulls and backpressure
- explicit open/input/output/close/list/state operations

`vsn-core` binds a stream to the authenticated principal that opened it, and Agent IPC exposes the stream commands. These frames are not yet routed through the remote Control Plane/WebSocket gateway, so this is a transport foundation rather than a claim of complete remote streaming.

### Native MongoDB

`vsn-database-native` adds native MongoDB:

- inspect databases/collections
- browse with bounded limit/offset/filter
- structured insert/update/delete
- indexes
- collection statistics

Plain `mongodb://` is accepted only for loopback hosts in the current VSN baseline. `mongodb+srv://` is accepted for TLS-oriented remote discovery. Mongo mutations are local DB-write operations and are deliberately not added to the signed remote mutation allowlist.

### Verified PostgreSQL/MySQL TLS read profiles

New operator-controlled profiles add:

- PostgreSQL inspect/browse/read-query over TLS with an explicit PEM root CA
- MySQL inspect/browse/read-query with an explicit root CA
- bounded CA file validation
- conservative read-only SQL checks and read-only transaction attempts

These profiles are local-only until a remote certificate/profile/approval policy is designed.

### Real WebAuthn passkeys

The Control Plane now supports:

- authenticated passkey registration begin/finish
- public passkey login begin/finish
- short-lived server-side registration/authentication ceremony state
- persisted passkey credentials on accounts
- duplicate credential rejection
- credential-state/counter update after authentication
- passkey-authenticated sessions
- browser dashboard registration/login flows

Passkey ceremony state is intentionally memory-only and one-time. A Control Plane restart invalidates an in-progress ceremony, but registered passkeys remain in persistent account state.

### Shared PostgreSQL snapshot store

`vsn-control-store` adds an optional PostgreSQL snapshot backend:

- explicit verified TLS + operator-provided root CA
- SHA-256 payload verification
- per-snapshot PostgreSQL advisory transaction lock
- generation compare-and-swap
- optional explicit one-time local-state import

This solves shared durable snapshot storage and stale-writer protection. It does **not** yet distribute in-memory command queues, presence, rate windows, pending WebAuthn/OIDC transactions or sessions across Control Plane instances.

### VPS health rollback

Existing release upload/activate/rollback gains a deterministic health check:

- fixed localhost target
- validated port/path/status range
- bounded curl timeout
- optional rollback to the already-known previous release
- no arbitrary user deployment shell hook

## Security boundaries retained

- new MongoDB mutations are not remotely allowlisted;
- verified DB TLS paths require explicit local CA files;
- generic stream operations remain local authenticated IPC until remote routing policy exists;
- passkey challenge state is server-side and expires quickly;
- shared PostgreSQL storage uses generation CAS rather than last-write-wins;
- cloud health checks cannot target arbitrary internet hosts.

## Not completed in 0.10

- true browser/Control-Plane/Agent multiplexed stream routing and reconnect/resume
- OIDC authorization-code exchange, JWKS/token validation and account mapping
- SAML
- shared distributed queues/presence/sessions/rate limiting
- provider-native AWS/Azure/GCP/VPS create/destroy
- application-specific deploy/migrate/restart policies
- full remote DB mutation/TLS proxy policy
- production installer/signing/notarization
