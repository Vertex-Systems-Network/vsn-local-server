# Batch 0.8.0 — native DB adapters, PTY/ConPTY, persistent gateway, account/TOTP auth

## Delivered in this batch

- Native Rust PostgreSQL read/browse/query baseline restricted to loopback while using `NoTls`; existing external client path remains for remote/TLS-oriented PostgreSQL workflows.
- Native Rust MySQL read/browse/query baseline restricted to loopback; query gate rejects multi-statement/destructive and selected file/DoS functions. Existing external client path remains for remote/TLS-oriented MySQL/MariaDB workflows.
- Native Redis inspect/get with plaintext restricted to loopback and `rediss://` required for non-loopback URLs.
- Real PTY/ConPTY session backend through `portable-pty`: start, write, bounded read, resize, status, stop, remove, list; pipe sessions remain available.
- Persistent WebSocket Agent channel (`/v1/agent/ws`) using the same signed poll/result envelopes, replay checks, leases and durable result semantics as HTTPS. Agent falls back to HTTPS automatically.
- Persistent Control Plane users and sessions: Argon2id PHC password hashes, encrypted TOTP secrets, matched-step replay rejection, session expiry/idle policy and logout; admin update operations cover disable/enable, password reset/change, role reassignment and TOTP clearing with session revocation.
- Dashboard account create/login/TOTP-enroll/logout surfaces.
- Existing VPS workspace prepare/status/remove-empty lifecycle after strict key-only SSH preflight and fixed validated remote workspace paths.
- Desktop surfaces for native DB adapters and PTY/ConPTY sessions.

## Deliberately still partial

- Native external database CRUD, TLS policy for every native driver, MongoDB native Rust driver and streaming DB proxy.
- Browser/remote byte-stream PTY transport and terminal session resume.
- Passkeys/WebAuthn ceremonies and OIDC authorization-code callback/session exchange.
- Regional gateway relays, QUIC, distributed presence, multi-instance state/queues.
- Cloud provider VM create/destroy/apply and production billing/tenant isolation.

## Security posture

The Agent remains the execution boundary. Attached machines need no inbound development/database port. Sensitive remote operations remain permission checked and locally opt-in where configured. Unknown DB protocols are not guessed.
