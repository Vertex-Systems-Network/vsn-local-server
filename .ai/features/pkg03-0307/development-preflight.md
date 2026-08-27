# PKG-03 03.07 — Development Preflight

Task: `03.07`
Linear: `ABD-82`
Canonical base: `a5c7781767d9bf5870f66085de7f3c247b943b87`
Parent plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`

## Authority check

- Parent PKG-03 plan is frozen.
- 03.01–03.06 are DONE on canonical main.
- PKG-03 is `6/25 = 24%`.
- 03.07 is Wave 2 / scope lane, dependency-ready and the deterministic cursor.
- 03.08–03.10 are also READY but grant no authority to this branch.
- Linear ABD-82 is In Progress.
- No self-expansion into 03.08–03.25 or PKG-04 is permitted.

## Locked product inputs

- `apps/desktop/src-tauri/tauri.conf.json`
  - SHA-256 `172cf6110e58a15442bcf97e9db6a8bdbeb6cbfd2f631d91a3031603ed474180`
  - `productName = VSN Dev Platform`
  - `mainBinaryName = VSN Dev Platform`
  - `version = 0.38.1`
  - `identifier = dev.vsn.platform`
  - `publisher = Vertex Systems Network`
  - default NSIS `installMode = currentUser`
- `apps/desktop/src-tauri/tauri.per-machine.conf.json`
  - SHA-256 `48fd4eb22ffe99a884ce5f4770de83e29ad919650d7c254b5d180fca3add7429`
  - overlay NSIS `installMode = perMachine`
- `installer/windows/owned-payload.v1.json`
  - SHA-256 `5292a1ec1ae0a48d76f80258c9e00ba17b1466dab5c5e0cdda60caf4658dabd1`
  - owned paths remain exactly:
    - `VSN Dev Platform.exe`
    - `bin/vsn.exe`
    - `bin/vsn-agent.exe`
- CLI/Agent real installer placement remains 03.10.

## Runtime correction

Run `33027545330` / job `98372313386` is not acceptance evidence. The exact per-machine bundle built, but the harness stopped before installer launch because a fixed `EnableLUA=0` assumption did not match the current hosted image. The corrected contract measures `EnableLUA` and does not prescribe it.

## Required exact-head gates

Corrected planning head:
- AI Planning Governance
- Repository Governance
- PKG-03 Acceptance Sequence

Final implementation/evidence head:
- AI Planning Governance
- Repository Governance
- PKG-03 Acceptance Sequence
- PKG-03 03.07 NSIS Per-Machine Lifecycle

## Privilege evidence rule

The workflow must:
- measure runner Administrator/elevated/high-integrity state;
- measure and record `EnableLUA` without requiring a fixed value;
- measure installer/uninstaller process elevation/high-integrity;
- measure Program Files/HKLM lifecycle;
- keep `uac_prompt_observed=false` and `uac_prompt_certified=false`;
- use no `RunAs` verb.

## Stop conditions

Stop rather than widen scope if:
- canonical main or parent-plan bytes drift before mutation;
- any 03.02–03.05 prerequisite is no longer DONE;
- identity/currentUser-default/perMachine-overlay/ownership contracts drift;
- Program Files/HKLM state cannot be proven from the stock per-machine overlay;
- runner, installer or uninstaller token is not elevated/high-integrity;
- successful evidence requires a custom NSIS template or product change;
- evidence requires MSI, CLI/Agent placement, service registration, ACL mutation, signing secrets, updater behavior or broader cleanup semantics;
- certification would need to claim a UAC prompt that was not actually observed.
