# VSN Batch 0.22 — maximum close-first sprint

0.22 closes six additional finite source/product phases rather than opening broad new feature branches.

- **P3:** macOS launchd start/stop/restart joins Windows SCM and Linux systemd; native service provider descriptor/conformance is exposed through Agent/CLI.
- **P6:** formal `ProjectProvider` SDK, versioned descriptor/conformance, builtin template catalog and existing workspace-contained bounded bootstrap execution.
- **P12:** `/v1/admin/control/validate` checks account/role/session/token references, Fleet topology, approval targets/permissions, auth policy, live cluster registration and Team Vault key availability.
- **P19:** `/v1/admin/iam/validate` validates role permission syntax and account/token role references while preserving scoped-delegation rules.
- **P21:** `/v1/admin/security/validate` verifies signed central audit events and sampled per-device hash continuity plus auth policy state.
- **P29:** `scripts/control-plane-dr.py` provides bounded PostgreSQL backup, SHA-256 manifest verification and explicit-confirm restore; `scripts/source-readiness.py` separates source closure from P30 external certification.

External build/signing/notarization, native OS acceptance, HA/DR drills, load/fuzz/penetration evidence and stable-release acceptance remain P30 certification work and are not silently counted as source gaps in these closed phases.
