# PKG-03 03.04 — Install Scope and Elevation Contract v1

Status: frozen task execution contract.
Canonical base: `8f2919923005ba29b1475bd646a3f6953100ca9e`.
Parent package plan: `.ai/plans/pkg03-windows-installer-v1.md`.
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`.
Task: `03.04`.
Linear: `ABD-79`.

## Acceptance criteria

1. The default NSIS installer scope is explicitly `currentUser`, preserving non-elevated least-privilege installation.
2. A separate task-owned Tauri config overlay defines the per-machine NSIS variant with `installMode=perMachine`.
3. The accepted VSN contract does not use NSIS `both`, because Tauri documents that it requires Administrator privileges even when current-user is selected.
4. The stock Tauri MSI/WiX package family is classified as per-machine/elevated; 03.04 does not introduce a custom WiX template to force per-user MSI.
5. Existing product identity, publisher, downgrade policy and WiX UpgradeCode remain unchanged from accepted 03.03.
6. The locked repository-local Tauri CLI on GitHub-hosted Windows accepts both scoped configuration paths without executing either installer.
7. Evidence is bound to exact source SHA, config/toolchain digests and resolved scope values.
8. No installer execution, UAC prompt, privileged mutation, service registration, ACL mutation, payload ownership, signing or updater behavior occurs.
9. Pre-evidence state is canonical `3/25`; accepted state increments only 03.04 to `4/25`, leaving 03.05 READY and 03.06 BLOCKED.

## Frozen scope decisions

- Default interactive NSIS package: current-user / non-elevated.
- Explicit machine NSIS variant: per-machine / Administrator boundary.
- NSIS `both`: prohibited by this contract.
- MSI/WiX: enterprise per-machine package family under the stock Tauri template.
- Default current-user install metadata scope: HKCU.
- Per-machine install metadata scope: HKLM.
- Default current-user location class: user-writable LocalAppData.
- Per-machine location class: Program Files.
- Elevation is requested only by the explicit machine-wide package path; 03.04 does not execute that prompt.

## Planned repository realization

After planning-gate acceptance:
- `apps/desktop/src-tauri/tauri.conf.json` receives explicit NSIS `currentUser`;
- `apps/desktop/src-tauri/tauri.per-machine.conf.json` receives the per-machine NSIS overlay;
- task validator/workflow prove both configs are accepted and no forbidden `both` mode is present.

No custom installer template is authorized here. Detailed install/uninstall behavior remains 03.06–03.08.

## Evidence

Required workflow: `PKG-03 03.04 Install Scope + Elevation`.
Required validator: `python scripts/ci/validate-pkg03-0304.py`.
Required governance: AI Planning Governance, Repository Governance, PKG-03 Acceptance Sequence.

## Exit state

After genuine 03.04 evidence and state reconciliation:
- `done=4`, `percent=16.0`, `complete=false`;
- `03.04=DONE`;
- `03.05=READY`;
- deterministic resume cursor `03.05`;
- `03.06` remains BLOCKED until 03.05 is DONE.
