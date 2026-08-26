# PKG-03 03.03 — Research

Task: `03.03 — Package identity, version, publisher and upgrade metadata contract`
Linear: `ABD-78`
Canonical base reviewed: `d1d3e6997878aa16b8d4ad05f094754b5b1699b2`
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`
Reviewed: 2026-08-26

## Repository authority

- PKG-03 architecture contract fixes the product identity source to `apps/desktop/src-tauri/tauri.conf.json`.
- Accepted identity at the task base is `VSN Dev Platform`, version `0.38.1`, identifier `dev.vsn.platform`.
- Canonical 03.02 is DONE at this base; 03.03 remains independently READY.
- 03.03 owns publisher/version/upgrade metadata only. Install scope/elevation is 03.04, exact payload ownership is 03.05, lifecycle execution is 03.06–03.08, signing is 03.22, updater behavior is PKG-04.
- Canonical repository ownership is `Vertex-Systems-Network/vsn-local-server`; the Windows publisher display value is frozen as `Vertex Systems Network`.

## Current platform review

Official Tauri v2 configuration and bundler sources were rechecked on 2026-08-26.

- `bundle.publisher` maps to the Windows Installer Manufacturer property.
- WiX `upgradeCode` must remain stable across upgrades or Windows treats a later release as another application.
- Tauri's default WiX upgrade code is UUIDv5/DNS over `<productName>.exe.app.x64`; Tauri recommends explicitly pinning it to avoid accidental identity changes if the product name changes.
- For the accepted product name `VSN Dev Platform`, that deterministic value is `157f304f-1d1b-55e0-b89c-0610ea27c645`.
- Windows `allowDowngrades=false` blocks installation of an older version over a newer one. This is upgrade-version policy, not install-scope authority.

Primary sources:
- https://v2.tauri.app/reference/config/
- https://v2.tauri.app/distribute/windows-installer/
- https://docs.rs/tauri-utils/latest/tauri_utils/config/struct.BundleConfig.html
- https://docs.rs/tauri-utils/latest/src/tauri_utils/config.rs.html

## Decision

`change_required=false`.

03.03 will make the minimum identity mutation:
- preserve `productName=VSN Dev Platform`;
- preserve `version=0.38.1`;
- preserve `identifier=dev.vsn.platform`;
- set `bundle.publisher=Vertex Systems Network`;
- set `bundle.windows.allowDowngrades=false`;
- pin `bundle.windows.wix.upgradeCode=157f304f-1d1b-55e0-b89c-0610ea27c645`.

No installer mode, payload path, signing key, certificate, updater feed, service, firewall, hosts, resolver or trust-store behavior is authorized by this task.
