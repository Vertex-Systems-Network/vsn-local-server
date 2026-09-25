# SignPath unsigned beta publication handoff

Issue: #216
Candidate PR: #217
Public policy PR: #218
Publication handoff PR: #219

Status: **DORMANT / NO PUBLICATION AUTHORITY**

This document freezes the already-built unsigned beta candidate into a reviewable publication handoff. It does not publish a GitHub Release, does not enable production signing, and does not change PKG-03 acceptance state.

## Immutable candidate binding

- Product: `VSN Dev Platform`
- Product version: `0.38.1`
- Proposed GitHub pre-release tag: `v0.38.1-beta.1`
- Release class: **unsigned beta / pre-release**
- Exact product source: `79812eafdead24de88d8b3fafd19f1bfc0e1435c`
- Candidate PR head: `137d509e2c92c8763644d5c122530782848919c9`
- Candidate workflow run: `34157478206`
- Candidate artifact ID: `10031774089`
- Candidate artifact name: `vsn-0.38.1-unsigned-beta-candidate-79812eaf`
- GitHub artifact digest: `sha256:9deee3893f43ea87122f7876bc3b356c10a428206744c34f0f0d8f634d7c51cd`

The candidate workflow already proved the four Windows forms are Authenticode `NotSigned`, bound to canonical `main` source `79812eafdead24de88d8b3fafd19f1bfc0e1435c`, and generated `candidate-manifest.json`, `SHA256SUMS.txt`, and draft release notes.

## Publication preconditions

A public GitHub pre-release may be authorized only after all of these are true:

1. PR #217 has an independent approval for the exact candidate build/evidence and is merged.
2. PR #218 has an independent approval, is merged, and the public `Code signing policy` plus privacy notice are present on canonical `main`.
3. PR #219 has an independent approval and is merged with the read-only preflight, verifier, dormant publisher and exact release handoff.
4. The candidate artifact is still unexpired and its GitHub artifact metadata still matches the exact run, head, name and digest above.
5. Downloaded candidate contents independently match `candidate-manifest.json` and `SHA256SUMS.txt`.
6. All four Windows assets remain explicitly named `UNSIGNED` and are represented as beta/pre-release artifacts only.
7. The release target is the exact product source SHA above, not the later documentation/workflow commit used to publish it.
8. The `release-publication` GitHub Environment is configured with the intended independent approval policy and `VSN_RELEASE_ENV_GUARD=release-publication-v1` before enabling publication.
9. A separate reviewed activation change removes the literal `if: false` from the publisher and replaces it with the intended trusted-main/manual-authorization condition.

## Proposed release identity

- Tag: `v0.38.1-beta.1`
- Title: `VSN Dev Platform 0.38.1 — UNSIGNED Beta 1`
- Target: `79812eafdead24de88d8b3fafd19f1bfc0e1435c`
- GitHub setting: **pre-release=true**, **latest=false**

The release notes must prominently state:

- this is an **UNSIGNED beta/pre-release**;
- Windows may display normal warnings for unsigned software;
- production Authenticode signing through SignPath Foundation is pending;
- VSN 1.0 is not stable/certified;
- this release is not PKG-03 `03.22` acceptance evidence;
- exact source SHA, workflow run and SHA-256 digests;
- links to the repository Code signing policy and privacy notice; and
- genuine Windows uninstallation is supported according to `docs/PKG03-INSTALLER-UNINSTALL-CLEANUP-PRESERVATION-V1.md`.

## Split trust boundary

The publication handoff is intentionally split into two workflows.

### Read-only PR preflight

`.github/workflows/signpath-unsigned-beta-publication-preflight.yml`:

- may run on pull requests and manual dispatch;
- has only `actions: read` and `contents: read`;
- never enters the `release-publication` Environment;
- contains no write-capable release job;
- validates that the separate publisher has no `pull_request` or `pull_request_target` trigger;
- validates that the publisher carries the disable marker and a literal YAML `if: false`;
- validates the verifier’s PowerShell syntax;
- verifies immutable GitHub artifact metadata;
- downloads the exact frozen candidate artifact;
- re-verifies all four Windows files with Windows Authenticode, size and SHA-256 checks; and
- uploads only preflight evidence.

### Dormant write-capable publisher

`.github/workflows/signpath-unsigned-beta-publish.yml`:

- is `workflow_dispatch` only;
- has top-level read-only permissions;
- scopes `contents: write` only to the publication job;
- requires the protected `release-publication` Environment when eventually enabled;
- is currently mechanically disabled by:

```yaml
# SIGNPATH_UNSIGNED_BETA_PUBLICATION_PENDING_APPROVAL
if: false
```

- rechecks merged independent approvals for PRs #217, #218 and #219;
- rechecks public policy/privacy/uninstall documentation on canonical `main`;
- rechecks exact artifact metadata and bytes immediately before publication; and
- uses a transactional draft-release flow so a failure before publication removes the partial draft.

Merging this handoff cannot publish a release. Publication requires a later independently reviewed activation change.

## Security finding that caused the split

An earlier combined workflow attempted to hard-disable its write-capable job with a folded GitHub expression ending in `&& false` plus an inline marker comment. GitHub Actions run `34159527252` showed that this was not a reliable mechanical job skip: the write-capable job entered execution.

The old job still failed closed at its first publication-prerequisite step. Candidate download, byte verification, release creation, asset upload and publish steps were skipped. A fresh GitHub Releases read after that run returned zero releases, and the proposed `v0.38.1-beta.1` tag was absent.

That combined architecture is superseded by the split workflows and literal `if: false` above.

## Nonclaims

This handoff does not:

- claim SignPath Foundation approval;
- create or expose `SIGNPATH_API_TOKEN`;
- create `.ai/requests/pkg03-0322-production-signing.v1.json`;
- mark `03.22` DONE;
- unblock `03.23`, `03.24`, or `03.25`;
- make the beta stable/certified/production-signed; or
- authorize downstream package activation.
