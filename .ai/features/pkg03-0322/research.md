# PKG-03 03.22 Research — Authenticode signing integration

Reviewed: 2026-08-30
Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Linear: `ABD-97`
Change required: **false (integration/certification-first)**

## Canonical findings

- Canonical PKG-03 is `15/25 = 60%`; task 03.22 is READY because 03.02, 03.03 and 03.14 are DONE.
- Microsoft recommends SHA-256 for Authenticode file digests and RFC 3161 timestamps with SHA-256. Timestamping is required for long-term signature validity after certificate expiry.
- Current SignTool requires explicit digest selection and supports verification of embedded Authenticode signatures.
- Tauri 2 supports Windows signing through `bundle.windows.signCommand`, including provider-backed signing such as Azure Artifact Signing or other custom signing tools.
- Production private keys, PFX bytes, client secrets, passwords, tokens and reusable credentials are external trust material. They must never be committed, echoed, uploaded in evidence, or copied into build artifacts.
- 03.22 proves signing integration and verification. It does not choose or provision production PKI ownership, implement updater/recovery, or perform PKG-05 release work.

Official references reviewed:
- Microsoft Learn — Time Stamping Authenticode Signatures
- Microsoft Learn — SignTool
- Tauri 2 — Windows Code Signing

## Frozen conclusion

`change_required=false` means integration/certification-first. The initial implementation may add a task-owned signing adapter/config overlay and CI verification surfaces only if they contain no credential material and preserve package identity/payload semantics.

A provider credential must be injected only at execution time through external secret/provider handles. Missing credentials must fail closed or select an explicitly defined verification-only lane; they must never fall back to an unsigned package while claiming signed acceptance.

## Required signing model

1. Build the exact Windows candidate bytes under the accepted toolchain.
2. Record unsigned SHA-256 before signing.
3. Sign through an external provider/credential handle using SHA-256.
4. Apply an RFC 3161 SHA-256 timestamp for production acceptance.
5. Verify signature status, signer/publisher identity, digest algorithm and timestamp using Windows-native verification.
6. Record signed SHA-256 and prove the only expected byte mutation is the signature-bearing artifact itself.
7. Never persist secret-bearing command lines, environment values or key material in evidence.
8. Keep unsigned/signed artifacts distinctly named and evidence-bound.

## Nonclaims

- no production certificate/private-key provisioning;
- no secret rotation policy;
- no SmartScreen reputation guarantee;
- no updater or update-feed signing;
- no PKG-05 release publishing;
- no weakening of 03.14 payload integrity or package identity contracts.
