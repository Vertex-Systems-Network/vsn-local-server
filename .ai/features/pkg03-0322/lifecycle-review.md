# PKG-03 03.22 Lifecycle Review — Windows signing and verification

Reviewed: 2026-08-30
Task: `03.22`
Linear: `ABD-97`

## Lifecycle

| Stage | Required evidence |
| --- | --- |
| unsigned candidate | exact source SHA, package path, size and SHA-256 |
| signing request | provider mode + non-secret key/profile identifier only |
| signed candidate | signed SHA-256 differs from unsigned bytes and file remains parseable |
| Authenticode verify | valid embedded signature, expected publisher, SHA-256 file digest |
| timestamp verify | RFC 3161 timestamp present and SHA-256 timestamp digest |
| negative probe | tampered signed copy must fail verification |
| cleanup | no credential/key material and zero tracked repository drift |

## Credential boundary

Production key/certificate private material is external. Evidence may contain only non-secret provider metadata required to reproduce trust routing, such as provider type, endpoint class, account/profile alias and certificate thumbprint/public subject when policy permits.

Forbidden evidence includes PFX/P12 bytes, private keys, passwords, client secrets, bearer tokens, raw credential environment values and reusable signing URLs/tokens.

## Package boundary

Signing may mutate only the exact package/binary bytes selected by the frozen signing set. It may not change package identity, ProductCode/UpgradeCode semantics, install scope, service identity, ACL policy, owned payload list or runtime source.

Unsigned and signed SHA-256 values must both be retained so provenance can distinguish pre-sign and post-sign bytes. 03.23 owns the later SBOM/provenance handoff.

## Verification boundary

Acceptance requires Windows-native signature verification plus a deterministic tamper-negative check. A command returning success without a valid expected signer/timestamp is insufficient.

Production acceptance requires a timestamp. A no-secret development lane may validate integration with an ephemeral test certificate only if it is clearly classified as non-production and cannot satisfy the final production-signing acceptance claim.

## Nonclaims

03.22 does not certify updater signatures, release publishing, SmartScreen reputation, PKG-05 distribution, or production secret provisioning.
