# SignPath Foundation integration plan for PKG-03 03.22

Status: **PREPARATION ONLY / PROVIDER APPROVAL PENDING**

This document prepares the migration of PKG-03 task `03.22` from the legacy PFX-based production-signing implementation to SignPath Foundation Open Source Code Signing. It does not activate production signing, does not mark `03.22` DONE, and does not unblock `03.23`–`03.25`.

## Current canonical boundary

- PKG-03 remains `21/25 = 84%` with `03.22` as the only dependency-ready implementation task.
- Existing PR #162 remains the authoritative `03.22` implementation lineage until a deliberate provider migration/reconciliation is completed.
- The trusted-main workflow `.github/workflows/pkg03-0322-production-signing-trusted.yml` remains the current production-signing authority on `main`.
- The protected GitHub Environment remains `production-signing`.
- No production-signing private key or certificate file is stored in Git.

## Why SignPath changes the credential model

The legacy trusted workflow expects a PFX payload and PFX password in the protected environment. SignPath Foundation instead keeps the signing key outside the repository and GitHub runner and accepts a GitHub workflow artifact through its trusted-build-system integration.

The migration therefore replaces the PFX credential pair with a SignPath submission token plus provider-issued identifiers. The GitHub Environment remains an approval and authorization boundary.

## Protected Environment contract after approval

### Secret

- `SIGNPATH_API_TOKEN`

### Variables

- `VSN_SIGNING_ENV_GUARD=production-signing-v1`
- `SIGNPATH_ORGANIZATION_ID`
- `SIGNPATH_PROJECT_SLUG`
- `SIGNPATH_SIGNING_POLICY_SLUG`
- `SIGNPATH_ARTIFACT_CONFIGURATION_SLUG`
- `VSN_SIGNING_EXPECTED_SUBJECT`

Do not guess provider-issued values. Populate them only from the approved SignPath organization/project configuration. Never paste `SIGNPATH_API_TOKEN` into Git, issues, PR comments, logs, artifacts, or chat.

The legacy names `VSN_SIGNING_PFX_B64` and `VSN_SIGNING_PFX_PASSWORD` must not be populated with fake/test values. They are retired only after the canonical trusted workflow has been intentionally migrated and re-certified.

## Preparation workflow

The isolated preparation branch contains the authoritative preflight workflow:

`.github/workflows/pkg03-0322-signpath-foundation-preflight.yml`

Supporting files:

- `scripts/ci/pkg03-0322-signpath-verify.ps1`
- `docs/SIGNPATH-0322-ACCEPTANCE-MAPPING.md`
- `docs/CODE-SIGNING-POLICY.md`

The preflight has two purposes:

1. Machine-validate the migration contract without contacting SignPath or reading production secrets.
2. Carry the exact SignPath submission/verification template that will later be reconciled into the trusted-main `03.22` workflow.

The submission job is mechanically hard-disabled using the marker `SIGNPATH_PENDING_APPROVAL`. It must stay disabled until SignPath Foundation approves the project and provider-issued identifiers are verified.

The SignPath GitHub action is pinned to the exact commit currently referenced by the public `v2` line at preparation time:

`c92b958760219087e01f8d67a1669ed57afe2627`

At activation time, re-resolve the official `v2` release/tag, review upstream changes, and deliberately freeze the accepted action commit before production use. SignPath's current GitHub integration requires the artifact to be stored as a GitHub Actions artifact before submission and, for OSS projects, requires all jobs leading up to the signing request to run on GitHub-hosted agents.

## Required production flow after SignPath approval

The final integrated flow must remain within one trusted GitHub workflow lineage so SignPath can verify artifact origin:

1. Trusted `main` authorizes one governed `03.22` request bound to the exact source SHA, base SHA and source ref.
2. A GitHub-hosted Windows runner checks out that exact source.
3. The runner builds the four exact unsigned candidates without production credentials:
   - `nsis-current-user.exe`
   - `nsis-per-machine.exe`
   - `vsn-platform.msi`
   - `VSN Dev Platform.exe`
