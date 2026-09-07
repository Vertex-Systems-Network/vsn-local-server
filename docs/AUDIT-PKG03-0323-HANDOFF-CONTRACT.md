# Audit-only PKG-03 03.22 → 03.23 handoff contract

Status: **AUDIT-ONLY / SYNTHETIC / NON-PRODUCT / NO IMPLEMENTATION AUTHORITY**

This probe mechanically exercises the future signed-artifact continuity rules between PKG-03 task `03.22` and blocked task `03.23` without consuming real production evidence and without activating 03.23.

## Why this exists

The 03.23 planning contract requires future SBOM/provenance subjects to equal the exact production-signed package bytes accepted by 03.22. A rebuilt installer, unsigned candidate, test-signed candidate, stale source, different upstream evidence record or changed package identity must fail closed.

Because 03.22 is not canonically DONE, real 03.23 implementation remains forbidden. This audit therefore uses only synthetic in-memory fixtures and intentionally exposes no command-line input for real signing evidence.

## Synthetic invariants exercised

The positive fixture requires:

- upstream task `03.22` with `production_accepted=true`;
- one exact 40-hex source SHA;
- one 64-hex accepted 03.22 evidence digest;
- exactly four governed Windows subjects:
  - `nsis-current-user.exe`;
  - `nsis-per-machine.exe`;
  - `vsn-platform.msi`;
  - `VSN Dev Platform.exe`;
- distinct unsigned and signed SHA-256 values for each subject;
- Windows Authenticode status `Valid`;
- timestamp present;
- package identity preserved;
- non-empty signer subject;
- downstream 03.23 handoff subject SHA-256 exactly equal to the corresponding accepted **signed** SHA-256;
- exact source and upstream-evidence lineage continuity;
- no token/password/PFX/private-key fields in handoff evidence.

The negative matrix must reject:

1. non-production 03.22 evidence;
2. rebuilt/different signed bytes;
3. unsigned hash substitution;
4. source lineage mismatch;
5. upstream evidence digest mismatch;
6. duplicate subject;
7. missing subject;
8. invalid Authenticode result;
9. missing timestamp;
10. package identity drift;
11. secret-bearing handoff fields.

## Boundary

This audit does **not**:

- mark 03.22 or 03.23 DONE;
- create a real SBOM, attestation or PKG-05 handoff;
- consume PR #162 production artifacts;
- consume SignPath credentials or provider output;
- mutate installer/product/runtime/updater code;
- change canonical tracker/master status;
- authorize merge of PR #163;
- unblock 03.24 or 03.25.

When 03.22 is genuinely accepted, PR #163 must still be reconciled onto fresh `main`, the actual 03.22 evidence schema must be bound, standards/tool versions must be re-checked and a real 03.23 implementation validator must be frozen under the normal task authority process.
