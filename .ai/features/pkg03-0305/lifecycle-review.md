# PKG-03 03.05 — Lifecycle Review

Task: `03.05`
Linear: `ABD-80`
Canonical base: `7cd671de8af410ee348083c42c716cce1dd22543`

## Lifecycle position

03.05 is the final Wave 1 prerequisite. Canonical `03.01–03.04=DONE`; PKG-03 is `4/25`, 03.05 is READY, and 03.06–03.10 remain BLOCKED until this ownership contract is accepted.

## Entry invariants

- PKG-03 denominator/order remains exactly 25 tasks (`03.01`–`03.25`).
- 03.01–03.04 are canonically DONE.
- 03.05 is READY or IN_PROGRESS and depends only on 03.01.
- deterministic cursor is 03.05.
- no Wave 2 task is accepted from this branch.
- 03.03 identity and 03.04 scope/elevation metadata remain unchanged.

## Mutation boundary

After planning gates pass, 03.05 may add:
- `installer/windows/owned-payload.v1.json` as the exact root-relative durable payload ownership contract;
- task-local validator and Windows evidence workflow.

No `tauri.conf.json`, NSIS/WiX template, executable source, service configuration, PATH, registry, ACL, updater or signing mutation is authorized.

## Acceptance lifecycle

1. Validate frozen parent-plan digest and task bundle.
2. Validate canonical pre-state `03.01–03.04=DONE`, `03.05=READY`, PKG-03 `4/25`.
3. Validate the ownership manifest has exactly Desktop, CLI and Agent executable entries.
4. Validate all owned paths are canonical, case-insensitively unique and strictly relative to `${INSTALL_ROOT}`.
5. Run negative containment vectors for drive/UNC/device/traversal/ADS/reserved-name/control/trailing-dot-space/collision cases.
6. Use locked Cargo metadata on GitHub-hosted Windows to prove package names/versions and build CLI/Agent executables without installing them.
7. Emit exact-source machine-readable evidence with zero installer execution and zero privileged/system mutation.
8. Only after genuine evidence passes may 03.05 become DONE.

## State reconciliation

Pre-evidence:
- `done=4`, `percent=16.0`;
- 03.05 READY;
- cursor 03.05;
- 03.06–03.10 BLOCKED.

After genuine 03.05 evidence:
- `done=5`, `percent=20.0`;
- 03.05 DONE;
- 03.06, 03.07, 03.08, 03.09 and 03.10 become READY;
- deterministic cursor advances to 03.06.

## Explicit non-actions

No actual install/uninstall, no CLI/Agent package placement into a real installer, no Start Menu/registration mutation, no service lifecycle, no ACL/state-directory change, no repair/uninstall cleanup, no Authenticode signing and no updater behavior.
