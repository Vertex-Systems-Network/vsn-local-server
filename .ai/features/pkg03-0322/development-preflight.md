# PKG-03 03.22 Development Preflight

Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Task: `03.22`
Linear: `ABD-97`

## Dependency/state check

- 03.02 deterministic Windows bundle build: DONE
- 03.03 package identity/version/publisher contract: DONE
- 03.14 payload integrity detection: DONE
- canonical PKG-03: 15/25 = 60%
- 03.22: READY
- lane: signing
- max parallel package lanes: 5

## Locked inputs

- Node `22.12.0`
- Rust `1.97.1`
- Tauri CLI `2.11.4`
- product `VSN Dev Platform`
- product version `0.38.1`
- parent plan SHA-256 `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`
- signing digest `SHA256`
- timestamp protocol `RFC3161`
- timestamp digest `SHA256`

## Initial mutation authority

Planning may change only the 03.22 planning/contract bundle.

After planning gates pass, implementation may add only:
- task-owned `scripts/ci/pkg03-0322-*` signing/verification helpers;
- task-owned `.github/workflows/pkg03-0322-*` certification workflow;
- a narrowly scoped signing configuration/adapter only after a preflight proves it is necessary and contains no secret material;
- canonical projection surfaces only after genuine accepted evidence.

Initial authority does not permit:
- production private key/PFX/secret material in Git;
- changing package identity, payload ownership or install scopes;
- service/ACL/network/runtime mutation;
- updater or PKG-05 implementation;
- claiming production-signed acceptance from an ephemeral/self-signed development certificate.

## Fail-closed rule

If no production signing credential/provider is available, the task remains not accepted. A verification-only or ephemeral integration test can prove wiring but cannot project 03.22 DONE as production-signing acceptance.

Any required shared Tauri config mutation must be minimum-scope, secret-free and separately justified by exact preflight evidence.
