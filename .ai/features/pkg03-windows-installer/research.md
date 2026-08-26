# PKG-03 Windows Installer — Research Baseline

Reviewed: 2026-08-26
Canonical base: `67e9a64da07ae36646cef7f95e343a069b4da5bf`

## Repository baseline

- Tauri config product version is `0.38.1`, product name `VSN Local Server`, identifier `network.vsn.local-server`.
- `bundle.active=true`, `targets="all"`, and tracked Windows icon resources already exist.
- PKG-02 accepted the Agent/CLI/Desktop and Windows service/local-system boundaries; PKG-03 packages those accepted surfaces rather than redesigning them.
- PKG-04 is explicitly Updater & Recovery, so update-feed/self-update behavior is excluded.

## Official platform/tooling baseline

Tauri Windows Installer documentation (updated 2026-06-09) states Windows apps can be distributed as MSI via WiX v3 or NSIS setup executables, MSI must be built on Windows, and NSIS install modes include current-user/per-machine/both.
https://v2.tauri.app/distribute/windows-installer/

Microsoft `msiexec` documents install/uninstall, quiet/passive UI, restart and logging options used by enterprise deployment acceptance.
https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/msiexec

Windows Restart Manager exists specifically to coordinate files/apps/services in use and reduce required reboots during install/servicing.
https://learn.microsoft.com/en-us/windows/win32/rstmgr/about-restart-manager

Microsoft SmartScreen/code-signing guidance reviewed in August 2026 treats unsigned/self-signed public distribution as warning-prone and recommends Microsoft Artifact Signing for non-Store distribution. Production signing identity remains an external secret/trust boundary.
https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation
https://learn.microsoft.com/windows/apps/package-and-deploy/code-signing-options

## Decisions

- Support and certify both current Tauri Windows installer families where practical: NSIS for interactive/current-user/per-machine flows and MSI for enterprise Windows Installer semantics.
- All acceptance builds execute on GitHub-hosted Windows; no cross-compiled installer is acceptance authority.
- Production signing credentials are never committed. Signing work establishes a signer interface and signature-verification gate using non-production/test material until separately authorized production signing exists.
- Firewall/hosts/resolver/trust-store mutation is not an installer side effect.
- Update/recovery behavior is deferred to PKG-04; PKG-03 only produces stable install identity/provenance needed for that handoff.

Market delta: `none` for this package scope as of review date.
