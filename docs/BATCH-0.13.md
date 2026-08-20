# Batch 0.13.0

## Implemented

- Shared PostgreSQL relay checkpoints and bounded replay frames with hashed resume tokens.
- Safe Agent stream reconnect: file upload/download reopen from progress; untouched one-shot read-only preview/database relays may retry; terminal PTY remains fail-closed.
- Stream engine non-zero sequence reconstruction and restored-history byte/frame bounding.
- Shared PostgreSQL account sessions with hash-only bearer lookup, touch, logout, account-wide revoke, cleanup and guarded one-time migration from legacy snapshot sessions.
- SCIM 2.0 User provisioning baseline with scoped permission, bounded filter/pagination, create/read/replace/delete and delegation checks.
- Cloud CLI snapshot/image operations for AWS/Azure/GCP and confirmed AWS/GCP clone baselines; Azure full clone intentionally unsupported.
- CLI and cloud provider manifests expose snapshot/clone where actually implemented.
- New schemas/runbooks for relay checkpoints, SCIM Users and cloud snapshot/clone requests.

## Explicitly not claimed

- PTY/ConPTY reconstruction after Agent restart.
- Full external DB streaming cancellation/transactions.
- Complete preview asset/SSE/WebSocket tunnel.
- SAML or complete SCIM Groups/PATCH/Bulk lifecycle.
- Azure full VM clone or provider-native SDK orchestration.
- Full HA normalization of accounts/roles/fleet/WebAuthn/OIDC pending transactions.
- Native Rust/Tauri build validation in this execution environment.
