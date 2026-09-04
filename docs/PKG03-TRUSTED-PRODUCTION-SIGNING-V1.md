# PKG-03 03.22 — Trusted Production Signing Boundary V1

Status: security hardening for issue #176. This document does **not** mark 03.22 DONE and does not provision production credentials.

## Threat model

The 03.22 implementation PR is mutable until accepted. Production code-signing credentials must therefore never be exposed to workflow steps or helper scripts checked out from that PR head. A same-repository PR can legitimately execute unprivileged build code, but it cannot become the trust anchor for access to the production PFX/private key.

## Architecture

`.github/workflows/pkg03-0322-production-signing-trusted.yml` is merged to `main` before any production-signing request is created.

The workflow has two trust zones:

1. **Unprivileged exact-source build**
   - consumes one governed request file from `main`;
   - verifies the request commit changed exactly that one file;
   - binds `source_base_sha`, the authoritative 03.22 branch ref and exact `source_sha`;
   - requires the candidate to be a descendant of the trusted base;
   - independently requires the exact 11-file 03.22 pre-production diff and no other path;
   - builds the four accepted Windows candidates with locked Node/Rust/Tauri graphs;
   - has no production signing secrets or environment;
   - proves each handoff candidate is unsigned and uploads only unsigned candidates + non-secret provenance.

2. **Privileged production signing**
   - runs only from a `push` to `refs/heads/main` in this repository;
   - references the fixed GitHub Environment `production-signing`;
   - downloads the secret-free unsigned artifact instead of checking out or executing PR-head signing helpers;
   - requires the environment guard `VSN_SIGNING_ENV_GUARD=production-signing-v1`;
   - imports the production PFX only in the signing step;
   - signs with SHA-256 Authenticode + RFC3161/SHA-256 timestamping;
   - requires exact expected publisher identity, Windows-native verification, package-identity equality and tamper-negative rejection;
   - removes the imported certificate, clears the in-memory PFX byte array, scans evidence for forbidden key material and uploads only signed candidates + non-secret evidence.

The third-party artifact actions and Node setup action used by the trusted workflow are pinned to immutable commit SHAs. The build/signing workflow does not use `pull_request_target`.

## Mandatory GitHub Environment configuration

Before creating a production-signing request, repository administrators must create/configure an environment named exactly:

`production-signing`

Required protection:

- deployment branches/tags restricted so **only `main`** can deploy to the environment;
- at least one required reviewer who is independent of the request author;
- **Prevent self-review** enabled;
- administrator bypass disabled where the repository plan exposes that control;
- no ordinary repository-level values with the production-signing names below.

Environment-scoped values:

Secrets:
- `VSN_SIGNING_PFX_B64`
- `VSN_SIGNING_PFX_PASSWORD`

Variables:
- `VSN_SIGNING_EXPECTED_SUBJECT`
- `VSN_SIGNING_TIMESTAMP_URL`
- `VSN_SIGNING_ENV_GUARD` with exact value `production-signing-v1`

The timestamp variable must be an HTTP(S) RFC3161 endpoint. The expected subject must exactly equal the subject encoded in the production certificate.

## Governed request protocol

The trusted workflow is activated by a separate one-file PR that creates/updates:

`.ai/requests/pkg03-0322-production-signing.v1.json`

Example shape:

```json
{
  "schema_version": 1,
  "task_id": "03.22",
  "source_ref": "pkg03/0322-authenticode-signing-reconciled-v4",
  "source_sha": "<exact 40-hex 03.22 head>",
  "source_base_sha": "<exact main SHA immediately before the request PR>"
}
```

Acceptance rules enforced by the trusted workflow:

- the request merge/push changes exactly this request file;
- `source_base_sha` equals the request commit's immediate first parent on `main`;
- the authoritative source ref resolves exactly to `source_sha`;
- `source_sha` is a descendant of `source_base_sha`;
- the candidate diff from that base is exactly the pre-production 11-file 03.22 scope;
- no tracker/status/package-completion projection is present in the signing source.

If `main` advances before the request is merged, reconcile the 03.22 branch to the new main first and regenerate the request. Never reuse a stale source/base pair.

## Production evidence

A successful trusted signing run must produce `pkg03-0322-trusted-production-signing` containing:

- all four production-signed subjects;
- unsigned provenance with source/base/ref/request lineage;
- exact signed/unsigned SHA-256 values;
- public certificate thumbprint and expected publisher identity;
- RFC3161 timestamp presence;
- Windows Authenticode and SignTool verification results;
- unchanged MSI/PE package identity metadata;
- tamper-negative rejection proof;
- trusted workflow commit/run/attempt and runner image identity;
- evidence SHA-256;
- no PFX/private-key/password/token material.

A missing/invalid environment, approval, secret, publisher, timestamp, signature, identity check or tamper-negative result must fail closed.

## Completion boundary

Merging the trusted signer infrastructure does not complete 03.22. The required sequence remains:

1. merge trusted signer infrastructure to `main`;
2. configure and protect `production-signing` in GitHub settings;
3. reconcile PR #162 to the then-current main;
4. merge a one-file production-signing request referencing that exact 03.22 head/base;
5. approve the protected environment deployment;
6. obtain successful trusted production-signing evidence;
7. independently verify the artifact/run identity;
8. only then project 03.22 DONE in the governed task PR and merge with an exact expected head SHA.

03.23 remains blocked until that canonical 03.22 completion exists.
