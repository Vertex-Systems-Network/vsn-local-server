# PKG-03 Authenticode Signing and Verification Contract v1

## Purpose

This contract defines the 03.22 trust boundary for Windows code signing. It separates reproducible build output from externally controlled production signing credentials and requires verification evidence strong enough to reject unsigned, wrongly signed or tampered artifacts.

## Production cryptographic baseline

- Authenticode file digest: SHA-256.
- Timestamp: RFC 3161.
- Timestamp digest: SHA-256.
- Signature verification: Windows-native Authenticode verification with expected publisher binding.
- Secret custody: external provider/CI identity only.

## Evidence schema

For each signed artifact retain:
- exact source commit;
- artifact role/path;
- unsigned size and SHA-256;
- signed size and SHA-256;
- signature status;
- signer subject/public identity;
- certificate thumbprint/public serial metadata when allowed;
- digest algorithm;
- timestamp presence/protocol/digest and timestamp authority public identity;
- verification command/tool version;
- tamper-negative result.

Never retain private key bytes, PFX/P12 files, certificate passwords, client secrets, bearer tokens or other reusable credentials.

## Fail-closed rules

Acceptance fails if:
- the artifact is unsigned;
- signature validation fails;
- signer identity differs from the frozen expected publisher;
- SHA-256 is not used;
- production timestamp is absent/invalid;
- a tampered copy still validates;
- secret material appears in Git, logs or evidence;
- signing requires package/runtime identity changes outside approved change control.

## Development/test signing

An ephemeral test certificate may prove command wiring and negative verification behavior. Such evidence must be labeled non-production and cannot satisfy the final 03.22 production-signing gate.

## Ownership

03.22 owns signing integration and signature verification. 03.23 owns installer hashes/SBOM/provenance release handoff. Updater signing and recovery remain outside PKG-03.
