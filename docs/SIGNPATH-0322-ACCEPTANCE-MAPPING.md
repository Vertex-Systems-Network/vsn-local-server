# PKG-03 03.22 — SignPath acceptance mapping

Status: **PREPARATION ONLY / SIGNPATH FOUNDATION APPROVAL PENDING**

This document maps the frozen provider-neutral `03.22` acceptance contract to the proposed SignPath Foundation implementation. It does not change the frozen task requirements, does not create production authority, and does not mark `03.22` DONE.

## Frozen acceptance remains authoritative

The frozen `03.22` task requires a secret-safe Windows Authenticode path with exact-head candidate binding, SHA-256 signing, RFC 3161 timestamping, expected publisher verification, Windows-native verification, tamper rejection, package-identity preservation, secret-leak checks and zero tracked drift.

The frozen plan explicitly allows a **secret-external signing provider/command**. It does not require a PFX file or locally imported private key.

## Requirement-to-evidence mapping

| Frozen requirement | SignPath implementation/evidence |
| --- | --- |
| 1. Build accepted exact-head Windows candidate set | Reuse the trusted-main exact-source authorization and GitHub-hosted `build-unsigned` job. Candidate set stays `nsis-current-user.exe`, `nsis-per-machine.exe`, `vsn-platform.msi`, `VSN Dev Platform.exe`. |
| 2. Record unsigned SHA-256 before signing | Reuse `unsigned-provenance.json`; the SignPath verifier recomputes every unsigned SHA-256 and requires an exact match. |
| 3. Invoke a secret-external signing provider without persisting credentials | Submit the exact GitHub Actions artifact to SignPath Foundation. The production signing private key remains in provider custody and is never imported into the GitHub runner. Only the minimum SignPath submission token is supplied through the protected Environment. |
| 4. Require SHA-256 Authenticode digest | Freeze the approved SignPath signing policy to SHA-256 Authenticode and require Windows-native verification of the returned files. Provider policy/configuration must be reviewed after approval before production use. |
| 5. Require RFC 3161 timestamp with SHA-256 | Freeze the approved SignPath signing policy accordingly and require a timestamp certificate on every returned production candidate. Final provider configuration/evidence must establish SHA-256 timestamping before acceptance. |
| 6. Windows-native verification | `scripts/ci/pkg03-0322-signpath-verify.ps1` requires `Get-AuthenticodeSignature` status `Valid` plus `signtool verify /pa /all /v` success for every candidate. |
| 7. Bind expected publisher/public certificate metadata | Protected variable `VSN_SIGNING_EXPECTED_SUBJECT` is compared byte-for-byte with each returned signer certificate subject; public thumbprints/subjects may be recorded in evidence. |
| 8. Record signed SHA-256 and preserve unsigned/signed distinction | The verifier records both hashes independently and requires signed bytes to differ from unsigned bytes. Source/request lineage comes from trusted unsigned provenance. |
| 9. Deterministic tamper rejection | The verifier mutates a deterministic byte in the signed Desktop executable and requires both Authenticode and native SignTool verification to reject it. |
| 10. Preserve package identity/install semantics | The verifier compares MSI ProductCode/UpgradeCode/ProductName/ProductVersion and executable ProductName/ProductVersion/CompanyName/FileDescription before and after signing. No product/runtime source mutation is authorized by the migration. |
| 11. Scan for forbidden key/PFX/password/token material | The SignPath path carries no PFX/private key. Evidence rejects private-key-bearing file suffixes and PEM private-key markers. The SignPath API token is Environment-scoped and must never be copied into evidence/logs/artifacts. |
| 12. Prove zero tracked drift | Existing trusted build plus migration/preflight validation retain tracked-clean checks. Final integrated trusted workflow must repeat tracked-drift verification after provider integration. |

## PFX-specific cleanup hardening

The current trusted PFX implementation contains additional CNG/CAPI persisted-key cleanup proof because that architecture imports a production private key into a Windows runner certificate store.

That cleanup mechanism is **architecture-specific**, not a frozen requirement of the provider-neutral `03.22` plan. Under SignPath Foundation, the production signing private key remains in SignPath custody and is not materialized as a GitHub-runner CNG/CAPI key container. Therefore the migration must replace the old cleanup evidence with explicit provider-custody/runner-non-exposure evidence rather than pretending to clean up a key that never entered the runner.

The final SignPath evidence should state at minimum:

- `production_credentials_external=true`;
- `signing_private_key_runner_exposure=false`;
- `private_key_material_recorded=false`;
- `protected_environment=production-signing`;
- approved SignPath organization/project/signing-policy/artifact-configuration identifiers;
- exact source/request/unsigned-artifact binding;
- exact publisher identity and Windows-native signature results.

If the approved SignPath integration later introduces any client private-key material on the runner, this assumption must be re-audited before production activation.

## Provider values still intentionally unresolved

Until SignPath Foundation approves the project, do not guess or freeze:

- `SIGNPATH_API_TOKEN`;
- `SIGNPATH_ORGANIZATION_ID`;
- `SIGNPATH_PROJECT_SLUG`;
- `SIGNPATH_SIGNING_POLICY_SLUG`;
- `SIGNPATH_ARTIFACT_CONFIGURATION_SLUG`;
- `VSN_SIGNING_EXPECTED_SUBJECT`.

The action commit currently used in preparation is also re-verified at activation time before being accepted into trusted production authority.

## Completion rule

SignPath approval by itself is not `03.22` acceptance. `03.22` becomes DONE only after a genuine production SignPath request returns the exact governed candidate set and every frozen acceptance requirement above passes with independently verified evidence.
