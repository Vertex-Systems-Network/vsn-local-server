# Control Plane container deployment baseline — 0.10

This is a staging/development deployment skeleton, not the final production SaaS stack.

Generate a persistent Control Plane signing keypair:

```bash
cargo run -p vsn-control-plane -- --generate-key
```

Store the private key in `VSN_CONTROL_PRIVATE_KEY_B64`; Agents pin the corresponding public key. Generate a separate random 32-byte base64 `VSN_CONTROL_AUTH_KEY_B64` for TOTP-secret encryption.

For passkeys set the public relying-party values:

```text
VSN_WEBAUTHN_RP_ID=control.example.com
VSN_WEBAUTHN_ORIGIN=https://control.example.com
```

Copy `.env.example` to `.env`, point DNS at the server and start:

```bash
docker compose --env-file .env up -d --build
```

Caddy is the public HTTPS/WebSocket endpoint; port `9070` remains internal.

## Persistence modes

Default mode uses the `vsn_control_data` volume and transactional SQLite/WAL snapshot state with generation CAS.

For an optional shared PostgreSQL snapshot backend, set `VSN_CONTROL_POSTGRES_DSN` and `VSN_CONTROL_POSTGRES_CA_PEM`. The CA path must be mounted into the Control Plane container; add a read-only bind/secret mount appropriate to your deployment and point the env value to that in-container absolute path. Set `VSN_CONTROL_POSTGRES_IMPORT_LOCAL=1` only for an explicit one-time import when the shared snapshot does not yet exist.

The PostgreSQL backend gives multiple instances access to the same durable snapshot and rejects stale generation writes. It does not yet distribute command queues, gateway presence, sessions, rate limits or pending auth transactions, so do not treat it as full active/active HA.

Production still requires distributed runtime state/queueing, key rotation/recovery, backup/restore drills, OIDC/SAML completion, regional gateway scaling, security/load testing and signed release operations.
