# PKG-03 03.20 — Reboot-required, no-restart and pending-reboot semantics plan v1

Status: frozen task plan
Task: `03.20`
Linear: `ABD-95`
Canonical base: `73de463594650cb2ebc407957cbb010e8a0e4be8`
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`

## Objective

Certify the generated Windows installer candidates under a deterministic pending-reboot condition and prove native no-restart semantics without triggering a reboot, corrupting accepted lifecycle state, or crossing into 03.21 silent deployment.

## Acceptance

Exact-head Windows evidence must:
1. build/hash current-user NSIS, per-machine NSIS and MSI/WiX from the exact source head;
2. bind one boot-session identity before any tested installer activity;
3. snapshot `PendingFileRenameOperations` exactly, append one test-only runner-temp rename pair, and prove the pre-existing + probe prefix remains intact during certification;
4. reuse the accepted 03.19 visible running Desktop/CLI/Agent lifecycle while the pending signal exists;
5. prove all inherited 03.19 no-pre-kill, safe-block/coordination, protected-state and retry rules remain valid;
6. exercise exact MSI install and uninstall with `/norestart` and verbose logs;
7. prove `/norestart` reached Windows Installer as `REBOOT=ReallySuppress`;
8. prove the synthetic pending-file-rename condition is observable as `MsiSystemRebootPending=1`;
9. accept only MSI success codes `0` or `3010`; fail on `1641` or any unexpected native exit;
10. prove the boot-session identity never changes;
11. treat `MsiSystemRebootPending` only as the documented pending-file-rename signal, not as a universal reboot detector;
12. restore the exact original pending-registry state and clean all test-only files even after failure;
13. finish with zero tracked repository drift and no product/config/template/service/ACL/signing/provenance/updater mutation.

## Boundaries

- 03.20 owns reboot-required, no-restart and pending-reboot semantics only.
- `/qn` is permitted solely as the MSI reboot-control probe; it does not certify 03.21 unattended/silent deployment.
- 03.22 Authenticode and production signing credentials remain independent.
- 03.23 provenance/SBOM, PKG-04 updater/recovery and PKG-05 release are out of scope.
- Initial implementation is certification-first. Product/installer mutation requires evidence-bound change control after a genuine exact-head failure.

## Governance sequence

Frozen task bundle -> authority validator -> exact-head Windows certification -> independent artifact verification -> same-PR accepted-state projection -> exact final-head governance + task certification -> guarded merge -> canonical main re-read.

## Evidence artifact

`pkg03-0320-reboot-semantics`
