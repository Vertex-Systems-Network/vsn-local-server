# PKG-03 03.03 — Lifecycle Review

Task: `03.03`
Linear: `ABD-78`
Canonical base: `d1d3e6997878aa16b8d4ad05f094754b5b1699b2`

## Lifecycle position

03.03 is Wave 1 / identity lane and depends only on canonical `03.01=DONE`. At this base `03.02=DONE`; 03.04 and 03.05 remain READY. 03.03 may proceed without consuming any downstream dependency authority.

## Entry invariants

- PKG-03 denominator/order remains exactly 25 tasks (`03.01`–`03.25`).
- 03.01 and 03.02 are canonically DONE.
- 03.03 is READY or IN_PROGRESS.
- `productName`, `version`, and `identifier` still match the 03.01 architecture authority.
- no other active task consumes the identity lane.
- no 03.06+ dependency is claimed from this branch.

## Mutation boundary

Authorized product mutation is limited to package identity/upgrade metadata in `apps/desktop/src-tauri/tauri.conf.json`:
- publisher `Vertex Systems Network`;
- downgrade prevention;
- stable WiX upgrade code `157f304f-1d1b-55e0-b89c-0610ea27c645`.

Task-local governance, plan, manifest, validator, Windows workflow and identity contract documentation are also authorized.

## Acceptance lifecycle

1. Validate frozen parent-plan digest and task bundle.
2. Validate exact repository identity/version inputs.
3. Validate canonical pre-state `03.02=DONE`, `03.03=READY`, PKG-03 `2/25`.
4. Install the locked Desktop npm graph on GitHub-hosted Windows.
5. Use the repository-local Tauri CLI to parse the config and inspect the WiX upgrade code.
6. Prove the inspected upgrade code equals the pinned `157f304f-1d1b-55e0-b89c-0610ea27c645` value.
7. Emit exact-source machine-readable evidence with zero installer execution and zero privileged mutation.
8. Only after genuine evidence passes may 03.03 become DONE in canonical state.

## State reconciliation

Before evidence, the canonical shape is `2/25`, 03.02 DONE, 03.03–03.05 READY, cursor 03.03.
After genuine 03.03 evidence, the accepted shape is `3/25`, 03.03 DONE, 03.04–03.05 READY, cursor 03.04.
03.06 remains blocked until all of 03.02–03.05 are canonical DONE.

## Explicit non-actions

No NSIS/MSI install or uninstall, no UAC/elevation choice, no service registration, no payload ownership, no Authenticode signing, no updater configuration, and no privileged Windows mutation.
