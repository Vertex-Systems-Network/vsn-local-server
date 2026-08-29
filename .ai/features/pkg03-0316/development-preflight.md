# PKG-03 03.16 Development Preflight

Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Task: `03.16`
Linear: `ABD-91`

## Dependency/state check

- 03.11 Agent Windows service lifecycle: DONE
- 03.12 installer ACL/state separation: DONE
- 03.14 payload integrity detection: DONE
- 03.15 installer diagnostics/exit semantics: DONE
- canonical tracker: 15/25 = 60%
- 03.16 status: READY at branch activation; Linear moved to In Progress on 2026-08-30
- lane: `repair`
- READY siblings 03.17 (`uninstall`), 03.18 (`recovery`), 03.19 (`runtime`) and 03.22 (`signing`) are independent only within their own frozen authorities; 03.16 must not consume branch-local results from them.
- frozen max parallel implementation tasks: 5

## Locked inputs

- product: `VSN Dev Platform`
- version: `0.38.1`
- accepted formats: NSIS current-user, NSIS per-machine, MSI/WiX per-machine
- runner: `windows-2025`
- Node: `22.12.0`
- Rust: `1.97.1`
- Tauri CLI evidence version: `2.11.4`
- Agent service SCM name: `VSN-Agent`
- Agent service account: `NT AUTHORITY\LocalService`
- parent plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`
- canonical activation base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`

## Mutation authority

Planning may change only the 03.16 planning/contract bundle.

After planning gates pass, implementation may add only task-owned:
- `scripts/ci/pkg03-0316-*` validator/lifecycle/evidence helpers;
- `.github/workflows/pkg03-0316-*` exact-head Windows certification;
- tracker/master projection surfaces only after genuine accepted evidence.

Initial 03.16 authority does **not** permit:
- Tauri config mutation;
- custom NSIS/WiX template or hook mutation;
- package identity/version/upgrade-code changes;
- product payload source mutation;
- new repair daemon/self-healing runtime;
- service identity/account changes;
- ACL widening or security-state relocation;
- firewall/hosts/resolver/trust mutation;
- 03.17/03.18/03.19/03.20/03.21 behavior;
- signing secret access or updater mutation.

## Fail-closed rule

`change_required=false` is a certification-first conclusion, not a guarantee that stock generated installers will satisfy every repair case.

If exact-head evidence shows that a required lifecycle cannot genuinely restore exact bytes or preserve frozen invariants:
1. classify the failure as product behavior vs certification/harness defect;
2. retain all failing evidence;
3. do not weaken acceptance;
4. open a bounded change-control amendment naming the minimum additional product surface;
5. rerun planning/governance before that product surface is mutated.

## Planned evidence

The task-specific Windows job must build exact candidate packages and produce `pkg03-0316-reinstall-repair` containing at minimum:
- `evidence.json`
- `evidence.json.sha256`
- MSI verbose repair logs
- per-format lifecycle observations/actions
- expected/pre-repair/post-repair payload hashes
- install identity/cardinality observations
- service/ACL invariant observations where applicable
- exact source/toolchain/package metadata
- zero tracked repository drift proof
