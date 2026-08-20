# Shared HA authentication state — VSN 0.14

When `VSN_CONTROL_POSTGRES_DSN` is configured, authentication state is progressively normalized into shared PostgreSQL tables.

Shared authoritative records now include roles, accounts, account sessions, enterprise auth policy, SCIM groups and one-time OIDC PKCE transactions. Raw account bearer tokens are not persisted; only SHA-256 token hashes are stored in the session table.

On startup, legacy snapshot roles/accounts/groups/policy are backfilled only when their corresponding normalized table/record is empty. After that, shared records are the cross-node source and authentication refresh reloads them before authorization decisions.

OIDC transaction state uses an atomic consume operation so the same state value cannot be successfully consumed by two Control Plane nodes. Account session touch/logout/revoke is also shared.

Registered WebAuthn passkeys are part of the persistent account record. In-flight WebAuthn registration/authentication ceremony state remains process-local in 0.14 and therefore is fail-closed across a process restart or wrong-node callback rather than being serialized through an unsafe library feature.

Still not fully normalized: API-token registry, fleet/environment topology and in-flight WebAuthn ceremonies.
