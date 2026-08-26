# PKG-03 03.06 — NSIS Current-User Interactive Install/Uninstall v1

Status: frozen task execution contract.
Canonical base: `bc8d1403e589fa5f4f9833f6975b5cb53e94e01c`.
Parent package plan: `.ai/plans/pkg03-windows-installer-v1.md`.
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`.
Task: `03.06`.
Linear: `ABD-81`.

## Acceptance criteria

1. Exact-head Windows certification builds the VSN NSIS setup executable using the locked package/toolchain inputs.
2. The installer under test is the canonical `currentUser` NSIS variant; no custom template or install-mode override is introduced by 03.06.
3. Setup is launched with no `/S`, `/P`, `/UPDATE`, restart/app arguments, or elevation verb.
4. Evidence proves a visible installer window was observed and normal enabled GUI controls were progressed; a silent/passive process is not accepted as interactive evidence.
5. The clean install root resolves to `%LOCALAPPDATA%\VSN Dev Platform`, not Program Files, and the main Desktop executable plus `uninstall.exe` exist there.
6. Current-user Add/Remove Programs metadata exists under HKCU with `DisplayName=VSN Dev Platform`, `DisplayVersion=0.38.1`, `Publisher=Vertex Systems Network`, LocalAppData `InstallLocation`, and an `UninstallString` targeting the installed uninstaller.
7. No corresponding VSN uninstall registration is created in HKLM by this current-user lifecycle.
8. `bin/vsn.exe` and `bin/vsn-agent.exe` remain absent; 03.10 retains real CLI/Agent placement authority.
9. The installed uninstaller is launched with no silent/passive/update arguments; evidence proves its visible confirmation/instfiles GUI path was observed and progressed.
10. The clean interactive uninstall removes the HKCU package registration and installed Desktop executable/uninstaller. Comprehensive dirty user-data preservation remains 03.17.
11. Certification proves tracked repository drift is zero and emits exact-source machine-readable evidence including setup hash, install root, registry assertions, GUI observations and cleanup assertions.
12. No per-machine/UAC, MSI, service, ACL, signing, updater, silent deployment or downstream lifecycle behavior is claimed.
13. Accepted state advances only 03.06 from canonical `5/25` to `6/25`; 03.07–03.10 remain READY and cursor advances to 03.07.

## Frozen execution shape

### Build
Use the same deterministic GitHub-hosted Windows source and locked toolchain established by 03.02. Build only the NSIS target required for this task.

### Pre-install snapshot
Record:
- source SHA;
- setup path and SHA-256;
- absence of the expected clean current-user install root;
- HKCU and HKLM VSN uninstall-key state.

### Interactive install
Start setup directly with an empty argument vector. The harness must:
- discover a visible top-level NSIS window for the installer process;
- record visible/enabled GUI button captions/page observations;
- invoke normal buttons such as Next/Install/Finish rather than command-line silent/passive switches;
- fail closed if no visible UI is observed.

### Installed-state checks
Read authoritative runtime state from the filesystem and registry. The expected current-user root is `%LOCALAPPDATA%\VSN Dev Platform`.

### Interactive uninstall
Start the installed `uninstall.exe` directly with an empty argument vector. The harness must observe and progress the normal uninstall GUI without opting into destructive app-data deletion.

### Cleanup
On the clean hosted runner, verify the package's HKCU uninstall registration and clean executable payload are gone. Do not generalize this clean-root result into 03.17's dirty user-data preservation contract.

## Planned repository realization after planning gates

- `scripts/ci/validate-pkg03-0306.py`
- `scripts/ci/pkg03-0306-interactive-nsis.ps1`
- `.github/workflows/pkg03-0306-nsis-user-install.yml`

No product file mutation is planned.

## Exit state

After genuine 03.06 evidence and reconciliation:
- `done=6`, `percent=24.0`, `complete=false`;
- `03.06=DONE`;
- `03.07–03.10=READY`;
- deterministic cursor `03.07`.
