# PKG-03 03.10 Research — CLI and Agent payload placement

Reviewed: 2026-08-27
Canonical base: `4f5e8ab30f030e758c52c4ca4ac08f73f896247a`
Linear: `ABD-85`
Change required: **true**

## Current-source findings

- The frozen ownership manifest declares `bin/vsn.exe` and `bin/vsn-agent.exe`, both owned by 03.10, and both are currently `declared-not-yet-packaged`.
- The accepted base Tauri config packages only the Desktop application; it has no CLI/Agent resource mapping.
- Workspace package names are `vsn` and `vsn-agent`, version `0.38.1`, producing Windows release binaries `vsn.exe` and `vsn-agent.exe`.
- Tauri v2 supports `bundle.resources` as a source-to-destination map.
- On Windows, Tauri's application resource directory resolves to the directory containing the main executable, so resource targets `bin/vsn.exe` and `bin/vsn-agent.exe` align with the frozen install-root ownership paths.
- Tauri automatically merges `tauri.windows.conf.json` with the base config on Windows. This allows the payload packaging delta to remain Windows-only and avoids changing Linux/macOS bundle behavior.
- Tauri supports `build.beforeBundleCommand`, which runs before installer bundling. A Windows-only hook can therefore build/stage the two workspace executables before NSIS/WiX consume the resource map.
- `externalBin` is sidecar-oriented and applies target-triple naming semantics; it is not the preferred mechanism for the frozen human-facing `bin/*.exe` placement contract.
- 03.10 owns physical placement/discovery/launch only. Windows service registration remains 03.11; ACL/state separation remains 03.12; PATH/environment mutation is not implicitly authorized.

Official references:
- https://v2.tauri.app/develop/resources/
- https://v2.tauri.app/develop/configuration-files/
- https://v2.tauri.app/reference/cli/
- https://docs.rs/tauri-utils/latest/tauri_utils/platform/fn.resource_dir.html
- https://docs.rs/tauri-utils/latest/tauri_utils/config/struct.BuildConfig.html

## Platform delta

A Windows-specific product configuration delta is required because CLI and Agent are declared owned payloads but are not yet bundled.

Planned implementation direction:
1. Add `apps/desktop/src-tauri/tauri.windows.conf.json`.
2. Add a Windows-only `beforeBundleCommand` that invokes a task-owned staging script.
3. The staging script builds exact locked release binaries for `vsn` and `vsn-agent`, then hash-preserving copies them to a deterministic task staging directory.
4. The Windows config maps those staged sources to `bin/vsn.exe` and `bin/vsn-agent.exe` through `bundle.resources`.
5. Certify both NSIS current-user and MSI/WiX per-machine placements and direct absolute-path launch.
6. Keep base `tauri.conf.json` and non-Windows bundle behavior unchanged.

`change_required=true`
