# PKG-03 03.11 Research — VSN Agent Windows service lifecycle

Reviewed: 2026-08-28
Canonical base: `4f33813bec4254107e6027e98b2a4a8878b9198b`
Linear: `ABD-86`
Change required: **true**

## Current-source findings

- PKG-03 03.10 canonically installs the accepted Agent payload at `bin/vsn-agent.exe` for NSIS and MSI/WiX; 03.11 must reuse that owned payload and must not introduce a second Agent executable owner.
- The current Agent already exposes the production Windows service contract: service name `VSN-Agent`, display name `VSN Agent`, SCM dispatch through `--service-run`, automatic start, `NT AUTHORITY\LocalService`, STOP handling, and `service install/start/stop/status/uninstall` commands.
- The accepted PKG-02 02.13 service lifecycle already established the `VSN-*` service namespace and bounded Windows service state transitions. 03.11 is installer integration/certification, not a second service-management architecture.
- Tauri 2 supports NSIS installer hooks through `bundle.windows.nsis.installerHooks`. `NSIS_HOOK_POSTINSTALL` runs after files/registry/shortcuts are installed; `NSIS_HOOK_PREUNINSTALL` runs before files/registry/shortcuts are removed.
- The repository already has a separate `tauri.per-machine.conf.json` with NSIS `installMode=perMachine`, so machine-service behavior can remain isolated from current-user install behavior.
- Tauri 2 supports extending MSI/WiX with `.wxs` fragments through `bundle.windows.wix.fragmentPaths` plus explicit fragment references.
- WiX `ServiceInstall` binds the service executable to the KeyPath of its parent Component. Re-declaring `bin/vsn-agent.exe` in a second service Component would conflict with 03.10 payload ownership and is prohibited.
- Therefore MSI integration must consume the already-installed Agent path without duplicating Agent file ownership. A task-owned WiX fragment/custom-action strategy is permitted only if it compiles with the stock Tauri WiX template and proves install/start/removal on exact-head Windows evidence.
- A full custom NSIS or WiX installer template is not justified by current platform capability and remains prohibited unless an explicit change-control addendum is approved.
- Machine service creation is privileged and belongs only to elevated/per-machine installer paths. A current-user NSIS install must not create, start, stop, replace or remove `VSN-Agent`.

Official references:
- https://v2.tauri.app/distribute/windows-installer/
- https://v2.tauri.app/reference/config/
- https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-utils/src/config.rs
- https://docs.firegiant.com/wix3/xsd/wix/serviceinstall/
- https://docs.firegiant.com/wix3/xsd/wix/servicecontrol/
- https://docs.firegiant.com/wix3/customactions/qtexec/

## Platform delta

Installer integration is required because the Agent service runtime exists and the Agent executable is packaged, but the accepted installers do not yet own the production service lifecycle.

Planned implementation direction:
1. Keep the accepted Agent runtime/service identity byte-unchanged unless exact certification exposes a defect requiring explicit change control.
2. Extend the Windows-only Tauri configuration with a task-owned NSIS hook path and a task-owned WiX fragment path/reference only.
3. NSIS hooks are compile-time gated to `perMachine`; post-install installs/starts the existing Agent service and pre-uninstall stops/removes it before payload deletion.
4. MSI/WiX integration must operate on the already-installed `bin/vsn-agent.exe` and must not duplicate the Agent file in a second WiX Component.
5. Exact-head acceptance must prove service identity/configuration, running health, installed CLI-to-Agent health, stop/start behavior, uninstall removal, and current-user non-registration.
6. No ACL/state-directory, firewall/hosts/DNS/trust, repair, rollback, running-process coordination, reboot, unattended deployment, signing, updater or recovery scope is added.

`change_required=true`
