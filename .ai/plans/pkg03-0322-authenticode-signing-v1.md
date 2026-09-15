# PKG-03 03.22 — Authenticode signing integration and verification plan v1

Status: frozen task plan
Task: `03.22`
Linear: `ABD-97`
Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`

## Objective

Integrate a secret-safe Windows Authenticode signing path for the accepted VSN Windows candidates and certify exact-head signature validity, expected publisher identity, SHA-256 signing, RFC 3161 timestamping and tamper rejection without changing package/runtime semantics.

## Acceptance

The exact-head Windows certification must:

1. build the accepted exact-head Windows candidate set;
2. record unsigned package/binary SHA-256 before signing;
3. invoke a secret-external signing provider/command without persisting credentials;
4. require SHA-256 Authenticode digest;
5. require RFC 3161 timestamp with SHA-256 for production acceptance;
6. verify signed artifacts with Windows-native Authenticode verification;
7. bind expected signer/publisher identity and certificate/public metadata without exposing private material;
8. record signed SHA-256 and preserve unsigned/signed provenance distinction;
9. copy a signed artifact, tamper the copy deterministically, and require verification failure;
10. prove package identity/install semantics and accepted owned-payload contract were not widened;
11. scan tracked changes/evidence for forbidden key/PFX/password/token material;
12. prove zero tracked repository drift.

## Signing-provider architecture

Provider-specific credentials remain outside the repository. Tauri `bundle.windows.signCommand` or an equivalent task-owned adapter may be used only with non-secret configuration committed to Git. Secret values must enter at runtime through the CI/provider identity boundary.

A test certificate may be used for integration wiring only and must be explicitly marked non-production. It cannot satisfy the production timestamp/publisher acceptance gate.

## Boundaries

- No SmartScreen reputation guarantee.
- No production CA/account provisioning.
- No updater/update-feed signing.
- No PKG-05 release publishing.
- No package identity/version/upgrade-code, service, ACL, network or runtime mutation unless separate bounded change control is approved.
- 03.23 owns SBOM/provenance release handoff.

## Governance sequence

1. freeze planning on current canonical main;
2. require AI Planning Governance, Repository Governance, PKG-03 Acceptance Sequence, Engineering Contract Governance and Operational Governance;
3. only after 5/5 green, add task-owned verification/integration surfaces;
4. if a shared signing config is necessary, document minimum-scope change control before mutation;
5. run exact-head Windows signing verification;
6. independently verify evidence/artifact bytes;
7. only then project 03.22 DONE.
