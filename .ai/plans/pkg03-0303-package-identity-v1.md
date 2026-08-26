# PKG-03 03.03 — Package Identity, Publisher and Upgrade Metadata v1

Status: frozen task execution contract.
Canonical base: `d1d3e6997878aa16b8d4ad05f094754b5b1699b2`.
Parent package plan: `.ai/plans/pkg03-windows-installer-v1.md`.
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`.
Task: `03.03`.
Linear: `ABD-78`.

## Acceptance criteria

1. Existing Tauri identity remains exactly `VSN Dev Platform` / `0.38.1` / `dev.vsn.platform`.
2. Windows installer publisher is explicitly frozen to `Vertex Systems Network` through `bundle.publisher`.
3. WiX upgrade identity is explicitly pinned to `157f304f-1d1b-55e0-b89c-0610ea27c645` and the repository-local Tauri CLI resolves the same code.
4. Windows downgrade policy is explicitly `allowDowngrades=false`.
5. The metadata is accepted by the locked repository-local Tauri CLI on GitHub-hosted Windows.
6. Evidence is bound to exact source SHA, toolchain/config digests and inspected upgrade code.
7. No installer is executed and no privileged/system state is mutated.
8. 03.04 install-scope/elevation, 03.05 payload ownership, 03.06–03.08 lifecycle, 03.22 signing and PKG-04 updater remain untouched.
9. Pre-evidence state must be canonical `2/25` with 03.02 DONE; accepted state increments only 03.03 to `3/25`.

## Frozen identity decisions

- Product name: `VSN Dev Platform`
- Product version: `0.38.1`
- Application identifier: `dev.vsn.platform`
- Publisher/Manufacturer: `Vertex Systems Network`
- WiX UpgradeCode: `157f304f-1d1b-55e0-b89c-0610ea27c645`
- Windows downgrade policy: blocked (`allowDowngrades=false`)
- Identity source: `apps/desktop/src-tauri/tauri.conf.json`

The UpgradeCode is the explicit pin of Tauri's deterministic UUIDv5 value for `VSN Dev Platform.exe.app.x64`; future display-name changes must not silently fork Windows upgrade identity.

## Evidence

Required workflow: `PKG-03 03.03 Package Identity`.
Required validator: `python scripts/ci/validate-pkg03-0303.py`.
Required governance: AI Planning Governance, Repository Governance, PKG-03 Acceptance Sequence.

## Exit-state rule

After genuine 03.03 evidence:
- `03.03=DONE`;
- PKG-03 advances from `2/25` to `3/25`;
- 03.04 and 03.05 remain READY;
- 03.02 remains DONE;
- cursor advances from 03.03 to 03.04;
- 03.06 remains blocked until 03.02–03.05 are all DONE.
