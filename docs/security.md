# Security Baseline v0.2

## Trust boundaries

- Client UI is untrusted for authorization decisions.
- `vsn-agent` is the machine execution boundary.
- Providers are least-privilege components.
- Cloud control plane will never require direct inbound database/server ports.
- Local IPC is not treated as trusted merely because it originates from localhost.

## P2 implemented baseline

### Device identity

- Each Agent security context owns an Ed25519 signing key.
- Device ID is derived from SHA-256 of the public key, not hostname or MAC address.
- Private signing key is kept in the OS credential store.
- Public metadata is persisted separately and is verified against the private key on startup.
- If public metadata and the secure-store private key disagree, startup fails closed.

### Local authenticated IPC

P2 uses a versioned authenticated loopback control channel on `127.0.0.1:49731`.

Every request contains:

- protocol version
- timestamp
- cryptographically random nonce
- command
- parameters
- HMAC-SHA256 authentication code

The Agent enforces:

- loopback-only peers
- 30-second clock-skew window
- HMAC verification
- replay nonce cache
- 1 MiB maximum frame size
- authenticated responses bound to the request nonce

The protocol is intentionally transport-independent. Native Windows named pipes / Unix domain sockets can replace loopback TCP later without changing command semantics.

### Windows service IPC key

A Windows Service normally runs under a different security context than the interactive CLI. Therefore the IPC secret is not stored only in a per-user credential store.

On Windows, P2 provisions a random machine-local IPC key under:

`%PROGRAMDATA%\\VSN\\security\\ipc.key`

The file inherits no ACLs and explicitly grants:

- SYSTEM: Full Control
- Administrators: Full Control
- LocalService SID: Read
- installing user SID: Read

If the ACL operation fails, provisioning fails and the key file is removed. This is required so the LocalService Agent and the explicitly authorized installing user can authenticate the same channel without exposing the key to every local account.

### Audit integrity

Agent lifecycle and authenticated IPC actions are written as JSON Lines events.

Each event includes:

- event UUID
- timestamp
- actor/action/target/result
- previous event hash
- current event SHA-256 hash
- signer public key
- Ed25519 signature

Writes use an exclusive file lock, preventing concurrent requests from creating competing chain heads. Verification checks both the hash chain and each signature.

## Defaults

- Deny by default.
- No public inbound ports required for remote access.
- No reusable long-lived remote shell tokens.
- High-risk operations are separately permissioned.
- Secrets are not written to audit metadata.
- Provider manifests declare required permissions.
- Security initialization fails closed if required secure storage is unavailable.

## Authorization boundary after P3

IPC authentication proves possession of the local IPC credential; it does not grant unrestricted machine authority. P3 routes mutations through `vsn-policy`. Baseline local permissions omit high-risk machine/network/admin/database-destructive/secret-reveal authority, and service mutations are restricted to `VSN-*` managed names. Local authentication is not a substitute for authorization.

The Windows SCM service runs as `LocalService`, not `LocalSystem`. Future operations requiring administrator rights must use a separate narrow privileged-broker/approval design rather than elevating the whole Agent.

## Remote phase requirements

- TLS 1.3.
- Mutual Agent authentication.
- Short-lived user sessions.
- MFA/passkey support at control plane.
- Device pairing and revocation.
- Session and command audit records.
- No direct public database, terminal, or project ports.

## 0.5 remote workspace controls

Remote read/write/execute surfaces do not bypass the Agent policy layer. `files.write`, `terminal.exec` and `database.cli.query` are additionally gated by independent machine-local `RemoteConfig` booleans that default to false. Control Plane IAM can delegate only an exact Agent permission on a signed command; a scoped IAM principal may not create a role or token with permissions outside its own set.

Remote terminal execution is intentionally described as bounded direct-process execution, not sandboxing. A permitted executable can access resources available to the Agent OS account. Production use therefore still needs account MFA/approval policy and potentially isolated execution for high-trust workloads.

External database query mode is conservative defense in depth: one statement, restricted starting statements, selected side-effect clauses/functions denied, PostgreSQL/MySQL read-only session hints, and credential files constrained to a workspace or VSN data. A database credential with server-enforced read-only privileges remains the authoritative protection.

Remote command results are size bounded before signing/upload, the HTTP body limit is explicit, and retained full results are capped in the single-instance Control Plane state.

## 0.12 distributed-control hardening

PostgreSQL-backed Control Plane deployments can use shared authoritative operational tables for device enrollment/pairings, command leases/results, relay-bus frames, sensitive approvals, signed central audit continuity and fixed-window rate limiting. Command leasing uses database row locking, approval decision plus command insertion is transactional, and audit append uses a per-device transaction-level advisory lock plus previous-hash continuity checks.

Browser relay protocol v2 does not treat a Control Plane enqueue as proof that terminal/file input was applied. The Agent emits `InputAck` only after the local side effect succeeds; relay resume advertises Agent-processed input progress and uses a short-lived rotating resume token plus bounded output history. Browser relay authentication is sent inside the established WSS application channel rather than in the WebSocket URL, and stream opening still requires a short-lived Control Plane-signed authorization that the Agent revalidates against its local feature policy.

Cloud CLI lifecycle operations are local `RemoteManage` actions. VSN passes structured arguments directly to provider CLI processes, caps runtime/output, does not accept cloud credentials in lifecycle request payloads, requires explicit destroy confirmation, and keeps cloud mutation commands outside the signed remote Agent command allowlist.

## 0.13 HA session, SCIM and relay checkpoint hardening

When shared PostgreSQL mode is enabled, account sessions are normalized into a shared session table keyed by a SHA-256 token hash; raw bearer session tokens are not persisted in that table. Absolute expiry, idle expiry, revocation and periodic activity touch are evaluated against shared state so a load-balanced request can be authenticated on another Control Plane instance. Legacy snapshot sessions are backfilled only when the shared session table is empty, avoiding repeated startup overwrite of newer shared activity/revocation state. Accounts, roles, API tokens, auth policy and sessions now have normalized shared PostgreSQL records; pending WebAuthn cryptographic ceremony state and some advanced fleet/policy data still require further HA work.

SCIM user provisioning is protected by the existing scoped Control Plane principal/permission model (`control.scim.manage`). A scoped administrator cannot assign a role containing permissions outside its own delegated permission set. SCIM disable, role replacement and deletion revoke account sessions. The 0.13 SCIM surface is a bounded Users baseline, not complete SCIM conformance: Groups, PATCH, Bulk, sorting/ETag semantics and reconciliation remain pending.

Shared relay checkpoints persist only a hash of the rotating browser resume token plus bounded replay/progress metadata. Family-specific Agent reconnect recovery is fail-closed: resumable files and untouched read-only requests may reopen, while a PTY/ConPTY is never silently recreated after Agent loss.

Cloud snapshot/clone/image-copy operations remain local `RemoteManage` actions and are outside the signed remote Agent command allowlist. Snapshot/image creation, clone and image copy each require explicit acknowledgement/confirmation; Azure full VM clone/migration remains deliberately unsupported until disk/network/identity orchestration is deterministic.


## 0.22 source-closure note

Source-side Control Plane/IAM/audit consistency now has explicit `/v1/admin/control/validate`, `/v1/admin/iam/validate`, and `/v1/admin/security/validate` checks. Fleet/API-token normalization described in older paragraphs has since been completed. External HA failure injection, fuzz/load execution and independent penetration certification remain P30 evidence rather than unresolved P12/P19/P21 source functionality.
