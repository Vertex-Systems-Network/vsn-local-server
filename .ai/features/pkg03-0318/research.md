# PKG-03 03.18 Research — Transactional install failure rollback and interrupted-install recovery

Reviewed: 2026-08-30
Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Linear: `ABD-93`
Change required: **false (certification-first)**

## Canonical findings

- Canonical PKG-03 is `15/25 = 60%`; `03.18` is READY because `03.11`, `03.12`, `03.14`, and `03.15` are canonically DONE.
- 03.14 supplies exact owned-file integrity classification; 03.15 supplies deterministic installer exit/logging semantics; 03.11/03.12 freeze service/security-state boundaries. 03.18 owns the first forced-failure and interrupted-install recovery proof.
- Windows Installer is transactional: when an installation fails before commit, rollback actions restore the system toward its pre-install state. Rollback can be disabled by policy/property, so acceptance must prove rollback is actually active rather than assuming it.
- Windows Installer rollback is driven by a rollback script generated during installation. Evidence must therefore bind a genuine failing transaction and post-failure machine state, not just a synthetic nonzero process exit.
- Tauri's generated NSIS installer does not provide a documented transactional rollback guarantee equivalent to MSI. Exact generated NSIS behavior must be exercised under deterministic failure/interruption and fail closed if partial package state remains.
- Failure injection must not depend on live-running VSN product processes; that belongs to 03.19. Preferred probes are pre-created filesystem conflicts/denials inside the selected install root that force package payload placement to fail after setup begins, while preserving a known external sentinel.
- Interrupted-install recovery is distinct from ordinary rollback. The harness must terminate a genuinely active installer only after evidence shows the transaction has started, then prove bounded recovery/cleanup on the next launch without claiming reboot semantics (03.20).
- Any recovery mechanism that would require new product runtime, updater logic, broad recursive cleanup, package identity changes, or installer-template hooks is outside initial authority and requires bounded change control.

Official references:
- https://learn.microsoft.com/en-us/windows/win32/msi/rollback-installation
- https://learn.microsoft.com/en-us/windows/win32/msi/rollback-actions
- https://learn.microsoft.com/en-us/windows/win32/msi/disablerollback
- https://learn.microsoft.com/en-us/windows/win32/msi/rollback-disabled
- https://v2.tauri.app/distribute/windows-installer/

## Frozen failure model

1. **Deterministic failed install**
   - establish clean product state;
   - create an evidence-bound filesystem conflict within the selected install root that does not overlap user data;
   - start the exact package and require a genuine non-success result;
   - prove package-owned files, ARP/ProductCode, shortcuts and service identity did not remain partially registered;
   - prove the external sentinel and protected 03.13 state remain unchanged.
2. **Interrupted install**
   - start an exact candidate on a clean state;
   - wait for a positive transaction-start observation;
   - terminate only installer-owned process(es), never VSN runtime processes;
   - wait for bounded quiescence and record residue;
   - rerun the exact candidate and require deterministic recovery to a valid complete install or a clean fail-closed state;
   - perform accepted ordinary uninstall cleanup afterward.
3. **Format boundary**
   - MSI must demonstrate Windows Installer rollback is enabled and retain verbose failure/recovery logs;
   - NSIS behavior is evidence-driven and may fail the task if generated setup leaves unrecoverable partial state.

No product/config/template/toolchain mutation is authorized by this planning conclusion.
