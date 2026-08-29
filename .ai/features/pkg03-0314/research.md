# PKG-03 03.14 Research — Installed payload integrity and repair detection

Reviewed: 2026-08-29
Canonical base: `0eaa4abb7c5e817334f13672952a5901fbbc8fa9`
Linear: `ABD-89`
Change required: **false — certification-only**

## Current-source findings

- The frozen ownership manifest defines three installer-owned executables relative to the install root: `VSN Dev Platform.exe`, `bin/vsn.exe`, and `bin/vsn-agent.exe`.
- Accepted 03.10 Windows packaging maps CLI and Agent into `bin/` through `apps/desktop/src-tauri/tauri.windows.conf.json`; exact-head staging already produces SHA-256 values for CLI and Agent.
- Accepted 03.11 adds the Agent Windows service to per-machine NSIS and MSI/WiX lifecycles. 03.14 must not add service stop/start/re-registration or general running-process coordination; those responsibilities remain 03.11/03.19.
- The task name requires detection of missing or tampered owned files. Actual idempotent reinstall/repair is separately owned by 03.16, so 03.14 must not claim that it repairs product state.
- Windows Installer exposes repair modes through `msiexec /f` and `REINSTALLMODE`. Missing-file and different-version detection are native concepts. Checksum-based repair (`c`) applies only to MSI files authored with checksum metadata, so 03.14 cannot assume generic checksum repair for every Tauri-produced payload.
- NSIS does not provide an MSI-style product repair contract. Cross-installer integrity evidence therefore needs an installer-independent SHA-256 detector rather than relying on format-specific repair behavior.

Official references:
- https://learn.microsoft.com/en-us/windows/win32/msi/reinstallmode
- https://learn.microsoft.com/en-us/windows/win32/msi/command-line-options
- https://learn.microsoft.com/en-us/windows/win32/api/msi/nf-msi-msireinstallproductw

## Planned certification direction

1. Build exact-head current-user NSIS, per-machine NSIS and MSI/WiX packages with locked Node/Rust/Tauri inputs.
2. Bind expected SHA-256 values to exact-head build/staging outputs, never to mutable installed files.
3. Install each package through its already-accepted visible lifecycle and enumerate the three owned executable paths only.
4. Require a healthy baseline result of `MATCH` for every present owned executable.
5. Current-user NSIS (no Agent service): prove both `MISSING` and `HASH_MISMATCH` classification for all three owned executables using bounded test-fixture perturbations.
6. Per-machine NSIS and MSI/WiX: prove healthy hash identity for all three owned executables; destructive missing/tamper probes are limited to Desktop and CLI so 03.14 does not introduce Agent service/running-process coordination.
7. Restore test-fixture bytes after each probe and verify the detector returns to `MATCH`; this is test cleanup only and must not invoke installer repair or claim product repair.
8. Uninstall with the already-accepted lifecycle and require owned payload cleanup.
9. Emit exact-head evidence showing expected hashes, observed hashes, classification, perturbation, cleanup, and zero tracked repository drift.

No product/Tauri/installer-template/service/ACL/firewall/hosts/DNS/trust/signing/updater mutation is required.

`change_required=false`
