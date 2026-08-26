# PKG-03 03.03 — Lifecycle Review

Task: `03.03`
Linear: `ABD-78`
Canonical base: `9d33682f7c0cc30080792493c8f760f3fd120759`

## Lifecycle position

03.03 is Wave 1 / identity lane and depends only on canonical `03.01=DONE`. It is independent of 03.02, 03.04 and 03.05 and may execute in parallel under the frozen max-five DAG.

## Entry invariants

- PKG-03 denominator/order remains exactly 25 tasks (`03.01`–`03.25`).
- 03.01 is canonically DONE.
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
3. Install the locked Desktop npm graph on GitHub-hosted Windows.
4. Use the repository-local Tauri CLI to parse the config and inspect the WiX upgrade code.
5. Prove the inspected upgrade code equals the pinned `157f304f-1d1b-55e0-b89c-0610ea27c645` value.
6. Emit exact-source machine-readable evidence with zero installer execution and zero privileged mutation.
7. Only after genuine evidence passes may 03.03 become DONE in canonical state.

## Parallel-state reconciliation

Because 03.02 is an independent Wave 1 sibling, 03.03 acceptance must preserve its live canonical status.

- If 03.02 is still READY when 03.03 is accepted, the deterministic cursor remains 03.02.
- If 03.02 is already DONE when 03.03 is accepted, the deterministic cursor advances to 03.04.
- In both cases, 03.04 and 03.05 remain READY until separately accepted.
- 03.06 remains blocked until all of 03.02–03.05 are canonical DONE.

## Explicit non-actions

No NSIS/MSI install or uninstall, no UAC/elevation choice, no service registration, no payload ownership, no Authenticode signing, no updater configuration, and no privileged Windows mutation.
