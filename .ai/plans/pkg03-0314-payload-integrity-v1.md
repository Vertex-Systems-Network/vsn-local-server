# PKG-03 03.14 — Installed Payload Integrity Detection Plan v1

Status: frozen task plan candidate
Canonical base: `0eaa4abb7c5e817334f13672952a5901fbbc8fa9`
Linear: `ABD-89`

## Goal

Prove that every installer-owned executable can be deterministically compared against exact-head expected SHA-256 evidence, and that missing/tampered conditions are detected without implementing or claiming repair.

## Acceptance sequence

1. Validate frozen 03.14 planning authority, digests, dependency state and unchanged locked product inputs.
2. Build exact-head current-user NSIS, per-machine NSIS and MSI/WiX packages.
3. Capture exact expected SHA-256 values for Desktop, CLI and Agent from build/staging outputs before install.
4. For each installer lifecycle, install through the already-accepted visible path and require all three installed files to equal the expected hashes.
5. Run the certification-only integrity detector and require `MATCH` for all three.
6. Current-user NSIS perturbation matrix:
   - delete each owned executable one at a time and require `MISSING`;
   - restore the exact verified bytes;
   - tamper each owned executable one at a time and require `HASH_MISMATCH`;
   - restore exact verified bytes and require `MATCH`.
7. Per-machine NSIS and MSI/WiX perturbation matrix:
   - keep Agent read-only to avoid unauthorized service/running-process coordination;
   - perform missing/tamper probes for Desktop and CLI;
   - restore exact verified bytes and require all three owned files to return to `MATCH`.
8. Do not invoke `msiexec /f`, reinstall, self-healing, service re-registration, or any product repair path.
9. Uninstall through the already-accepted lifecycle and require the owned executable set to be absent.
10. Write exact evidence containing source SHA, package hashes, expected file hashes, every detector observation, perturbation/restoration proof, uninstall cleanup, toolchain metadata and zero tracked repository drift.
11. Upload one evidence artifact and independently verify `evidence.json.sha256`.
12. Only after genuine exact-head evidence and required governance are green may 03.14 be accepted.

## Failure classification

- expected/installed baseline mismatch -> product/package defect;
- detector misclassification -> certification defect;
- service/file-lock preventing an unauthorized perturbation -> scope boundary, not permission to add service coordination;
- GitHub runner/toolchain failure -> runner infrastructure;
- dependency/tracker/manifest mismatch -> governance state.

## Explicit deferrals

- actual reinstall/repair execution -> 03.16;
- Agent running-process/service coordination -> 03.19 (03.11 remains service lifecycle authority);
- rollback -> 03.18;
- reboot -> 03.20;
- silent deployment -> 03.21.
