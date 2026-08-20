# P30 release-readiness increment — VSN 0.17

0.17 adds source-level packaging and certification gates rather than claiming production signing success:

- Windows WiX/MSI source installs Agent, CLI and updater helper and registers `VSNAgent` as a LocalService Windows service.
- Linux `.deb` staging installs Agent/CLI/updater helper plus a hardened user systemd unit.
- macOS `.pkg` staging installs the same runtime plus a LaunchAgent plist.
- Windows Authenticode signing and macOS product signing/notarization are separate scripts that consume operator/CI credential references instead of embedding signing material.
- updater handoff scripts stop/restart the relevant service/LaunchAgent around the out-of-process updater helper.
- release-gate CI includes Rust multi-OS build/test/clippy, RustSec audit, frontend bundles and unsigned runtime package builds.
- nightly security workflow runs bounded `cargo-fuzz` targets and dependency auditing.
- `scripts/load-control-plane.py` is a bounded concurrent health/load probe; it does not replace full protocol/load testing.

These assets are release engineering source. They are not marked certified until they execute successfully on external Windows/Linux/macOS runners with real signing/notarization and installer acceptance tests.

## Protected signing workflow

`.github/workflows/release-signing.yml` is manual-only and targets the `production-signing` GitHub environment. Windows imports a base64 PFX into the ephemeral CurrentUser store, signs/verifies with SignTool, and macOS imports a P12 into a temporary keychain before `productsign`, `notarytool --wait`, staple and validation. Signing/notarization credentials are referenced only as protected secrets; they are not stored in source. The workflow remains un-certified until executed successfully with real credentials.
