# PKG-03 03.14 Lifecycle Review — Installed payload integrity

Reviewed: 2026-08-29
Canonical base: `0eaa4abb7c5e817334f13672952a5901fbbc8fa9`

## Lifecycle under test

03.14 certifies deterministic integrity detection for the installer-owned executable set without performing product repair.

### Expected-hash authority

Expected hashes must come from exact-head build inputs:
- Desktop: exact release executable produced for the installer build.
- CLI: 03.10 staged `vsn.exe` SHA-256.
- Agent: 03.10 staged `vsn-agent.exe` SHA-256.

The installed copy is never allowed to become its own expected baseline.

### Healthy-state detection

For every accepted installer format:
- resolve the accepted install root;
- require exactly the three owned executable relative paths;
- classify each file as `MATCH`, `MISSING`, or `HASH_MISMATCH`;
- a healthy post-install baseline must report `MATCH` for all three.

### Controlled perturbation matrix

Current-user NSIS has no Agent service and therefore proves the complete executable matrix:
- each owned executable deleted once -> `MISSING`;
- each owned executable byte-modified once -> `HASH_MISMATCH`;
- after test-fixture restoration -> `MATCH`.

Per-machine NSIS and MSI/WiX:
- all three owned executables must be healthy `MATCH` after install;
- Desktop and CLI each receive missing and tamper probes;
- Agent remains read-only in these two lifecycles so 03.14 does not add service/running-process coordination.

### Cleanup boundary

Restoration uses the exact bytes captured from the already-verified installed file solely to reset the test fixture between perturbations. It is not installer repair, self-healing, reinstall, or product behavior.

Each lifecycle ends with the already-accepted uninstall path and requires owned executable cleanup.

## Nonclaims

03.14 does not own:
- idempotent reinstall or repair execution: 03.16;
- Agent service coordination: 03.11 / 03.19;
- user-data preservation: 03.17;
- rollback/interrupted recovery: 03.18;
- reboot behavior: 03.20;
- silent deployment: 03.21;
- signing or provenance: 03.22 / 03.23.

The detector is certification tooling only and must not ship in the product.
