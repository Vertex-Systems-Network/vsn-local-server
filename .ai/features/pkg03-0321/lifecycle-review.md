# PKG-03 03.21 Lifecycle Review — unattended/silent deployment

Reviewed: 2026-09-03
Canonical base: `3edb4e1dcd2c062e7b2e270cde626c90a2c5459f`
Task: `03.21`
Linear: `ABD-96`

## Matrix

| Surface | Silent invocation | Required installed state | Uninstall |
| --- | --- | --- | --- |
| current-user NSIS | exact setup + `/S` | `%LOCALAPPDATA%\VSN Dev Platform`, HKCU ARP, Desktop/CLI/Agent payload, machine service absent | exact installed `uninstall.exe /S`; root + HKCU ARP removed |
| per-machine NSIS | exact setup + `/S` | `%ProgramFiles%\VSN Dev Platform`, HKLM ARP, Desktop/CLI/Agent, `VSN-Agent` running | stop service through installed Agent, then exact `uninstall.exe /S`; service/root/HKLM ARP removed |
| MSI/WiX | `msiexec /i <exact-msi> /quiet /norestart /L*V` | Program Files payload, exact ProductCode ARP, `VSN-Agent` running | stop service, then `/x <ProductCode> /quiet /norestart /L*V`; service/root/ARP removed |

## Zero-input proof

For every operation:
1. supply only the frozen command-line arguments;
2. send no UIAutomation, keyboard, mouse, stdin, prompt answer, or dialog action;
3. observe the root/descendant process family while it runs and record any visible titled window;
4. require the expected state transition and native process completion inside a fixed timeout;
5. fail if any visible installer-family titled window is observed in strict silent mode;
6. fail closed and retain diagnostics on timeout rather than killing a prompt and calling the operation successful.

## Exit/reboot contract

- NSIS silent operations must return native exit `0`.
- MSI silent install/uninstall may return only `0` or reboot-required `3010`.
- MSI reboot-initiated `1641` is forbidden.
- `/norestart` is mandatory for MSI silent operations.
- Verbose MSI logs must prove `REBOOT=ReallySuppress`; the broader reboot semantics remain owned by accepted 03.20.

## Inherited boundaries

- Current-user NSIS must never create or control `VSN-Agent`.
- Per-machine NSIS/MSI service identity and health remain the accepted 03.11 contract.
- Running-resource coordination remains 03.19; 03.21 stops the service before uninstall so this task measures silent command-line behavior, not live-resource conflict policy.
- Repair/idempotence remains 03.16 and cleanup/user-data preservation remains 03.17. No destructive-data switch is allowed.
- No signing, provenance/SBOM, updater/recovery, firewall/hosts/resolver/trust-store, product runtime, service identity or ACL changes are authorized initially.
