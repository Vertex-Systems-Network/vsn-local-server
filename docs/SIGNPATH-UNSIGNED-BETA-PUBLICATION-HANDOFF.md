# SignPath unsigned beta publication handoff

Issue: #216
Candidate PR: #217
Public policy PR: #218

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

1. PR #217 has an independent approval for the exact candidate build/evidence.
2. PR #218 has an independent approval and the public `Code signing policy` plus privacy notice are present on canonical `main`.
3. The candidate artifact is still unexpired and its GitHub artifact metadata still matches the exact run, head, name and digest above.
4. Downloaded candidate contents independently match `candidate-manifest.json` and `SHA256SUMS.txt`.
5. All four Windows assets remain explicitly named `UNSIGNED` and are represented as beta/pre-release artifacts only.
6. The release target is the exact product source SHA above, not the later documentation/workflow commit used to publish it.
7. A separately reviewed publication authorization removes the mechanical `SIGNPATH_UNSIGNED_BETA_PUBLICATION_PENDING_APPROVAL` disable from the publication job.
8. The `release-publication` GitHub Environment is configured with the intended independent approval policy before enabling the write-capable job.

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
- links to the repository Code signing policy and privacy notice;
- genuine Windows uninstallation is supported according to `docs/PKG03-INSTALLER-UNINSTALL-CLEANUP-PRESERVATION-V1.md`.

## Dormant workflow

`.github/workflows/signpath-unsigned-beta-publication-preflight.yml` validates this exact handoff and downloads/verifies the immutable candidate artifact using read-only permissions.

Its publication job is intentionally guarded by all of the following:

- `workflow_dispatch` only;
- canonical repository + `main` only;
- `release-publication` Environment;
- job-scoped `contents: write` only;
- an explicit boolean workflow input; and
- a final mechanical `false` condition marked `SIGNPATH_UNSIGNED_BETA_PUBLICATION_PENDING_APPROVAL`.

Therefore merging the handoff alone cannot publish a release. Enabling publication requires a later reviewed change.

## Nonclaims

This handoff does not:

- claim SignPath Foundation approval;
- create or expose `SIGNPATH_API_TOKEN`;
- create `.ai/requests/pkg03-0322-production-signing.v1.json`;
- mark `03.22` DONE;
- unblock `03.23`, `03.24`, or `03.25`;
- make the beta stable/certified/production-signed; or
- authorize downstream package activation.
