# Audit-only PKG-03 03.25 final evidence contract

Status: **AUDIT-ONLY / SYNTHETIC / NON-PRODUCT / NO FINAL-GATE AUTHORITY**

This probe models the future machine-readable final evidence index required by PKG-03 task `03.25`. It does not consume real production evidence, run final regressions, project task state, complete PKG-03 or activate PKG-04.

## Synthetic contract

A positive fixture requires:

- every frozen dependency `03.02` through `03.24` present exactly once and `DONE` with a machine evidence digest;
- one exact final source SHA shared by 03.22 production signing, 03.23 provenance and 03.24 VM evidence;
- 03.22 `production_accepted=true`;
- 03.23 accepted provenance/handoff;
- 03.24 accepted matrix with all rows PASS;
- if 03.24 claims a real reboot, same-machine proof must be present;
- exact four-package SHA-256 subject-byte equality across 03.22, 03.23 and 03.24;
- final regression PASS;
- independent verification PASS;
- zero tracked drift;
- secret-leak scan PASS;
- deterministic non-secret PKG-04 handoff digest;
- `pkg04_activated=false`;
- `pkg03_complete_projected=false` because package completion is a separate post-03.25 state-only projection.

## Negative self-test matrix

The audit rejects:

1. missing dependency evidence;
2. dependency not DONE;
3. non-production 03.22 signing;
4. 03.23 source mismatch;
5. rebuilt/substituted package subject bytes;
6. 03.24 VM failure;
7. reboot claim without same-machine proof;
8. final regression failure;
9. secret-leak failure;
10. premature PKG-04 activation;
11. premature PKG-03 COMPLETE projection.

## Boundary

This branch does not authorize the 03.25 workflow described by PR #165. After 03.24 is genuinely DONE, PR #165 must still be reconciled onto fresh `main`, bind real upstream evidence identities, freeze the real regression set and PKG-04 non-secret handoff schema, then execute final certification. Only after 03.25 itself is canonically DONE may a separate state-only projection mark PKG-03 COMPLETE and PKG-04 next.
