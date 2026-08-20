# VSN roadmap status — 0.10.0

Source of truth: `docs/roadmap-status.json`.

## Summary

- Done / usable baseline: **8** phases
- Meaningful partial: **22** phases
- Pending: **1** phase

## Largest 0.10 gains

- **P8/P9/P17:** native MongoDB plus verified PostgreSQL/MySQL TLS read profiles
- **P10:** Desktop advanced database surfaces
- **P12/P29:** optional shared PostgreSQL Control Plane snapshot store
- **P13/P15/P16/P18:** generic bounded stream protocol foundation
- **P23:** deterministic VPS localhost health-check with optional rollback
- **P28:** real WebAuthn passkey registration/login

## Highest-value remaining work

1. Route multiplexed streams end-to-end across browser ↔ Control Plane ↔ Agent with reconnect/resume.
2. Complete OIDC code exchange/JWKS/account mapping and organization SSO; add SAML.
3. Move distributed queue/presence/session/rate-limit state to shared infrastructure and prove multi-instance behavior.
4. Add provider-native cloud create/destroy/snapshot/clone/migrate workflows.
5. Define policy-governed app deployment hooks and health-driven rollout/rollback.
6. Run real Cargo/Tauri/Windows/Linux/macOS E2E builds, fuzz/load/security tests and release-signing pipeline.
