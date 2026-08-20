# Secure Update Design (design-only in P2)

The updater is intentionally not implemented yet because an unsigned updater would expand the attack surface before release signing infrastructure exists.

Required design:

1. Offline root signing key kept separate from build systems.
2. Release metadata signed by a delegated online release key.
3. Every Agent/Desktop artifact has SHA-256 digest and platform/architecture metadata.
4. Agent verifies metadata signature and artifact digest before installation.
5. Rollback protection prevents silently installing a lower security version unless an explicit recovery policy permits it.
6. Failed update leaves the previous executable recoverable.
7. Windows binaries are Authenticode-signed in addition to VSN release metadata signatures.
8. Update action and result are audited.

No updater code should be enabled until these verification paths have automated tests.
