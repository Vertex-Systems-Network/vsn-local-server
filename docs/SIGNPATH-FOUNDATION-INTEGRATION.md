# SignPath Foundation Integration Plan for PKG-03 03.22

Status: PREPARATION ONLY — SignPath Foundation application pending review.

This document does not activate production signing, does not change canonical PKG-03 progress, and does not mark task 03.22 DONE.

## Objective

Replace the legacy PFX-oriented production-signing assumption with a SignPath Foundation-compatible release-signing path if and only if the VSN Local Server application is accepted.

The intended trust model is:

1. GitHub-hosted Windows build produces exact unsigned release artifacts.
2. The unsigned artifact is uploaded as an immutable GitHub Actions artifact.
3. A governed SignPath signing request references that artifact.
4. Required SignPath approval is performed manually by an authorized maintainer.
5. SignPath returns the signed artifact.
6. GitHub-hosted Windows verification proves signature validity, signer identity, artifact/package identity and tamper rejection.
7. Evidence is bound to the exact source SHA, build run, unsigned artifact digest, signing request and final signed artifact digest.

## Current canonical boundary

At the time this preparation branch was created:

- canonical `main` = `79812eafdead24de88d8b3fafd19f1bfc0e1435c`;
- PKG-03 = `21/25 = 84%`;
- active/ready task = `03.22`;
- 03.23, 03.24 and 03.25 remain dependency-blocked;
- existing PR #162 remains the authoritative mutable 03.22 implementation lineage until a deliberate reconciliation/migration decision is made.

This preparation branch must not be used to project completion.

## Required SignPath data after approval

Do not invent these values before SignPath provisions the project. Record only the exact provider-issued values:

- SignPath organization identifier;
- SignPath project slug/identifier;
- artifact configuration slug/identifier;
- signing policy slug/identifier;
- API token or GitHub integration credential required by the approved integration method;
- signer/publisher identity observed on an accepted SignPath-signed test artifact.

Secrets must be stored only in the protected `production-signing` GitHub Environment or another equivalently protected trusted-main boundary. Non-secret stable identifiers may be environment variables.

## Legacy PFX variables

The following legacy names must not be populated with fake/test values merely to satisfy 03.22:

- `VSN_SIGNING_PFX_B64`
- `VSN_SIGNING_PFX_PASSWORD`

If SignPath becomes the accepted production provider, the active 03.22 implementation must remove or formally supersede these PFX requirements rather than leave dead production authority in parallel.

`VSN_SIGNING_ENV_GUARD=production-signing-v1` may remain as the repository-side protected-environment guard if the reconciled workflow retains that contract.

## Protected GitHub Environment

The trusted production-signing boundary must continue to require:

- environment name `production-signing`;
- deployment restricted to `main` only;
- independent required reviewer;
- Prevent self-review enabled;
- administrator bypass disabled where supported;
- signing credentials scoped to the Environment, not ordinary repository secrets;
- no production signing credentials exposed to mutable pull-request workflow code.

## Proposed CI stages after SignPath approval

### A. Authorization

Fail closed unless:

- execution is from trusted `main`;
- the governed request binds the exact approved 03.22 source/ref/base;
- all required protected-environment controls are active;
- SignPath project configuration values are present;
- no legacy PFX authority can silently bypass the SignPath path.

### B. Unsigned build

On GitHub-hosted Windows:

- checkout exact governed source;
- use the repository-pinned Rust/Node/npm/Tauri toolchain and lockfiles;
- build the exact Windows release candidates;
- calculate SHA-256 for every candidate;
- record package identity/version metadata;
- verify candidates are unsigned before submission;
- upload an immutable unsigned artifact with source/run metadata.

### C. SignPath request

Submit the GitHub Actions artifact using the SignPath-supported GitHub integration and bind:

- organization;
- project;
- artifact configuration;
- signing policy;
- exact unsigned artifact identity.

The request must fail closed on missing or mismatched provider configuration.

### D. Manual approval

The designated SignPath signing approver must inspect release intent and artifact/source identity before approving the request.

A test/development approval or test certificate cannot satisfy production acceptance.

### E. Signed-artifact verification

On GitHub-hosted Windows, independently verify:

- Authenticode/signature status is valid;
- signer/publisher identity equals the provider-approved expected identity;
- final signed artifact digest is recorded;
- NSIS/MSI/application identity remains correct;
- each signed artifact is traceable to its unsigned digest and signing request;
- deterministic tampering causes verification failure;
- no secret/private-key material appears in logs/artifacts;
- tracked repository drift is zero.

### F. Evidence and state projection

Only after all acceptance gates pass may the accepted 03.22 evidence record include:

- exact source SHA/ref/base SHA;
- unsigned build workflow/run/job;
- unsigned artifact ID/digest and per-file digests;
- SignPath organization/project/artifact-config/signing-policy identifiers;
- SignPath signing request identifier;
- approval evidence/identity where safely available;
- signed artifact ID/digest and per-file digests;
- signer/publisher identity;
- Windows-native verification results;
- tamper-negative results;
- source/package identity preservation;
- secret-leak and tracked-drift results.

Then, and only then, may 03.22 be projected DONE and downstream 03.23 reconciliation begin.

## SignPath Foundation readiness items

Before or during provider review, keep these project properties explicit and truthful:

- repository is publicly accessible;
- license is GNU GPL v3;
- public Code Signing Policy exists;
- release/signing responsibilities are documented;
- signing/release administrator accounts use MFA;
- no proprietary/private signing key is stored in the repository;
- distributed artifacts, once releases begin, are traceable to public source revisions.

The repository currently must not claim VSN 1.0 stable/certified before the existing package acceptance sequence completes.

## Release availability note

If SignPath requests evidence of an existing downloadable release while PKG-03 is incomplete, do not publish a falsely stable release. Any interim downloadable artifact must be clearly labeled as an unsigned beta/pre-release/development artifact and must not be treated as PKG-03 03.22 acceptance evidence.

## Migration decision after approval

When SignPath approves VSN Local Server:

1. capture the exact provider-issued integration identifiers without secrets in Git;
2. re-audit current `main` and PR #162;
3. decide whether to reconcile #162 or replace its signing-specific implementation with a clean SignPath task branch;
4. preserve all already-accepted 03.22 secret-free/source/package tests that remain provider-independent;
5. replace PFX-specific production authority with the SignPath path;
6. run exact-head governance and secret-free regression before any production request;
7. execute one genuine SignPath production signing acceptance sequence;
8. independently verify evidence before projecting DONE.

No downstream package is activated by this preparation document.
