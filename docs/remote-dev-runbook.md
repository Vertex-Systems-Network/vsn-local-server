# Remote Development Runbook — 0.10 baseline

This runbook exercises the current outbound remote path. It is appropriate for controlled development/staging tests, not a claim of production SaaS readiness.

## Trust model

1. Every attached machine owns an Ed25519 device keypair.
2. The Control Plane owns a separate Ed25519 command-signing keypair.
3. Device pairing uses a single-use expiring nonce.
4. Agent enrollment, gateway polling and command results are device-signed.
5. Every remote command is Control-Plane-signed, short-lived and device-bound.
6. The Agent validates signature, TTL, replay state, device binding, permission and local feature opt-in before dispatch.
7. Sensitive actions can require Control Plane approval, but Agent-side default-deny/local opt-ins remain authoritative.
8. Signed Agent audit batches can be synchronized centrally and verified for signer/hash/chain continuity.

## 1. Build the dashboard

```powershell
cd cloud\dashboard
npm install
npm run build
cd ..\..
```

## 2. Configure Control Plane secrets

Generate a persistent signing key:

```powershell
cargo run -p vsn-control-plane -- --generate-key
```

Set at least:

```powershell
$env:VSN_CONTROL_ADMIN_TOKEN = "replace-with-a-long-random-bootstrap-token"
$env:VSN_CONTROL_PRIVATE_KEY_B64 = "<generated private key>"
$env:VSN_CONTROL_AUTH_KEY_B64 = "<independent random 32-byte base64 key>"
$env:VSN_CONTROL_BIND = "127.0.0.1:9070"
```

For local passkey testing:

```powershell
$env:VSN_WEBAUTHN_RP_ID = "localhost"
$env:VSN_WEBAUTHN_ORIGIN = "http://localhost:9070"
```

For a real staging hostname use HTTPS and set the RP ID/origin to that public hostname/origin.

## 3. Start the Control Plane

```powershell
cargo run -p vsn-control-plane
```

Open `http://127.0.0.1:9070` for local development.

## 4. Pair an attached machine

Create a pairing code in the dashboard, then configure the machine while remote execution is initially disabled:

```powershell
vsn remote configure http://127.0.0.1:9070 <CONTROL_PLANE_PUBLIC_KEY> false
vsn remote enroll <PAIRING_NONCE>
vsn remote configure http://127.0.0.1:9070 <CONTROL_PLANE_PUBLIC_KEY> true
```

Restart the Agent if required by the current configuration lifecycle.

## 5. Create safer day-to-day auth

Use the bootstrap token to create the first role/account. The current baseline supports:

- Argon2id password login
- TOTP MFA
- single-use recovery codes
- WebAuthn/passkey registration and login
- scoped API tokens
- session expiry/logout

Passkey registration requires an existing authenticated account session. Passkey login is public and challenge state is kept server-side for a short lifetime. An in-progress ceremony is intentionally lost on Control Plane restart; registered passkeys persist.

OIDC currently supports authorization-start state/nonce/PKCE preparation only. Authorization-code exchange, JWKS/token validation and account mapping are not yet complete.

## 6. Exercise remote commands

The Agent prefers the persistent outbound WebSocket path and falls back to signed HTTPS polling/results. Current signed-command surfaces include safe machine/project/runtime/process inventory, workspace-contained file operations subject to local opt-ins, bounded process/PTY operations, conservative database reads and localhost preview.

The generic `vsn-stream` protocol is routed through authenticated Browser ↔ Control Plane ↔ Agent channels for bounded PTY, files, SQLite read queries and localhost preview. Shared PostgreSQL can carry cross-instance relay envelopes/checkpoints. Generic local WebSocket/full asset-cookie tunneling and server-side external DB transactions remain incomplete.

## 7. Optional shared PostgreSQL state

A staging Control Plane can use a shared PostgreSQL snapshot store:

```powershell
$env:VSN_CONTROL_POSTGRES_DSN = "postgresql://..."
$env:VSN_CONTROL_POSTGRES_CA_PEM = "C:\path\to\trusted-root-ca.pem"
```

Use `VSN_CONTROL_POSTGRES_IMPORT_LOCAL=1` only for an explicit one-time import when no shared snapshot exists.

The store provides shared durable snapshot data plus generation-CAS stale-writer protection and normalized operational tables for devices, commands/results, approvals, audit, rate limits, relay routing/checkpoints, accounts/roles/sessions/auth policy, API tokens, SCIM groups and fleet group/environment metadata. In-flight WebAuthn cryptographic ceremony state remains owner-node fail-closed.

## 8. Public staging deployment

Use the HTTPS deployment skeleton under `cloud/deploy`; keep backend port `9070` internal and expose Caddy/TLS rather than the Control Plane directly. The attached developer machine still does not need public MySQL/PostgreSQL/project ports or router port forwarding.

## Current production gaps

Before production/enterprise deployment VSN still needs:

- end-to-end browser/Control-Plane/Agent multiplexed streaming with reconnect/resume
- OIDC token/JWKS/account mapping and SAML
- shared distributed queue/presence/session/rate-limit infrastructure
- operational key rotation/recovery and external immutable audit retention
- provider-native cloud lifecycle and policy-governed deploy hooks
- fuzz/load/penetration testing, HA/DR drills and signed release operations
