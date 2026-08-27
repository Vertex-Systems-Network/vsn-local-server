# PKG-03 03.07 — Lifecycle Review

Task: `03.07`
Linear: `ABD-82`
Canonical base: `a5c7781767d9bf5870f66085de7f3c247b943b87`

## Lifecycle position

03.07 is Wave 2 / scope lane. Canonical prerequisites `03.02–03.05` are DONE, 03.06 is DONE, PKG-03 is `6/25`, and 03.07 is the deterministic cursor. 03.07–03.10 are READY subject to the five-lane cap.

## Entry invariants

- PKG-03 denominator/order remains exactly 25 tasks (`03.01`–`03.25`).
- 03.01–03.06 are canonically DONE.
- 03.07 is READY/IN_PROGRESS and depends on 03.02, 03.03, 03.04 and 03.05.
- default `tauri.conf.json` remains `currentUser`;
- `tauri.per-machine.conf.json` remains exactly `perMachine`;
- product identity remains `VSN Dev Platform` / `0.38.1` / `dev.vsn.platform` / `Vertex Systems Network`;
- owned payload remains exactly `VSN Dev Platform.exe`, `bin/vsn.exe`, `bin/vsn-agent.exe`;
- CLI/Agent placement remains declared-not-yet-packaged and owned by 03.10.

## Planning correction provenance

Exact-head run `33027545330` / job `98372313386` successfully built the stock per-machine NSIS bundle, then stopped before installer launch because the harness required `EnableLUA=0`. No installer/uninstaller executed, no privileged product state was mutated, and the run does not count as 03.07 acceptance.

The correction removes only that fixed environment-value assumption. UAC policy must be measured and recorded, not forced or predicted.

## Post-planning certification authority

After corrected planning gates pass, 03.07 may execute only:
- `scripts/ci/validate-pkg03-0307.py`;
- `scripts/ci/pkg03-0307-interactive-nsis.ps1`;
- `.github/workflows/pkg03-0307-nsis-machine-install.yml`.

The workflow may execute only the exact-head NSIS per-machine installer/uninstaller it builds on the ephemeral GitHub-hosted Windows runner.

## Privilege model

Accepted privilege evidence is:
- observed `EnableLUA` value recorded as environment context;
- current runner token is Administrator, elevated and high-integrity;
- installer and uninstaller process tokens are elevated/high-integrity;
- actual Program Files/HKLM per-machine state is created and later removed.

No fixed `EnableLUA` value is an acceptance requirement. No UAC prompt is claimed or certified. An explicit `RunAs` verb is not authorized.

## Acceptance lifecycle

1. Revalidate parent plan, canonical tracker/master state and prerequisites.
2. Revalidate immutable identity, current-user default config, per-machine overlay and owned-payload digests.
3. Build exact-head NSIS with the accepted per-machine overlay.
4. Capture runner Administrator/elevation/integrity state and measured `EnableLUA`, plus clean Program Files/LocalAppData and HKLM/HKCU pre-state.
5. Launch setup directly with no arguments and no explicit elevation verb.
6. Observe and progress the visible NSIS GUI.
7. Verify Program Files root, expected HKLM metadata, Desktop executable/uninstaller, elevated process token and no HKCU registration.
8. Verify CLI/Agent remain absent.
9. Launch installed `uninstall.exe` directly with no arguments, verify elevated token, observe its visible GUI and complete normal uninstall.
10. Verify HKLM registration and clean installed executable payload disappear; LocalAppData root remains absent.
11. Verify zero tracked repository drift and emit exact-source evidence.
12. Only after genuine evidence passes may 03.07 become DONE.

## State reconciliation

Pre-evidence:
- `done=6`, `percent=24.0`;
- 03.07 IN_PROGRESS/READY;
- cursor 03.07;
- 03.08–03.10 READY.

After genuine 03.07 evidence:
- `done=7`, `percent=28.0`;
- 03.07 DONE;
- 03.08–03.10 remain READY;
- deterministic cursor advances to 03.08.

03.11 and 03.12 remain blocked until 03.10. 03.13 and 03.15 remain blocked until 03.08. 03.14 remains blocked until 03.08 and 03.10.

## Explicit non-actions

No UAC-prompt certification, no fixed hosted-runner UAC-policy claim, no non-admin-account certification, no MSI execution, no custom installer template, no Start Menu/shortcut acceptance, no CLI/Agent placement, no service install, no ACL contract, no repair/reinstall/rollback acceptance, no comprehensive dirty-data preservation, no silent/passive deployment certification, no signing and no updater mutation.
