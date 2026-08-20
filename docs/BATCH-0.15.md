# Batch 0.15.0

Hard-way implementation focus:

1. Fail-closed verified SAML ACS with external XMLDSig verification, issuer/audience/destination/time/InResponseTo validation and explicit external-subject mapping.
2. Shared Passkey ceremony ownership without serializing WebAuthn cryptographic ceremony state.
3. SCIM Users/Groups Bulk, ETag/If-Match and conservative reconciliation.
4. Durable external DB result artifacts with SHA-256 and resumable output chunks.
5. Durable PTY/ConPTY scrollback without unsafe terminal reconstruction.
6. Out-of-process updater apply/rollback transaction after signed-manifest/artifact verification.

Production limitations are tracked in `docs/roadmap-status.json` and `docs/validation-report.md`.
