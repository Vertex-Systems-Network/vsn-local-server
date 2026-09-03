# PKG-03 03.21 Silent Deployment Contract v1

Task `03.21` certifies the generated Windows installer candidates as automation-safe through their documented strict-silent interfaces.

## Supported strict-silent commands

- Current-user NSIS install: `<current-user-setup.exe> /S`
- Current-user NSIS uninstall: `<installed-uninstall.exe> /S`
- Per-machine NSIS install: `<per-machine-setup.exe> /S`
- Per-machine NSIS uninstall: `<installed-uninstall.exe> /S`
- MSI install: `msiexec.exe /i <exact-msi> /quiet /norestart /L*V <log>`
- MSI uninstall: `msiexec.exe /x <ProductCode> /quiet /norestart /L*V <log>`

`/S` is case-sensitive. `/quiet` and `/qn` are equivalent no-UI Windows Installer display modes; the certification uses `/quiet` as the public operator form and verifies the resulting no-UI/reboot-suppression evidence.

## Automation properties

A compliant operation:
- needs no keyboard, mouse, UIAutomation, stdin or prompt answer;
- reaches its required state and process completion inside the frozen timeout;
- exposes no installer-family visible titled window during strict silent execution;
- returns only the task-accepted native code;
- leaves scope, package registration, payload and service semantics coherent;
- never initiates a reboot;
- does not opt into destructive user-data removal;
- leaves no tracked repository drift.

## Exit codes

- NSIS: `0` only.
- MSI: `0` or `3010`.
- MSI `1641`: always failure because it means restart was initiated.

## Ownership

03.21 owns silent command-line deployment only. Repair/reinstall (03.16), cleanup/data preservation (03.17), running-process coordination (03.19), reboot semantics (03.20), signing (03.22), provenance (03.23), updater/recovery (PKG-04), and cross-platform release (PKG-05) retain their existing ownership.
