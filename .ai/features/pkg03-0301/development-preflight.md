# PKG-03 03.01 — Development Preflight

Canonical base verified: `4606579e07ae57785d1bc1dc12073ea1d036ab4d`.

## Preconditions

- PR #106 is merged and the frozen PKG-03 plan exists on canonical main.
- Package plan digest matches `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`.
- Tracker denominator/order remains exactly `03.01`–`03.25`.
- Linear ABD-76 is the authoritative mirror for this task.
- No other 03.xx implementation task may start until 03.01 is canonically DONE.

## Market delta

Official-source refresh on 2026-08-26 found no material change to Tauri Windows installer formats, install modes, `msiexec` automation semantics or Windows Restart Manager architecture. `change_required=false`.

## Allowed mutations

- task-local `.ai/` research/lifecycle/preflight/manifest/plan assets;
- architecture contract documentation;
- 03.01 validator/workflow/evidence tooling;
- canonical PKG-03 activation/progress state only after genuine task evidence.

## Prohibited mutations

- installer payload implementation belonging to 03.02+;
- publisher/upgrade metadata finalization belonging to 03.03;
- install-scope/elevation implementation belonging to 03.04;
- exact payload/resource manifest implementation belonging to 03.05;
- updater/recovery or cross-platform release work;
- signing secrets or privileged host mutation.

## Merge rule

Exact final PR head must have green AI Planning Governance, Repository Governance, PKG-03 Acceptance Sequence and PKG-03 03.01 Architecture Contract certification. State may be reconciled to 1/25 only after the task-specific certification has genuinely passed; expected-head protected merge is required.
