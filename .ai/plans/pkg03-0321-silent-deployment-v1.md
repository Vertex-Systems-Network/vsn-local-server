# PKG-03 03.21 — Unattended and silent NSIS/MSI deployment contract plan v1

Status: frozen task plan
Task: `03.21`
Linear: `ABD-96`
Canonical base: `3edb4e1dcd2c062e7b2e270cde626c90a2c5459f`
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`

## Objective

Certify that the exact generated Windows installer candidates can be deployed and removed without user interaction through their documented silent command-line interfaces, while preserving the already accepted scope, service, cleanup and reboot contracts.

## Acceptance

Exact-head Windows evidence must:
1. build/hash current-user NSIS, per-machine NSIS and MSI/WiX from the exact source head;
2. run current-user NSIS install with uppercase `/S` and send zero UI/user input;
3. prove current-user root + HKCU registration + Desktop/CLI/Agent payload appear and the machine service remains absent;
4. run the installed current-user uninstaller with `/S`, with zero input, and prove owned install root + HKCU registration are removed;
5. run per-machine NSIS install with uppercase `/S`, with zero input, and prove Program Files + HKLM registration + accepted payload + running `VSN-Agent`;
6. stop `VSN-Agent` through the installed Agent before uninstall so 03.21 does not duplicate 03.19 running-resource acceptance;
7. run the installed per-machine uninstaller with `/S`, with zero input, and prove service/root/HKLM registration removal;
8. run MSI install as `/i <exact-msi> /quiet /norestart /L*V <log>` with zero input and prove exact ProductCode registration, payload and running service;
9. stop the accepted service, then run MSI uninstall as `/x <ProductCode> /quiet /norestart /L*V <log>` and prove service/root/registration removal;
10. reject any installer-family visible titled window observed while a strict silent operation is active;
11. require bounded completion; timeout is a failure and may not be converted into success by UI automation or prompt handling;
12. require NSIS native exit `0`; MSI native exit only `0` or `3010`; forbid `1641`;
13. prove MSI verbose logs contain `REBOOT=ReallySuppress`;
14. finish every lane cleanly, with no cross-lane scope contamination and zero tracked repository drift;
15. make no product/config/template/service-identity/ACL/signing/provenance/updater mutation unless exact-head failure evidence authorizes a minimum AC-mapped change.

## Nonclaims

- `/passive` is not strict silent acceptance because it intentionally displays progress.
- 03.21 does not replace 03.16 repair semantics, 03.17 destructive-cleanup/user-data rules, 03.19 running-resource coordination, or 03.20 reboot semantics.
- 03.21 does not certify Authenticode/signing (03.22) or provenance/SBOM/release handoff (03.23).

## Governance sequence

Frozen task bundle -> authority validator -> exact-head Windows certification -> independent artifact verification -> same-PR accepted-state projection -> exact final-head task-specific + governance checks -> guarded merge -> canonical main re-read.

## Evidence artifact

`pkg03-0321-silent-deployment`
