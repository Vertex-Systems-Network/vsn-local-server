# PKG-03 03.07 — NSIS Per-Machine Elevated Install/Uninstall v1

Status: frozen task execution contract.
Canonical base: `a5c7781767d9bf5870f66085de7f3c247b943b87`.
Parent package plan: `.ai/plans/pkg03-windows-installer-v1.md`.
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`.
Task: `03.07`.
Linear: `ABD-82`.

## Acceptance criteria

1. Exact-head Windows certification builds the VSN NSIS setup with the accepted `perMachine` overlay and locked package/toolchain inputs.
2. The default source config remains `currentUser`; 03.07 must not mutate product config merely to realize the per-machine variant.
3. The setup/uninstaller are launched with empty argument vectors; `/S`, `/P`, `/UPDATE` and explicit `RunAs` are forbidden.
4. The hosted runner's Administrator/elevated/high-integrity state and UAC-disabled environment are recorded before installer execution.
5. Evidence proves a visible installer window was observed and normal enabled GUI controls were progressed.
6. The installer process token is elevated/high-integrity.
7. The clean install root resolves to `%ProgramFiles%\VSN Dev Platform`, not LocalAppData, and `VSN Dev Platform.exe` plus `uninstall.exe` exist there.
8. Per-machine Add/Remove Programs metadata exists under HKLM with `DisplayName=VSN Dev Platform`, `DisplayVersion=0.38.1`, `Publisher=Vertex Systems Network`, Program Files `InstallLocation`, and an `UninstallString` targeting the installed uninstaller.
9. No corresponding VSN uninstall registration is created in HKCU by this per-machine lifecycle.
10. `bin/vsn.exe` and `bin/vsn-agent.exe` remain absent; 03.10 retains real CLI/Agent placement authority.
11. The installed uninstaller is launched with no arguments, its process token is elevated/high-integrity, and evidence proves its visible GUI path was observed and progressed.
12. The clean interactive uninstall removes the HKLM package registration and installed Desktop executable/uninstaller; the current-user LocalAppData root remains absent.
13. Evidence explicitly records `uac_prompt_observed=false` and `uac_prompt_certified=false`; GitHub-hosted Windows UAC-disabled behavior is a declared environment limitation, not hidden.
14. Certification proves tracked repository drift is zero and emits exact-source machine-readable evidence including setup hash, privilege state, install root, registry assertions, GUI observations and cleanup assertions.
15. No MSI, shortcut semantics, CLI/Agent placement, service, ACL, signing, updater, silent deployment or downstream lifecycle behavior is claimed.
16. Accepted state advances only 03.07 from canonical `6/25` to `7/25`; 03.08–03.10 remain READY and cursor advances to 03.08.

## Frozen execution shape

### Build

Use the locked GitHub-hosted Windows toolchain and build only NSIS with:
- canonical source config;
- `apps/desktop/src-tauri/tauri.per-machine.conf.json` as an additive Tauri config overlay.

No product file mutation is permitted.

### Pre-install privilege and state snapshot

Record:
- source SHA;
- setup path and SHA-256;
- runner Windows identity, Administrator membership, token elevation and integrity;
- documented UAC-disabled hosted-runner boundary;
- absence of expected Program Files and LocalAppData VSN roots;
- absence of HKLM and HKCU VSN uninstall keys.

### Interactive install

Start setup directly with an empty argument vector. The harness must:
- discover a visible top-level NSIS window;
- record visible/enabled GUI controls and page observations;
- progress normal controls;
- record installer process elevation/high-integrity;
- fail closed if no visible UI or elevated token is observed.

### Installed-state checks

The expected per-machine root is `%ProgramFiles%\VSN Dev Platform`. Verify exact HKLM metadata and absence of corresponding HKCU registration.

### Interactive uninstall

Start the installed `uninstall.exe` directly with an empty argument vector. Record elevated/high-integrity token state, observe and progress the visible uninstall GUI, and avoid any unrelated/destructive user-data choice.

### Cleanup

Verify HKLM registration and clean executable payload are removed and no LocalAppData current-user install root was created.

## Hosted-runner UAC boundary

GitHub documents that Windows hosted runners are administrators with UAC disabled. 03.07 therefore certifies actual elevated Administrator-token execution and per-machine lifecycle semantics, but **does not** certify the appearance or user interaction of a UAC consent/credential prompt.

## Planned repository realization after planning gates

- `scripts/ci/validate-pkg03-0307.py`
- `scripts/ci/pkg03-0307-interactive-nsis.ps1`
- `.github/workflows/pkg03-0307-nsis-machine-install.yml`

No product file mutation is planned.

## Exit state

After genuine 03.07 evidence and reconciliation:
- `done=7`, `percent=28.0`, `complete=false`;
- `03.07=DONE`;
- `03.08–03.10=READY`;
- deterministic cursor `03.08`.
