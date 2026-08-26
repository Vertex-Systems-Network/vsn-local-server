# PKG-03 03.05 — Research

Task: `03.05 — Owned payload/resource manifest and install-root containment`
Linear: `ABD-80`
Canonical base reviewed: `7cd671de8af410ee348083c42c716cce1dd22543`
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`
Reviewed: 2026-08-26

## Repository authority

- 03.01 assigns 03.05 exact path-level installer ownership before install/uninstall cleanup acceptance.
- Canonical 03.01–03.04 are DONE; PKG-03 is `4/25 = 16%`; 03.05 is the only READY task and deterministic cursor.
- 03.05 owns the durable Windows payload ownership map and containment policy. It does not own real install/uninstall (03.06–03.08), Desktop registration/shortcuts (03.09), CLI/Agent placement/discovery/launch (03.10), service registration (03.11), ACL/state separation (03.12), repair/integrity (03.14–03.18), signing (03.22), or updater behavior (PKG-04).
- Current product sources define three PKG-03 durable executable payloads: Desktop package `vsn-desktop`, CLI package `vsn`, and Agent package `vsn-agent`.
- `apps/updater-helper` is deliberately excluded from PKG-03 installer ownership because updater/recovery orchestration belongs to PKG-04.

## Current Tauri review

Official Tauri v2 documentation and current upstream state were rechecked on 2026-08-26.

- Tauri `bundle.externalBin` embeds sidecar binaries and requires target-triple-specific source names.
- Tauri `bundle.resources` supports explicit extra resource paths.
- Windows resource-directory layout and sidecar placement are bundler behavior; 03.10 is the frozen task that owns CLI/Agent placement, discovery and launch.
- An upstream MSI + `externalBin` defect reported in Tauri issue #14681 is closed upstream. Because this task does not own placement, 03.05 does not rely on either the old defect or its fix; 03.10 must empirically certify the locked Tauri version when it realizes placement.

Primary sources:
- https://v2.tauri.app/develop/sidecar/
- https://v2.tauri.app/develop/resources/
- https://v2.tauri.app/reference/config/
- https://github.com/tauri-apps/tauri/issues/14681

## Ownership decision

`change_required=false`.

Freeze exactly three durable executable ownership paths relative to the installer-selected root:

1. `VSN Dev Platform.exe` — Desktop application payload.
2. `bin/vsn.exe` — CLI payload; placement realization remains 03.10.
3. `bin/vsn-agent.exe` — Agent payload; placement realization remains 03.10.

The ownership manifest is root-relative and scope-neutral. 03.04 remains authoritative for whether the installer-selected root is the current-user or per-machine location class.

## Containment decision

Every owned path is a canonical relative Windows path under one logical `${INSTALL_ROOT}` anchor. The contract rejects:
- absolute, drive-qualified, UNC and device paths;
- `.` or `..` traversal;
- alternate data stream `:` syntax;
- control/NUL characters;
- trailing spaces or dots in path segments;
- Windows reserved device names;
- empty segments and mixed/non-canonical separators in the manifest;
- case-insensitive duplicate/colliding owned paths.

Downstream install/uninstall tasks must also fail closed if filesystem reparse-point resolution would escape the selected install root. 03.05 freezes that invariant but does not perform an installer lifecycle.

## Evidence shape

03.05 certification will:
- validate the exact three-entry machine-readable ownership manifest;
- prove each source package identity/version from locked Cargo metadata;
- build `vsn` and `vsn-agent` on GitHub-hosted Windows and verify the expected executable names without installing them;
- exercise positive and malicious containment vectors against an ephemeral root;
- prove no installer execution, privileged mutation, service registration, ACL mutation, signing, updater mutation or external filesystem ownership occurs.
