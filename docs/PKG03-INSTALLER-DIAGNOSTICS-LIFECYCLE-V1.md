# PKG-03 03.15 Installer Diagnostics Lifecycle Contract v1

Canonical base: `4f5e8ab30f030e758c52c4ca4ac08f73f896247a`
Task: `03.15`
Linear: `ABD-90`

## Required outcomes

### NSIS
- Current-user successful setup: visible UI observed, exit code `0`.
- Per-machine successful setup: visible/elevated UI observed, exit code `0`.
- Setup cancellation: cancel via genuine visible UI before commit, exit code `1`, no accepted install residue.
- Generated uninstaller cancellation exit code is explicitly **not certified** because stock NSIS self-copy semantics prevent reliable parent-process propagation.
- Operator evidence consists of process outcome plus normalized UI observations/actions; no native persistent NSIS log is claimed.

### MSI/WiX
- Successful visible install: `msiexec /i <msi> /L*V <log>` returns `0`.
- Successful visible uninstall: `msiexec /x <ProductCode> /L*V <log>` returns `0`.
- Visible pre-commit install cancellation returns `1602` (`ERROR_INSTALL_USEREXIT`).
- Every MSI operation retains a non-empty verbose log and SHA-256 in exact-head evidence.

## Clean-state rules

Before each cancellation case, accepted install root/registration state must be absent. Cancellation must occur before commit and the same state must remain absent after process exit. Successful install cases must be genuinely uninstalled before the next case.

## Exact-head evidence

Evidence binds:
- source commit and workflow run/job;
- package path/hash/size;
- runner/toolchain;
- operation, visible UI observation, user action, process exit code;
- MSI log path/size/SHA-256;
- post-operation clean/installed assertions;
- zero tracked drift.

## Explicit nonclaims

No silent/passive deployment, repair, rollback/recovery, reboot semantics, Restart Manager coordination, signing, updater/recovery, custom installer templates/hooks, special NSIS build, or production logging subsystem is certified by 03.15.
