# Batch 0.14.0

Hard-way focus: normalize HA identity state, expand SCIM, add durable cancellable external DB read jobs, add SAML trust-policy validation, and strengthen release gating.

Implemented:

- normalized shared PostgreSQL roles, accounts, account sessions, enterprise auth policy, OIDC PKCE transactions and SCIM Groups;
- one-time upgrade backfill and cross-node auth refresh;
- SCIM Users/Groups CRUD plus bounded PATCH and session revocation on security changes;
- durable PostgreSQL/MySQL/MariaDB read-query jobs with bounded output, exact child-process cancellation and fail-closed interrupted recovery;
- local Desktop and signed remote command surfaces for DB jobs;
- validated SAML trust/provider policy without unsafe assertion-consumption claims;
- release-gate CI: schema/static checks, cross-platform Rust fmt/clippy/test/release build, RustSec audit, frontend builds and deterministic source hash manifest.

Explicitly incomplete: SAML assertion/ACS login, SCIM Bulk/ETag, shared WebAuthn ceremony state, generic external DB binary streaming/transactions, full preview SSE/WebSocket tunnel, production platform signing/notarization and stable updater apply/rollback.
