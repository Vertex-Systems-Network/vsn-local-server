# VSN Dev Platform 0.38.1 — UNSIGNED Beta 1

> **UNSIGNED BETA / PRE-RELEASE — NOT STABLE, NOT CERTIFIED, NOT PKG-03 03.22 ACCEPTANCE EVIDENCE**

This Windows beta is published for project evaluation and SignPath Foundation open-source code-signing eligibility. Production Authenticode signing is still pending. Windows may display normal warnings for unsigned software.

## Exact build identity

- Product: `VSN Dev Platform`
- Product version: `0.38.1`
- Release tag: `v0.38.1-beta.1`
- Exact source SHA: `79812eafdead24de88d8b3fafd19f1bfc0e1435c`
- Candidate workflow run: `34157478206`
- Candidate artifact ID: `10031774089`
- Candidate artifact digest: `sha256:9deee3893f43ea87122f7876bc3b356c10a428206744c34f0f0d8f634d7c51cd`

The downloadable assets are the same four Windows forms intended for the future governed signing path:

1. current-user NSIS installer;
2. per-machine NSIS installer;
3. MSI installer; and
4. Desktop release executable.

Every downloadable binary in this beta is intentionally named `UNSIGNED` and was verified as Authenticode `NotSigned` by the candidate build before publication.

## SHA-256 checksums

These hashes are frozen from the exact candidate `SHA256SUMS.txt` produced by workflow run `34157478206`:

```text
d3c4704c91413141dcabe21211059c4bd851c65363a4cea2544678231590650e  VSN-Dev-Platform-0.38.1-current-user-UNSIGNED.exe
812d3c50909826d6dd3764d1100b80397aecb92fbb6084c3decb5933c7d737e9  VSN-Dev-Platform-0.38.1-per-machine-UNSIGNED.exe
defe356afa6eb3538f2081c938347e930f3cd70730f79ba1ac3db54a956dbff0  VSN-Dev-Platform-0.38.1-UNSIGNED.exe
08a6b955174ae34d00be3610ab0245b0408b90eacfaada7435f9da1b3cbb5cac  VSN-Dev-Platform-0.38.1-UNSIGNED.msi
```

Verify the SHA-256 value for the file you download before testing it.

## Signing and privacy

Free code signing is planned through SignPath.io, certificate by SignPath Foundation. This beta is **not yet signed**.

- [Code signing policy](https://github.com/Vertex-Systems-Network/vsn-local-server/blob/main/docs/CODE-SIGNING-POLICY.md)
- [Privacy notice](https://github.com/Vertex-Systems-Network/vsn-local-server/blob/main/docs/PRIVACY.md)
- [Source repository](https://github.com/Vertex-Systems-Network/vsn-local-server)

VSN Local Server includes user-directed network-capable features. See the privacy notice for the project’s network/privacy boundary.

## Installation and uninstallation

The Windows installer line supports genuine installer lifecycle uninstallation. Package-owned payload and registration are removed while user-created or mutable data outside the owned installation boundary is preserved according to the accepted cleanup/preservation contract:

- [Windows uninstall cleanup & preservation contract](https://github.com/Vertex-Systems-Network/vsn-local-server/blob/main/docs/PKG03-INSTALLER-UNINSTALL-CLEANUP-PRESERVATION-V1.md)

## Maturity warning

VSN 1.0 is still under active development and is not stable/certified. PKG-03 remains in progress at task `03.22` until genuine production signing and the remaining PKG-03 gates pass. This beta must not be presented or reused as production-signing acceptance evidence.