4. The workflow proves all four inputs are `NotSigned`, records SHA-256 hashes and package identity, and uploads the exact unsigned handoff as a GitHub Actions artifact.
5. A protected `production-signing` job receives the approved Environment after independent reviewer approval.
6. The SignPath GitHub action submits that exact GitHub artifact ID using the protected `SIGNPATH_API_TOKEN` and approved provider identifiers.
7. SignPath performs the signing request and returns the signed artifact.
8. A GitHub-hosted Windows verifier proves for every returned candidate:
   - Authenticode status `Valid`;
   - signer subject equals the approved `VSN_SIGNING_EXPECTED_SUBJECT` byte-for-byte;
   - timestamp certificate is present;
   - Windows-native SignTool verification succeeds;
   - signed bytes differ from unsigned bytes;
   - package identity metadata is unchanged;
   - exact unsigned/signed SHA-256 values are recorded;
   - deterministic tamper is rejected;
   - no private-key material or signing token leaks into evidence/artifacts.
9. Independent evidence verification checks source/request/artifact/signature/hash continuity.
10. Only then may `03.22` be projected DONE.

## Origin-verification rule

Do not activate a design that manually downloads an unsigned artifact from an unrelated workflow and then submits it from an unbound ad-hoc signing workflow. The production implementation must preserve SignPath trusted-build origin verification: the unsigned build/upload and SignPath submission must be part of the approved trusted GitHub workflow chain on GitHub-hosted runners.

The current standalone preflight workflow is therefore **a migration template, not production signing authority**. Its submit job remains disabled and must be reconciled into `.github/workflows/pkg03-0322-production-signing-trusted.yml` after provider approval.

## GitHub Environment controls

`production-signing` must remain fail-closed:

- allowed deployment branch: `main` only;
- independent required reviewer;
- prevent self-review enabled;
- no production signing provider token at repository level;
- provider token stored only as an Environment secret;
- non-secret provider identifiers stored as Environment variables;
- `VSN_SIGNING_ENV_GUARD` fixed to `production-signing-v1`.

## SignPath Foundation approval boundary

While the application is pending:

- do not enable the SignPath submission job;
- do not create a production signing request;
- do not mark `03.22` DONE;
- do not merge PR #162 as completed production signing;
- do not activate `03.23`, `03.24`, `03.25`, or PKG-04;
- do not publish an artifact as a stable/certified VSN 1.0 release.

If SignPath asks for an existing downloadable release, any interim artifact must be explicitly labeled beta/pre-release and unsigned unless separately signed. Such an artifact is not acceptance evidence for `03.22`.

## Activation checklist

After SignPath approval:

1. Record the approval without exposing secret material.
2. Confirm the SignPath GitHub App/trusted build system is authorized for `Vertex-Systems-Network/vsn-local-server` if SignPath requires it.
3. Create the SignPath API token with the minimum submitter permission required for the approved project/signing policy.
4. Add the token directly to `production-signing` as `SIGNPATH_API_TOKEN`.
5. Add the provider-issued organization/project/policy/artifact-configuration identifiers as Environment variables.
6. Determine the exact SignPath Foundation Authenticode publisher subject from a provider-approved signing configuration or signed test artifact and set `VSN_SIGNING_EXPECTED_SUBJECT` exactly.
7. Re-resolve and review the SignPath `v2` GitHub action commit and freeze it.
8. Reconcile the SignPath submission/verification stage into the trusted-main `03.22` workflow while preserving the current exact-source request authorization and unsigned-build provenance checks.
9. Run a fresh readiness audit against the exact current PR #162 lineage/base or its deliberately reconciled successor.
10. Create the governed one-file production request only when every environment and provider gate passes.
11. Execute signing and independent evidence verification.
12. Project `03.22` DONE only after genuine production evidence passes.
13. Then reconcile/activate `03.23`; continue `03.24`, then `03.25` in frozen dependency order.

## Non-goals

This preparation does not:

- change product/runtime/installer code;
- change canonical package/task progress;
- grant SignPath submission authority to pull-request code;
- make a beta artifact a stable release;
- authorize downstream package implementation;
- weaken existing exact-source, package-identity, tamper-negative, or secret-leak acceptance requirements.
