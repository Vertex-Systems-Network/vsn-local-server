# VSN Batch 0.4.0

## Added in this batch

- Runtime catalog, platform selection, HTTPS/file artifact acquisition, SHA-256 validation, extraction, registry, shims, activation and uninstall
- Managed child processes and CPU/RAM metrics baseline
- Hosts-file `.test` management, mkcert integration and Caddy configuration/lifecycle helper
- Project dependency/remediation report and allowlisted bootstrap execution
- Real read-only SQLite DatabaseProvider
- Docker/Podman lifecycle and Compose orchestration baseline
- Encrypted local vault
- Tauri/React desktop source with runtime, process, container, DB, network and remote surfaces
- Signed Agent poll/result protocol and outbound remote loop
- Control Plane with pairing, signed command queue, replay windows, durable single-node state and browser dashboard
- Container/Caddy deployment skeleton for an HTTPS Control Plane endpoint
- Signed extension-manifest trust-root verification baseline

## Security changes

- Remote command-to-permission binding is enforced Agent-side.
- High-risk delegated remote permissions remain denied.
- Control Plane poll/result replay is bounded and rejected.
- Network mutations require explicit OS elevation and are not silently elevated by desktop/remote clients.
- Vault reveal is separated from normal local secret-management permissions.
- Attached PCs do not require public inbound project/database ports for the current remote model.

## Deliberately not marked complete

- Full Cargo/Tauri build verification in the artifact environment
- Production account auth/passkeys/MFA
- Transactional remote command inflight/ack/retry
- Remote terminal/files/database proxy/private preview tunnel
- PostgreSQL/MySQL/Mongo/Redis production DB providers
- Official signed runtime catalogs
- Extension runtime sandbox/loader
- Cloud workspace provisioning and AI automation
