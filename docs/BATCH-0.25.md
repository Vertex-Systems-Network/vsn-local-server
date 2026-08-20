# Batch 0.25 — maximum close-first source-closure sprint

VSN 0.25 source-closes the final two partial source phases, P23 Cloud Workspaces and P25 Extensions. P0–P29 are therefore source-closed. P30 remains the sole pending phase because Stable 1.0 requires external/native evidence rather than additional source labels.

## P23 closure

- AWS/Azure/GCP create/status/start/stop/destroy/snapshot/clone lifecycle is explicit.
- Azure incremental snapshots use managed copy-start semantics.
- Azure full snapshots/managed disks use a bounded direct-copy path with temporary source/target SAS grants, AzCopy PageBlob transfer, revoke attempts, failure cleanup and copy-status inspection.
- Provider values use structured argv; no browser-supplied cloud credentials or shell interpolation are introduced.

## P25 closure

- Signed install/uninstall, dependency lifecycle and provider manifest resolution remain mandatory.
- Linux executable isolation uses Bubblewrap.
- Windows executable isolation uses the native `vsn-extension-appcontainer.exe` AppContainer helper.
- macOS executable isolation uses a temporary App Sandbox application bundle and codesign entitlements.
- Unsupported capabilities fail closed and no platform has an unsandboxed fallback.

## Release boundary

P30 owns native compilation, real provider/cloud execution, AppContainer helper compilation, MSI acceptance, macOS signing/notarization, updater acceptance, fuzz/load/HA/DR and penetration evidence. 0.25 does not claim Stable 1.0.
