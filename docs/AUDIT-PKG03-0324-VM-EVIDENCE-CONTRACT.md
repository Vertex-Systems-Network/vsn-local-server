# Audit-only PKG-03 03.23 → 03.24 VM evidence contract

Status: **AUDIT-ONLY / SYNTHETIC / NON-PRODUCT / NO VM EXECUTION / NO IMPLEMENTATION AUTHORITY**

This probe mechanically exercises future PKG-03 `03.24` evidence invariants while `03.24` remains blocked on `03.22 -> 03.23`. It does not run a real Windows VM and does not consume real signed packages, SBOMs, attestations or production evidence.

## Future boundary being tested

A real 03.24 matrix must consume the exact accepted 03.23 handoff and package hashes. Each executable row must prove the package signature before execution, bind a deterministic Windows image and seed, record native outcomes, preserve already-frozen installer semantics, verify cleanup/non-mutation, and distinguish ordinary hosted-runner cases from any scenario requiring persistence across an actual reboot.

## Positive synthetic fixture

The positive fixture requires:

- accepted upstream task `03.23`;
- exact source SHA and exact 03.23 handoff SHA-256 continuity;
- exactly the four governed production package subjects;
- upstream signature + provenance verification for every package;
- all three installer families represented: current-user NSIS, per-machine NSIS, MSI;
- required synthetic scenarios represented:
  - fresh install;
  - repair-needed tamper;
  - preserved-user-data uninstall;
  - running-resource handling;
  - pending-reboot handling;
- deterministic Windows image identity and seed SHA-256 per row;
- exact package SHA-256 continuity per row;
- signature verification before execution;
- command line + native exit code + PASS outcome;
- cleanup verified;
- forbidden firewall/hosts/resolver/trust-store mutation represented as zero;
- any `real_reboot=true` row must use a persistent provider, preserve the same machine identity before/after, and carry distinct pre/post boot markers.

## Negative matrix

The synthetic self-test must reject:

1. unaccepted 03.23 evidence;
2. different 03.23 handoff digest;
3. rebuilt/different package bytes;
4. missing pre-execution signature verification;
5. missing deterministic dirty/fresh seed digest;
6. missing required scenario;
7. a real-reboot claim on an ordinary GitHub-hosted runner;
8. machine identity change across a claimed reboot;
9. no demonstrable boot boundary;
10. cleanup failure;
11. forbidden system mutation;
12. missing installer-family coverage.

## Boundary

This audit does **not**:

- run Windows installers;
- create or restore VM snapshots;
- claim a real reboot occurred;
- mark 03.23 or 03.24 DONE;
- freeze the real activation-time VM provider or matrix;
- mutate product/runtime/installer/signing/provenance code;
- update canonical trackers/status;
- authorize merge of PR #164;
- unblock 03.25.

After 03.23 is genuinely DONE, PR #164 still must be reconciled onto fresh `main`, bind the real accepted handoff/package/SBOM/provenance identities, freeze actual VM images/providers/rows and then execute the real certification-first matrix under task authority.
