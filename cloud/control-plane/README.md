# VSN Control Plane — 0.12 development baseline

Current capabilities include:

- persistent Ed25519 Control Plane signing identity and explicit key generator
- bootstrap admin token, scoped roles/API tokens, sensitive approvals and persistent account sessions
- Argon2id passwords, encrypted TOTP with accepted-step replay rejection, single-use recovery codes and WebAuthn passkeys
- verified OIDC authorization-code callback with PKCE, provider discovery/JWKS/issuer/audience/nonce checks and explicit provider-subject account mapping
- single-use device pairing and signed enrollment proof
- persistent outbound WebSocket Agent gateway with signed HTTPS polling/results fallback
- browser live-stream relay for PTY/ConPTY, workspace files, workspace SQLite reads and read-only localhost preview
- relay protocol v2 browser resume token/input acknowledgement/bounded output replay
- transactional SQLite local state with generation CAS and legacy migration
- optional verified-TLS PostgreSQL shared snapshot plus shared devices/pairings, command leases/results, cross-instance relay bus, transactional approvals, signed central audit-chain append and shared rate limiting
- PostgreSQL Control Plane instance heartbeats and Agent route ownership
- fleet/environment records and same-origin browser dashboard

## Local development

Build the dashboard, then run:

```bash
cargo run -p vsn-control-plane
```

Generate a persistent signing keypair:

```bash
cargo run -p vsn-control-plane -- --generate-key
```

For TOTP configure a separate 32-byte base64 `VSN_CONTROL_AUTH_KEY_B64`. Do not reuse the command-signing private key.

For WebAuthn configure:

```text
VSN_WEBAUTHN_RP_ID=control.example.com
VSN_WEBAUTHN_ORIGIN=https://control.example.com
VSN_CONTROL_PUBLIC_ENDPOINT=https://control.example.com
```

HTTPS is required except loopback development. Non-loopback startup requires an explicit public endpoint so browser Origin and relay URLs are not guessed from an untrusted request host.

## Optional PostgreSQL shared store

Set both:

```text
VSN_CONTROL_POSTGRES_DSN=postgresql://...
VSN_CONTROL_POSTGRES_CA_PEM=/absolute/path/to/root-ca.pem
```

The root CA path must exist inside the Control Plane process/container. Optional `VSN_CONTROL_POSTGRES_IMPORT_LOCAL=1` imports a local snapshot only when the PostgreSQL snapshot is absent.

In 0.12 PostgreSQL is authoritative for several operational paths: device/pairing lookup, command lease/result delivery, relay-bus frames, approval decision+enqueue, central audit continuity and rate limits. Snapshot generation CAS remains for the broader state document.

This is still not complete horizontal HA. Browser resume/replay state is owned by the original Control Plane process, Agent reconnect does not reconstruct active relays, and account/session/role/fleet/auth-ceremony data is not yet fully normalized into shared operational tables.

## Production gaps

Durable relay reconstruction, cross-node shared resume state, full preview HTTP asset/cookie/SSE/WebSocket forwarding, generic external-DB live sessions, SAML/SCIM, shared auth/session state, regional relay scaling, operational key rotation, HA/DR, immutable external audit retention and full penetration/load/failure testing remain production work.
