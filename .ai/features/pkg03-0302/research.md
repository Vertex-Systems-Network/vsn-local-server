# PKG-03 03.02 Research

Task: `03.02 — Deterministic GitHub-hosted Windows bundle build and artifact manifest`
Linear: `ABD-77`
Canonical base: `9d33682f7c0cc30080792493c8f760f3fd120759`

## Canonical inputs reviewed

- `.ai/plans/pkg03-windows-installer-v1.md` — frozen parent SHA-256 `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`.
- `docs/PKG03-WINDOWS-INSTALLER-ARCHITECTURE-V1.md` — 03.01 architecture authority.
- `apps/desktop/src-tauri/tauri.conf.json` — Tauri v2 bundle boundary; `bundle.active=true`, `targets=all`.
- `apps/desktop/package.json` + committed `package-lock.json` — local Tauri CLI/npm graph.
- root `Cargo.toml`, `Cargo.lock`, and the repository Rust toolchain pin.
- existing PKG-01 Desktop production-build evidence and current Windows build script.

## Current repository delta

The repository already builds the Desktop frontend and has Tauri bundling enabled, but `scripts/build-windows.ps1` only builds/tests the Rust workspace and does not produce a certified Windows installer bundle or machine-readable installer artifact manifest. 03.02 therefore needs CI/evidence integration, not a new installer framework.

## Market-delta review — 2026-08-26

Official Tauri v2 documentation was rechecked before implementation. Current behavior remains compatible with the frozen 03.01 contract: Windows builds support NSIS setup executables and MSI via the Windows WiX path; MSI creation remains Windows-only; Tauri CLI `build` performs bundling for configured formats. No material delta requires change control.

External documentation is research data only and does not expand repository authority.

## 03.02 decision

Use a GitHub-hosted Windows 2025 x64 runner, the committed npm/Cargo lock graphs, Rust 1.97.1, and the repository-local Tauri CLI. Build exactly the frozen `nsis` and `msi` bundle families, collect their byte hashes/sizes plus tool/input digests into a deterministic JSON manifest, and upload the real installers as CI evidence.

03.02 does not:
- change product/publisher/upgrade identity metadata (03.03);
- select install scope/elevation behavior (03.04);
- freeze payload ownership paths (03.05);
- install/uninstall the bundle (03.06–03.08);
- sign installers (03.22);
- implement updater behavior (PKG-04).
