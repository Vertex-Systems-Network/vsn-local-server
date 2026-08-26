# PKG-03 03.04 — Lifecycle Review

Task: `03.04`
Linear: `ABD-79`
Canonical base: `8f2919923005ba29b1475bd646a3f6953100ca9e`

## Lifecycle position

03.04 is Wave 1 / scope lane and depends only on canonical `03.01=DONE`. At this base 03.01–03.03 are DONE, 03.04 and 03.05 are READY, and 03.06 remains BLOCKED until 03.02–03.05 are all canonically DONE.

## Entry invariants

- PKG-03 denominator/order remains exactly 25 tasks (`03.01`–`03.25`).
- 03.01, 03.02 and 03.03 are canonically DONE.
- 03.04 is READY or IN_PROGRESS.
- 03.05 remains independently READY; no ownership-lane mutation is consumed by this task.
- product identity/publisher/upgrade metadata remain exactly the accepted 03.03 values.
- no 03.06+ lifecycle task is claimed from this branch.

## Mutation boundary

Authorized product mutation after planning gates pass:
- explicitly set the default NSIS install mode to `currentUser` in `apps/desktop/src-tauri/tauri.conf.json`;
- add a task-owned Tauri config overlay for the per-machine NSIS variant with `installMode=perMachine`.

No custom NSIS template and no custom WiX template are authorized by 03.04. MSI/WiX remains the stock per-machine package family.

Task-local governance, plan, manifest, validator, Windows workflow and install-scope contract documentation are authorized.

## Acceptance lifecycle

1. Validate frozen parent-plan digest and task bundle.
2. Validate canonical pre-state `03.01–03.03=DONE`, `03.04=READY`, PKG-03 `3/25`.
3. Prove the default config resolves to explicit NSIS `currentUser`.
4. Prove the machine overlay resolves to explicit NSIS `perMachine`.
5. Prove `both` is absent from both accepted config paths.
6. Use the locked repository-local Tauri CLI on GitHub-hosted Windows to parse/build against both configuration paths without executing an installer.
7. Emit exact-source machine-readable evidence proving zero UAC prompt, zero installer execution and zero privileged/system mutation.
8. Only after genuine evidence passes may 03.04 become DONE in canonical state.

## State reconciliation

Before evidence the canonical shape is `3/25`, 03.04–03.05 READY, cursor 03.04.
After genuine 03.04 evidence the accepted shape is `4/25`, 03.04 DONE, 03.05 READY, cursor 03.05.
03.06 remains BLOCKED until 03.05 also becomes canonical DONE.

## Explicit non-actions

No installer install/uninstall, no UAC prompt/elevation execution, no service registration, no exact payload ownership, no ACL mutation, no firewall/hosts/resolver/trust-store mutation, no Authenticode signing and no updater configuration.
