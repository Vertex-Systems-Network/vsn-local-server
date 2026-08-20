# Installer / signing boundary — VSN 0.17

`packaging/windows/build-msi.ps1`, `packaging/linux/build-deb.sh` and `packaging/macos/build-pkg.sh` build **unsigned** runtime packages containing the Agent, CLI and out-of-process updater helper. The Windows MSI installs the Agent as the `VSNAgent` LocalService service and controls start/stop/remove through the installer.

Signing keys are deliberately not accepted by the build scripts or committed configuration. `packaging/windows/sign-msi.ps1` signs an already-built MSI by certificate thumbprint using Windows SDK SignTool and verifies the result. `packaging/macos/sign-notarize.sh` signs an already-built `.pkg`; notarization is optional and uses an existing keychain profile.

The GitHub release gate uploads unsigned OS packages. A production publication pipeline should run signing/notarization in a separately permissioned environment, verify signatures, publish checksums and only then update the signed VSN update manifest/channel.
