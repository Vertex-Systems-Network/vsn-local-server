# PKG-03 03.08 — Lifecycle Review

Task: `03.08`
Linear: `ABD-83`
Canonical base: `0ac71c6392c19ad070a9ec442323c46f3c0e08b9`

## Lifecycle position

03.08 is Wave 2 / enterprise lane. Canonical prerequisites `03.02–03.05` are DONE, 03.06 and 03.07 are DONE, PKG-03 is `7/25`, and 03.08 is the deterministic cursor. 03.08–03.10 are READY subject to the five-lane cap.

## Entry invariants

- PKG-03 denominator/order remains exactly 25 tasks (`03.01`–`03.25`).
- 03.01–03.07 are canonically DONE.
- 03.08 is READY/IN_PROGRESS and depends on 03.02, 03.03, 03.04 and 03.05.
- product identity remains `VSN Dev Platform` / `0.38.1` / `dev.vsn.platform` / `Vertex Systems Network`;
- WiX UpgradeCode remains `157f304f-1d1b-55e0-b89c-0610ea27c645`;
- `allowDowngrades=false`;
- MSI scope remains stock WiX per-machine;
- owned payload remains exactly `VSN Dev Platform.exe`, `bin/vsn.exe`, `bin/vsn-agent.exe`;
- CLI/Agent placement remains declared-not-yet-packaged and owned by 03.10.

## Post-planning certification authority

After planning gates pass, 03.08 may add/modify only:
- `scripts/ci/validate-pkg03-0308.py`;
- `scripts/ci/pkg03-0308-interactive-msi.ps1`;
- `.github/workflows/pkg03-0308-msi-enterprise.yml`.

The workflow may execute only the exact-head MSI package it builds on the ephemeral GitHub-hosted Windows runner. Planning itself may not execute MSI or mutate product state.

## MSI identity model

- Stable source-controlled identity is the 03.03 contract, including UpgradeCode.
- ProductCode is generated for the concrete built MSI and must be extracted from that exact package.
- ARP proof is bound to `HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\{ProductCode}`.
- A blanket HKCU-non-mutation assertion is forbidden because the stock Tauri WiX template contains HKCU vendor bookkeeping. 03.08 distinguishes that bookkeeping from the ProductCode-keyed per-machine ARP registration.

## Acceptance lifecycle

1. Revalidate parent plan, canonical tracker/master state and prerequisites.
2. Revalidate immutable identity, UpgradeCode, Tauri config and owned-payload digests.
3. Build the exact-head MSI target on `windows-2025`.
4. Extract exact MSI ProductCode/name/version/manufacturer and record package SHA-256.
5. Capture runner Administrator/elevation/high-integrity state and clean Program Files/HKLM ProductCode pre-state.
6. Launch normal `msiexec /i <msi>` with full/default UI and no silent/passive UI switches.
7. Observe/progress visible Windows Installer UI.
8. Verify Program Files Desktop payload and exact ProductCode-keyed HKLM ARP metadata.
9. Verify CLI/Agent remain absent.
10. Launch normal visible uninstall for the same ProductCode/package without silent/passive switches.
11. Verify the exact ARP ProductCode key and installed Desktop executable disappear.
12. Verify zero tracked repository drift and emit exact-source evidence.
13. Only after genuine evidence passes may 03.08 become DONE.

## State reconciliation

Pre-evidence:
- `done=7`, `percent=28.0`;
- 03.08 IN_PROGRESS/READY;
- cursor 03.08;
- 03.09–03.10 READY.

After genuine 03.08 evidence:
- `done=8`, `percent=32.0`;
- 03.08 DONE;
- 03.09 and 03.10 remain READY;
- 03.13 and 03.15 become READY because 03.06–03.08 are then DONE;
- deterministic cursor advances to 03.09;
- ready set becomes exactly `03.09`, `03.10`, `03.13`, `03.15`.

03.11 and 03.12 remain blocked until 03.10. 03.14 remains blocked until 03.10. No later wave is unlocked by 03.08 alone.

## Explicit non-actions

No custom WiX template, no Tauri product mutation, no silent/passive deployment certification, no shortcut acceptance, no CLI/Agent placement, no service install, no ACL contract, no repair/reinstall/rollback acceptance, no signing and no updater mutation.
