# Batch 0.5.0 — Remote workspace surfaces, DB adapters and reliable delivery

## Implemented baseline

- Workspace-root sandbox for file list/read/atomic text write operations.
- Direct-process terminal runner with workspace-contained cwd, 60-second timeout and bounded stdout/stderr. It does not perform shell interpolation itself; explicitly launching an interpreter/shell remains powerful and remote use is therefore local-opt-in.
- CLI-backed PostgreSQL, MySQL, MariaDB, MongoDB and Redis discovery/introspection adapters.
- PostgreSQL/MySQL/MariaDB single-statement query baseline with conservative read-only checks and no password command-line arguments. PostgreSQL and MySQL also apply client/session read-only hints; MariaDB relies on the conservative query filter plus server-side account privileges in this baseline.
- Database credential config paths are restricted to configured workspaces or VSN-owned data.
- Private local preview fetcher restricted to loopback HTTP GET/HEAD, redirects disabled and bounded response size.
- Remote command delivery state machine: queued -> inflight lease -> completed/failed, expiry and bounded retries.
- Agent durable pending-result outbox prevents duplicate command execution after delivery/ACK loss. Cached results are re-signed with a fresh result nonce/timestamp when resent, and removed only after the Control Plane acknowledges them. A fail-closed execution record is persisted before a remote command runs so an Agent crash does not automatically re-run the same side-effecting command.
- Control Plane result acknowledgement is idempotent for already completed command/session pairs.
- Control Plane custom roles and high-entropy scoped API tokens with revocation. Scoped IAM principals cannot mint roles/tokens broader than their own permission set.
- Agent-side local opt-in gates for remote terminal execution, remote file writes and remote external-DB queries. All default to disabled.
- Remote command executions are recorded in the local signed audit chain with command/session/permission metadata.
- Tauri desktop surfaces for Projects, Services, Files, bounded Terminal, external DB inspection/query and private preview.
- Browser Control Plane surfaces for delivery status, custom roles/tokens and signed command composition.
- Provider examples for MySQL, MariaDB, MongoDB and Redis.
- JSON Schema validation script now validates provider manifests against their provider contract, not only JSON syntax.

## Security boundaries retained

- Attached machines still require no public inbound development/database ports.
- Remote commands remain short-lived, device-bound and signed by the Control Plane.
- The Agent still maps each exposed remote command to one exact delegated permission.
- High-risk baseline permissions (`machine.manage`, `network.manage`, `terminal.admin`, `database.destructive`, `secrets.reveal`) remain unavailable to remote delegated principals until an approval/MFA policy exists.
- Remote terminal is not an OS sandbox. Enabling `allow_remote_terminal` grants a scoped principal process-execution capability under the Agent account and should be treated as sensitive.
- DB "read-only" mode is defense in depth, not a replacement for a database account with read-only privileges. Vendor functions can have side effects that generic SQL filtering cannot perfectly model.
- MongoDB and Redis baseline expose introspection only; arbitrary scripts/commands are not exposed.
- Preview relay is a bounded fetch/result path, not yet a live streaming/WebSocket tunnel.

## Still intentionally incomplete

- Native protocol drivers and full CRUD for PostgreSQL/MySQL/MariaDB/MongoDB/Redis.
- PTY/WebSocket interactive terminal and scalable relay infrastructure.
- Streaming/binary remote file transfer.
- Native DB tunnel/proxy sessions.
- Live private preview tunnel with WebSocket/SSE forwarding.
- Production user accounts, passkeys/MFA, recovery and destructive-action approval workflow.
- Multi-instance transactional Control Plane storage; current durable JSON state remains single-instance baseline.
- Rust compiler-backed build/test verification in the artifact environment.
