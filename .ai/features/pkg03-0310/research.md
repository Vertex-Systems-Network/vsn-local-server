# PKG-03 03.10 Research — CLI and Agent payload placement

Reviewed: 2026-08-27
Canonical base: `4f5e8ab30f030e758c52c4ca4ac08f73f896247a`
Linear: `ABD-85`
Change required: **true**

## Current-source findings

- The frozen ownership manifest declares `bin/vsn.exe` and `bin/vsn-agent.exe`, both owned by 03.10, and both are currently `declared-not-yet-packaged`.
- The accepted Tauri config currently packages only the Desktop application; it has no `bundle.resources` or `bundle.externalBin` entry.
- Workspace package names are `vsn` and `vsn-agent`, version `0.38.1`, producing Windows release binaries `vsn.exe` and `vsn-agent.exe`.
- Tauri v2 supports additional bundled resources and allows an object map from source paths to explicit resource-relative destinations.
- On Windows, Tauri resolves the application resource directory to the directory containing the main executable. This makes a resource mapping to `bin/vsn.exe` and `bin/vsn-agent.exe` compatible with the frozen install-root ownership paths.
- `externalBin` is sidecar-oriented and applies target-triple naming semantics; it is not the preferred mechanism for the frozen human-facing `bin/*.exe` placement contract.
- 03.10 owns physical placement/discovery/launch only. Windows service registration remains 03.11; ACL/state separation remains 03.12; PATH/environment mutation is not implicitly authorized.

Official references:
- https://v2.tauri.app/develop/resources/
- https://v2.tauri.app/develop/sidecar/
- https://docs.rs/tauri-utils/latest/tauri_utils/platform/fn.resource_dir.html

## Platform delta

A product configuration delta is required because CLI and Agent are declared owned payloads but are not yet bundled.

Planned implementation direction:
1. Build exact locked release binaries for `vsn` and `vsn-agent`.
2. Stage deterministic copies under the Desktop bundling context.
3. Add a narrow Tauri `bundle.resources` map that installs them as `bin/vsn.exe` and `bin/vsn-agent.exe`.
4. Certify both NSIS current-user and MSI/WiX per-machine placements.
5. Prove direct launch from the installed absolute paths without claiming service registration or PATH mutation.

`change_required=true`
