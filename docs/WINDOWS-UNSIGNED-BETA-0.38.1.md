# VSN Dev Platform 0.38.1 — Windows unsigned beta guide

Status: **unsigned beta / pre-release preparation**

This guide describes the reviewed Windows artifact forms prepared for the VSN Dev Platform `0.38.1` beta eligibility lane. VSN remains in active development. VSN 1.0 is not certified or stable, Windows production Authenticode signing is still pending, and this guide does not mark PKG-03 task `03.22` complete.

## What this beta is

VSN Local Server is a local development/server platform. The current project covers local project/runtime/database management, HTTPS/domain workflows, Desktop and CLI control surfaces, and remote-management/provider capabilities that the operator explicitly configures or invokes.

Feature maturity is governed by the repository execution status. In particular, updater/recovery, cross-platform release, final security certification, production resilience and stable-1.0 work remain later packages and must not be inferred from this beta label.

## Reviewed Windows artifact forms

The governed `0.38.1` candidate produces these Windows forms from exact canonical source commit `79812eafdead24de88d8b3fafd19f1bfc0e1435c`:

1. `VSN-Dev-Platform-0.38.1-current-user-UNSIGNED.exe` — NSIS installer configured for current-user installation.
2. `VSN-Dev-Platform-0.38.1-per-machine-UNSIGNED.exe` — NSIS installer configured for per-machine installation.
3. `VSN-Dev-Platform-0.38.1-UNSIGNED.msi` — MSI installer package.
4. `VSN-Dev-Platform-0.38.1-UNSIGNED.exe` — Desktop release executable produced by the governed build. It is a release binary, not a substitute for the installer lifecycle when an installed application is required.

Use an installer form when you want Windows application registration and the governed install/uninstall lifecycle. Choose current-user versus per-machine deployment according to the intended installation scope and the privileges available to the operator.

## Before installation

This beta is intentionally **not production-signed**. Windows may therefore display the normal warnings associated with unsigned software. A warning must not be bypassed by pretending the beta is signed or certified.

Before running a downloaded candidate:

- confirm that the release is explicitly marked as a GitHub **pre-release** / beta;
- confirm the release identifies source commit `79812eafdead24de88d8b3fafd19f1bfc0e1435c`;
- verify the downloaded file against the release `SHA256SUMS.txt`;
- read the project [Code signing policy](CODE-SIGNING-POLICY.md) and [Privacy notice](PRIVACY.md); and
- do not treat the unsigned beta as PKG-03 `03.22` production-signing evidence.

If the published release metadata or checksum does not match the reviewed candidate, do not use that file as the governed beta.

## Installation

For an installed Windows application, launch one of the reviewed installer forms appropriate to the desired scope and follow the installer prompts.

The accepted installer lifecycle owns the installed VSN application payload and its Windows application registration. Per-machine packages may also own the `VSN-Agent` Windows service registration when that package form installs it.

This guide intentionally does not hardcode an installation directory or command-line switch that is not part of the frozen public beta contract. The installer-selected scope and the accepted package metadata remain authoritative.

## Uninstallation

Remove an installed beta through its genuine Windows registered uninstall / installer lifecycle. For the applicable package form, the accepted uninstall contract requires cleanup of package-owned application payload, shortcuts/application registration, the current-user or machine ARP entry / MSI ProductCode registration, and owned `VSN-Agent` service registration where applicable.

User-created and mutable runtime/configuration/workspace data outside the owned install boundary is preserved unless an explicit ownership rule proves that the product owns that data. The uninstall process must not traverse junction/reparse boundaries into unrelated preserved locations, and it must not unexpectedly mutate protected firewall, hosts, resolver or trust-store state.

The underlying acceptance contract is documented in [`PKG03-INSTALLER-UNINSTALL-CLEANUP-PRESERVATION-V1.md`](PKG03-INSTALLER-UNINSTALL-CLEANUP-PRESERVATION-V1.md).

## Network and privacy behavior

VSN includes user-directed network-capable features. Depending on what the operator configures or invokes, these can include remote SSH/workspace operations, control-plane enrollment, update/download operations and provider integrations.

The project does not claim that the application never communicates with networked systems. See the [Privacy notice](PRIVACY.md) for the current release/privacy boundary.

## Signing status

The selected free OSS production-signing route is SignPath Foundation, but the Foundation application/onboarding is not represented by this guide as approved. The governed `0.38.1` beta candidate was built and reviewed as **NotSigned** input.

A later provider-backed production signature must be independently verified against the project’s exact source/artifact identity and signing policy before any build can satisfy PKG-03 `03.22`.

## Release verification

The candidate release lane binds immutable artifact sizes and SHA-256 values. A separate read-only release verifier is being prepared to reject a public beta page if its tag, source commit, pre-release status, warnings, asset set or digests differ from the reviewed candidate.

This guide does not authorize publication. Public release creation remains a separately reviewed action under issue `#216`.
