# Personal Windows install (unsigned build)

This guide installs VSN Dev Platform on your own Windows machine without a code-signing certificate. Signing only removes the "Unknown publisher" warning; the app works the same without it. Signing is deferred until public release (see `.ai/changes/PERSONAL-USE-UNSIGNED-DEFERRAL-V1.md`).

> Only install builds you produced from this repository's own workflow. An unsigned installer from anywhere else could be tampered with.

## 1. Build the installer

1. Open the repository on GitHub → **Actions**.
2. Select **Personal Windows Build (unsigned)** (`.github/workflows/personal-windows-build.yml`).
3. Click **Run workflow** on `main`, then wait for it to finish (about 20–40 minutes on `windows-2025`).
4. On the run page, download the artifact **`vsn-windows-unsigned-<commit>`** and unzip it.

The artifact contains:

- an NSIS installer (`*-setup.exe`), recommended for a single user;
- an MSI installer (`*.msi`), for per-machine installs;
- `SHA256SUMS.txt` with the SHA-256 hash of each installer.
- `sbom.cdx.json`, a CycloneDX list of every Rust crate and Desktop npm package in the build, and `provenance.json` with the source commit, workflow run, toolchain and file hashes. Both are marked pre-implementation for 03.23 and say the build is unsigned.

This workflow skips the historical PKG-03 acceptance validators, so it is for personal builds only and produces no certification evidence.

## 2. Verify the download (recommended)

In PowerShell, compare the hash with the value in `SHA256SUMS.txt`:

```powershell
Get-FileHash .\<installer-file> -Algorithm SHA256
```

Install only if the hashes match.

## 3. Install

1. Double-click the installer.
2. Windows SmartScreen shows "Windows protected your PC". Click **More info**, then **Run anyway**.
3. Follow the installer. The MSI and per-machine NSIS modes ask for administrator approval; the per-user NSIS mode does not.

If the file is blocked as downloaded from the internet, right-click it → **Properties** → tick **Unblock** → **OK**, or run `Unblock-File .\<installer-file>`.

## 4. Update or uninstall

- **Update:** build a newer version and run its installer over the existing install. The installer repairs/upgrades in place.
- **Uninstall:** Settings → Apps → Installed apps → **VSN Dev Platform** → Uninstall. User data is preserved unless you choose to remove it.

## Before sharing with others

Do not distribute unsigned builds publicly. Public release still requires PKG-03 tasks 03.22 (Authenticode signing) through 03.25 to be accepted with genuine evidence.
