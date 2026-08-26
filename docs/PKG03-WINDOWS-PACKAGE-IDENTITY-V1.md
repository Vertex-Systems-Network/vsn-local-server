# PKG-03 Windows Package Identity Contract v1

Task authority: `03.03` / Linear `ABD-78`.
Parent package: PKG-03 Windows Installer.

## Canonical identity

Windows package identity is sourced only from `apps/desktop/src-tauri/tauri.conf.json`.

| Field | Frozen value | Purpose |
|---|---|---|
| Product name | `VSN Dev Platform` | Windows installer/app display identity |
| Product version | `0.38.1` | Current package version authority |
| Application identifier | `dev.vsn.platform` | Stable application identifier |
| Publisher | `Vertex Systems Network` | Windows Installer Manufacturer/publisher |
| WiX UpgradeCode | `157f304f-1d1b-55e0-b89c-0610ea27c645` | Stable MSI upgrade-family identity |
| Downgrade policy | `allowDowngrades=false` | Reject older-over-newer package installation |

## Upgrade invariant

The WiX UpgradeCode is a persistent product-family identifier and must not rotate when the display product name changes. The pinned code equals Tauri's current deterministic UUIDv5 result for `VSN Dev Platform.exe.app.x64`, so this task preserves the existing default upgrade family while making it explicit.

Changing the UpgradeCode after release is a breaking installer-identity change and requires explicit change control.

## Version invariant

03.03 does not independently bump the product. `0.38.1` remains aligned with the existing product/version authority. A future version bump must update the canonical version authorities under the owning release task and must not change the UpgradeCode.

Downgrade prevention is enabled. Detailed MSI/NSIS installation and error-code behavior is not claimed until the lifecycle tasks.

## Publisher invariant

The Windows publisher/manufacturer display value is `Vertex Systems Network`. This is package metadata only; it is not an Authenticode certificate subject and does not authorize signing credentials. Authenticode remains 03.22.

## Reserved downstream authority

- 03.04: current-user/per-machine install scope and elevation.
- 03.05: exact installed payload/resource ownership.
- 03.06–03.08: NSIS/MSI install/uninstall lifecycle.
- 03.22: Authenticode signing/verification.
- PKG-04: updater/apply/rollback behavior.

No firewall, hosts, resolver, trust-store, service, updater or privileged mutation is introduced here.
