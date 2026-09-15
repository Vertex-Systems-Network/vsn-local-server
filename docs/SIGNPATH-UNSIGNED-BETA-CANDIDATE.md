# SignPath unsigned beta/pre-release candidate

Issue: #216

Status: **candidate-only / no publication authority**

This lane exists only to close the SignPath Foundation eligibility gap that the project must already be released in the form that is intended to be signed. It does not authorize production signing, does not complete PKG-03 task `03.22`, and does not declare VSN 1.0 stable or certified.

## Frozen candidate identity

- Product: `VSN Dev Platform`
- Product version: `0.38.1`
- Application identifier: `dev.vsn.platform`
- Exact source branch: `main`
- Exact source commit: `79812eafdead24de88d8b3fafd19f1bfc0e1435c`
- Candidate class: unsigned beta / GitHub pre-release only
- Future signing provider under review: SignPath Foundation

The candidate workflow fails closed if live canonical `main` moves away from the frozen source SHA before the build is executed. A stale candidate must be refreshed through review rather than silently rebuilt from a different source.

## Exact Windows forms

The review candidate builds the same four artifact forms used by the trusted 03.22 unsigned-build contract:

1. current-user NSIS installer;
2. per-machine NSIS installer;
3. MSI installer; and
4. Desktop release executable.

Every candidate must report Windows Authenticode status `NotSigned` before it can be uploaded as review evidence.

## Build authority

`.github/workflows/signpath-unsigned-beta-candidate.yml`:

- runs only on GitHub-hosted Windows;
- has repository `contents: read` only;
- checks out the exact frozen canonical source with persisted Git credentials disabled;
- refuses to proceed if the frozen source is no longer live `main`;
- requires exact Node `22.12.0`, Rust `1.97.1`, committed npm/Cargo lock graphs, and repository-local Tauri CLI `2.11.4`;
- exposes no SignPath API token and no production PFX credentials;
- produces SHA-256 digests and a machine-readable candidate manifest;
- marks `production_signed=false`, `production_accepted=false`, `pkg03_0322_accepted=false`, `stable_1_0=false`, and `publish_authorized=false`; and
- uploads only a GitHub Actions review artifact.

There is intentionally no GitHub Release creation step and no `contents: write` permission in this workflow.

## Public-release gate

A future public GitHub pre-release may be created only after all of the following are true:

1. this exact candidate build succeeds on its final reviewed head;
2. an independent reviewer verifies the candidate manifest, SHA-256 list, unsigned status and source binding;
3. the repository homepage exposes the public **Code signing policy** and privacy notice;
4. the release is explicitly marked as a beta/pre-release, never stable/certified/1.0;
5. release notes state that production Authenticode signing is pending and normal Windows unsigned-software warnings may appear;
6. release notes identify the exact source SHA and immutable artifact digests;
7. the genuine Windows install/uninstall behavior remains backed by the accepted PKG-03 lifecycle contracts; and
8. publication is separately authorized after review.

The public release must not be reused as production 03.22 acceptance evidence.

## Installation and uninstallation

The candidate uses the same Windows installer forms and package identity as the accepted installer line. Uninstallation is governed by `docs/PKG03-INSTALLER-UNINSTALL-CLEANUP-PRESERVATION-V1.md`: package-owned payload/registration is removed through the genuine installer lifecycle while user-created/mutable data outside the owned install boundary is preserved according to the accepted contract.

## Relationship to PKG-03 03.22

PR #162 remains the authoritative 03.22 implementation lineage until a deliberate provider migration/reconciliation is approved. The candidate release lane does not modify #162, does not create `.ai/requests/pkg03-0322-production-signing.v1.json`, does not consume production signing credentials, and does not unblock `03.23`, `03.24`, or `03.25`.

SignPath Foundation approval, if received, is still insufficient by itself for 03.22 acceptance. Production signing must later pass the protected-environment, exact-artifact, publisher, timestamp, Windows-native verification, identity, tamper-negative and secret-leak requirements of the frozen 03.22 contract.
