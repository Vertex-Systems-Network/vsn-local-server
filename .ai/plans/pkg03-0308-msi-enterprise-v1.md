# PKG-03 03.08 — MSI/WiX Enterprise Lifecycle v1

Status: frozen task execution contract.
Canonical base: `0ac71c6392c19ad070a9ec442323c46f3c0e08b9`.
Parent package plan: `.ai/plans/pkg03-windows-installer-v1.md`.
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`.
Task: `03.08`.
Linear: `ABD-83`.

## Acceptance criteria

1. Exact-head Windows certification builds the stock Tauri MSI/WiX target with locked package/toolchain inputs and no product/config mutation.
2. The generated MSI is non-empty and evidence records exact path, size and SHA-256.
3. Evidence extracts ProductCode, ProductName, ProductVersion and Manufacturer from the exact built MSI; ProductCode must be a valid GUID.
4. ProductName/version/manufacturer and frozen UpgradeCode semantics remain aligned with the accepted 03.03 identity contract.
5. The runner is Administrator/elevated/high-integrity before the per-machine lifecycle is executed.
6. The installer is launched through normal `msiexec /i <package>` full/default UI. `/quiet`, `/passive`, `/qn`, `/qb`, `/qr`, `/qf` and equivalent UI-suppression switches are forbidden.
7. A visible Windows Installer UI is observed and progressed through normal enabled controls.
8. Successful install resolves to the architecture-appropriate Program Files `VSN Dev Platform` root and `VSN Dev Platform.exe` exists.
9. The exact ProductCode-keyed `HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\{ProductCode}` record exists after install and matches expected DisplayName, DisplayVersion and Publisher/Manufacturer identity.
10. The ARP record exposes Windows Installer uninstall semantics tied to the same ProductCode.
11. No blanket HKCU non-mutation claim is made; stock WiX vendor bookkeeping under HKCU is outside the ARP scope assertion.
12. `bin/vsn.exe` and `bin/vsn-agent.exe` remain absent; 03.10 retains real placement authority.
13. The same exact product is uninstalled through a normal visible Windows Installer UI path without silent/passive switches.
14. After uninstall, the exact ProductCode-keyed HKLM ARP record and installed Desktop executable are absent.
15. Certification proves tracked repository drift is zero and emits exact-source machine-readable evidence.
16. No shortcut semantics, CLI/Agent placement, service/ACL behavior, repair/reinstall/rollback, silent deployment, signing or updater behavior is claimed.
17. Accepted state advances only 03.08 from canonical `7/25` to `8/25`; cursor advances to 03.09 and READY becomes exactly `03.09`, `03.10`, `03.13`, `03.15`.

## Frozen execution shape

### Build

Use GitHub-hosted `windows-2025` with locked Node/Rust/package inputs and build only the MSI target. Do not use a custom WiX template or mutate Tauri configuration.

### Package introspection

Before execution, bind evidence to:
- source SHA;
- MSI path, size and SHA-256;
- ProductCode;
- ProductName;
- ProductVersion;
- Manufacturer;
- expected frozen UpgradeCode from source.

ProductCode is package evidence, not a new checked-in identifier.

### Pre-install state

Record:
- runner Windows identity, Administrator membership, token elevation and integrity;
- expected Program Files root absence;
- exact ProductCode-keyed HKLM ARP absence.

### Normal visible install

Start `msiexec.exe /i <exact-msi>` without UI-suppression flags. Observe and progress the visible Windows Installer UI. No silent/passive wrapper is permitted.

### Installed-state checks

Verify Program Files Desktop payload and exact ProductCode-keyed HKLM ARP identity. Keep CLI/Agent absent.

### Normal visible uninstall

Uninstall the same product via normal Windows Installer UI semantics tied to the exact ProductCode/package, without UI-suppression flags. Observe/progress the visible UI.

### Cleanup

Verify the exact ProductCode-keyed HKLM ARP entry and clean installed Desktop executable are gone. Preserve zero tracked repository drift.

## Exit state

After genuine 03.08 evidence and reconciliation:
- `done=8`, `percent=32.0`, `complete=false`;
- `03.08=DONE`;
- `03.09`, `03.10`, `03.13`, `03.15` are READY;
- deterministic cursor `03.09`.
