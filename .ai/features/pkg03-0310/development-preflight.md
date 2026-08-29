# PKG-03 03.10 Development Preflight

Canonical base: `4f5e8ab30f030e758c52c4ca4ac08f73f896247a`
Task: `03.10`
Linear: `ABD-85`

## Dependency/state check

- 03.02 deterministic Windows bundle build: DONE
- 03.05 owned payload/install-root containment: DONE
- canonical tracker: 8/25 = 32%
- 03.10 status: READY
- parallel lane: `payload`
- 03.09 is independently active on lane `desktop`; no branch/code sharing is permitted.

## Locked inputs

- product: `VSN Dev Platform`
- version: `0.38.1`
- bundle identifier: `dev.vsn.platform`
- publisher: `Vertex Systems Network`
- CLI package/binary: `vsn` / `vsn.exe`
- Agent package/binary: `vsn-agent` / `vsn-agent.exe`
- install destinations: `bin/vsn.exe`, `bin/vsn-agent.exe`
- Tauri CLI evidence version: `2.11.4`
- Node: `22.12.0`
- Rust: `1.97.1`

## Mutation authority

Planning stage may change only this 03.10 planning bundle.

After planning gates pass, implementation may change only:
- `apps/desktop/src-tauri/tauri.windows.conf.json` for the Windows-only `beforeBundleCommand` and two-resource mapping;
- task-owned `scripts/ci/pkg03-0310-*` staging/validation/certification helpers;
- `.github/workflows/pkg03-0310-*` task certification;
- tracker/master state only after genuine exact-head acceptance.

The accepted base `apps/desktop/src-tauri/tauri.conf.json` must remain byte-identical to canonical 03.08/03.09 input.

No service registration, PATH mutation, ACL mutation, custom installer template, signing, updater or recovery mutation is authorized.
