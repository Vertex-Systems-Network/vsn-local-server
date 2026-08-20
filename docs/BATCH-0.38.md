# VSN 0.38 — PKG-01 Linux Core Certification

0.38 starts **PKG-01**, the first certification chunk. The package is not complete until all six Linux Core controls are genuine valid PASS for one exact candidate.

## PKG-01 controls

1. `rust-linux`
2. `desktop-build`
3. `dashboard-build`
4. `deb-install-uninstall`
5. `updater-linux`
6. `rustsec-audit`

## One-command completion path

```bash
python scripts/pkg01-linux-core.py all --allow-network
python scripts/pkg01-finalize.py
```

The bootstrap step pins Rust to the repository toolchain, requires/generates `Cargo.lock` and both frontend `package-lock.json` files, installs dependencies from those locks, and requires `cargo-audit`. Any generated lockfile changes the source fingerprint and forces a candidate refreeze before certification execution.

The executor then runs the real six-control Linux pack, creates and verifies an import-ready result ZIP, imports it into the candidate-bound evidence journal, verifies the journal, runs the release gate, and refuses completion unless all six controls remain valid PASS.

Current container cannot complete PKG-01 because external DNS/network access is unavailable and Rust/cargo-audit/lockfiles are not present. This is recorded as BLOCKED, not PASS.

## Preserved release infrastructure

The existing **P30 Runner-Pack Execution** model remains active. 0.38 preserves candidate-bound result handoff/resume, evidence governance, `quarantine`, `supersede`, and disaster-recovery `checkpoint` operations while adding PKG-01 execution. Evidence from **different source candidates** is rejected; every result continues to carry a `candidate_id`.

This is also the continuation of the **P30 handoff/resume sprint**: a frozen source handoff can run on an equipped Linux runner and return a candidate-bound result bundle for evidence import.

P0–P29 remain 30/30 source-closed. With zero genuine P30 PASS controls the exact overall state remains **98.9032%** (rounded headline 99%); P30 remains the only release-certification phase.
