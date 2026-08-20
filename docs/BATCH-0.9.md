# Batch 0.9.0 — structured native DB writes, remote wait-read, release rollback, recovery auth

## Implemented in this batch

- Native PostgreSQL structured insert/update/delete, indexes, foreign-key relations and table statistics. Mutation values are typed through PostgreSQL record population rather than interpolated into SQL text.
- Native MySQL structured insert/update/delete with positional parameters, plus indexes, relations and table statistics.
- Native Redis string set with bounded optional TTL and key deletion; non-loopback plaintext Redis remains rejected.
- Database mutation commands are local Agent/CLI/Desktop surfaces only. The remote signed-command allowlist retains read/query boundaries and does not expose native structured DB writes.
- PTY and process-pipe sessions now have bounded wait-read operations (maximum five seconds) to reduce tight polling without pretending to provide unsolicited terminal push streaming.
- Resumable binary uploads expose the Agent-confirmed committed offset, and workspace files can be SHA-256 digested for independent transfer verification.
- Advanced local preview request surface supports common HTTP methods, bounded request/response bodies and filtered headers against `127.0.0.1` only. The mutation-capable surface is not remotely allowlisted.
- Existing VPS release lifecycle now supports checksum-verified SCP upload, versioned release directories, atomic CURRENT/PREVIOUS pointers and deterministic rollback. No arbitrary user deployment command is accepted.
- Account MFA adds single-use recovery codes; only Argon2 PHC hashes are persisted and regeneration revokes existing sessions.
- OIDC adds state/nonce/PKCE-S256 transaction primitives and an authorization-start endpoint when an explicit HTTPS authorization endpoint is configured. VSN does not guess issuer endpoints. Token exchange/JWKS verification is still pending.
- Control Plane SQLite state writes add compare-and-swap snapshot generations so a stale process fails instead of silently replacing a newer snapshot. This is a multi-instance correctness primitive, not a distributed datastore.
- Desktop exposes the new native DB CRUD/metadata, Redis writes, PTY wait-read, binary-upload status/digest and local advanced preview controls.

## Explicitly not complete

- Native PostgreSQL/MySQL remote TLS connectors remain conservative/loopback-only; remote TLS-capable external-client paths remain available.
- Native MongoDB CRUD is not implemented.
- PTY output is not yet an unsolicited multiplexed browser stream and sessions do not yet resume across Agent restart.
- Preview is not yet a long-lived WebSocket/SSE tunnel.
- VPS releases are transferred and selected, but application-specific unpack/install/migrate/restart hooks are not run without a future provider policy.
- OIDC authorization-code exchange, discovery/JWKS validation, user mapping, Passkeys/WebAuthn and SAML are not implemented.
- CAS-protected SQLite state detects stale writers but does not provide shared multi-node queue/state synchronization.
