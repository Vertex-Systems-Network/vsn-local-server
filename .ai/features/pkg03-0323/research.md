# PKG-03 03.23 Research — Installer hashes, SBOM/provenance and PKG-05 handoff

Status: **PLANNING-ONLY / BLOCKED**. This artifact does not activate 03.23 and does not authorize implementation or canonical state projection.

Reviewed: 2026-09-04
Canonical preflight base: `e3fb61581646a475c117cc893566286e397c2108`
Task: `03.23`
Linear: `ABD-98`
Frozen dependency contract: `03.02`, `03.14`, `03.22`

## Live dependency state

- `03.02` — canonically DONE.
- `03.14` — canonically DONE.
- `03.22` — current canonical cursor / only READY implementation task; production signing acceptance is not yet canonical DONE.
- Therefore 03.23 remains BLOCKED and must not consume test/self-signed output as release provenance.

## Current external requirements / standards delta

1. GitHub artifact attestations can establish build provenance for binaries and can attach SBOM attestations. The workflow model requires explicit `id-token: write`, `contents: read` and `attestations: write` permissions and supports verification through GitHub tooling. Public repositories can use artifact attestations on current GitHub plans.
   - https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations
2. GitHub's current SBOM attestation flow supports SPDX or CycloneDX predicates. Production implementation must pin any GitHub Action by immutable commit SHA rather than a floating major tag.
3. GitHub documents downloadable attestation bundles and offline verification with `gh attestation verify` plus a trusted-root snapshot. Because repository attestations may later be deleted and trusted roots rotate over time, 03.23 should preserve the exact verification bundle and its digest as release evidence while still performing live cryptographic identity verification at acceptance time.
   - https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/verify-attestations-offline
   - https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/manage-attestations
4. SPDX lists 3.0 as its current specification. GitHub's current verification examples still show SPDX 2.3 predicate compatibility; therefore 03.23 must not assume newest-spec support without exact-head tool validation.
   - https://spdx.dev/use/specifications/
5. CycloneDX stable current specification is 1.7 (released 2025-10-21). CycloneDX 2.0 is announced for later 2026 and is not the stable baseline at this preflight date. CycloneDX 1.7 JSON is therefore the preferred current SBOM candidate if exact-head generator + GitHub attestation verification prove interoperability at activation time.
   - https://cyclonedx.org/specification/overview/
   - https://cyclonedx.org/news/

## Preflight design decision

03.23 is an evidence/provenance task, not an installer mutation task.

When 03.22 becomes canonically DONE, 03.23 must start from fresh `main` and bind to the **accepted production-signed 03.22 subjects**, not rebuild-and-substitute unsigned or test-signed packages.

Expected subject set inherited from 03.22 acceptance:
- current-user NSIS installer;
- per-machine NSIS installer;
- MSI/WiX installer;
- exact Desktop executable if 03.22 production evidence includes it as a signed candidate.

The activation-time plan should prefer one canonical machine-readable SBOM serialization and one explicit provenance/attestation representation, while retaining deterministic SHA-256 subject hashes in a repository-owned handoff manifest. Selection is provisional until the exact generator/tool versions are validated on the fresh activation base.

## Required provenance properties

The future 03.23 evidence must bind, at minimum:
- exact source commit SHA;
- exact accepted 03.22 workflow/run/job/artifact identity;
- production-signed subject filename, size and SHA-256;
- signer/publisher identity evidence reference from 03.22 without copying secret material;
- timestamp/verification acceptance reference from 03.22;
- SBOM format/version and generator name/version;
- SBOM SHA-256;
- provenance/attestation predicate type and verification result;
- attestation subject digest equal to the exact accepted production-signed subject SHA-256;
- attestation repository/workflow/source identity extracted by verification;
- exact attestation bundle file SHA-256 and GitHub CLI/verifier version;
- trusted-root snapshot SHA-256 when offline-verification evidence is retained, without treating a stale root snapshot as a substitute for activation-time trust validation;
- workflow identity and immutable action/tool versions;
- deterministic PKG-05 handoff manifest digest.

## Portable attestation verification boundary

The final implementation should fail closed unless each attested release subject is independently verified against its exact accepted SHA-256 and expected repository/workflow identity. The evidence package should retain a downloaded attestation bundle for each subject (or one deterministic bundle set covering the exact subjects), compute its SHA-256, and record the predicate type used for verification.

Offline verification is a reproducibility/retention aid, not a weaker alternative to live acceptance. At acceptance time the verifier must validate cryptographic signatures/timestamps and signer identity using current trusted metadata. A retained trusted-root snapshot may support later forensic reproduction, but any later verification should refresh trusted roots when possible so key rotation/revocation is not silently ignored.

## Security / trust boundaries

- No PFX, password, private key, signing token, certificate export, OIDC token or other secret material may be stored in repository files or uploaded evidence.
- Test/self-signed 03.22 output cannot satisfy 03.23 release handoff.
- 03.23 must not change package identity/version/upgrade code, installer payload, service, ACL, network, updater/recovery, or Linux/macOS implementation.
- SBOM/provenance generation must be observational over the accepted candidate and locked dependency/source graph; it must not mutate the candidate bytes.
- Attestation permissions must remain least-privilege and bounded to the attestation-producing job; OIDC tokens themselves must never become evidence artifacts.
- Every generated manifest/SBOM/attestation must be independently parsed/verified before any state projection.

## Change-required decision

`change_required = false` for the frozen PKG-03 task boundary.

The current research does not require a PKG-03 scope change. A fresh activation-time delta check remains mandatory because CycloneDX 2.0 is expected later in 2026 and GitHub attestation/tool behavior may change.
