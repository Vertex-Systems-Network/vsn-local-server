# PKG-03 03.17 Development Preflight

Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Task: `03.17`
Linear: `ABD-92`

## Dependency/state check

- 03.11 Agent Windows service lifecycle: DONE
- 03.12 installer ACL/state separation: DONE
- 03.13 firewall/hosts/resolver/trust non-mutation boundary: DONE
- canonical tracker: 15/25 = 60%
- 03.17 status: READY at branch activation
- lane: `uninstall`
- 03.16 is active in lane `repair`; 03.18/03.19/03.22 are READY siblings. 03.17 may not consume any branch-local result from them.
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

Planning may change only the 03.17 planning/contract bundle.

After planning gates pass, implementation may initially add only task-owned:
- `scripts/ci/pkg03-0317-*` validator/lifecycle/evidence helpers;
- `.github/workflows/pkg03-0317-*` exact-head Windows certification;
- tracker/master projection surfaces only after genuine accepted evidence.

Initial 03.17 authority does **not** permit:
- Tauri configuration mutation;
- custom NSIS/WiX template or hook mutation;
- broad recursive deletion logic;
- package identity/version/upgrade-code changes;
- product payload source mutation;
- service identity/account changes;
- ACL widening or security-state relocation;
- firewall/hosts/resolver/trust mutation;
- 03.18/03.19/03.20/03.21 behavior;
- signing secret access or updater mutation.

## Fail-closed rule

`change_required=false` is a certification-first conclusion.

If exact-head uninstall evidence leaves required owned artifacts behind, deletes preserved dirty data, follows a reparse-point escape, or exposes an unresolved machine-security ownership conflict:
1. retain exact failing evidence;
2. classify product behavior vs certification/harness defect;
3. do not weaken ownership/preservation acceptance;
4. open a bounded change-control amendment naming the minimum NSIS/WiX/Tauri/product surface;
5. rerun planning/governance before mutating that surface.

## Planned evidence

The task-specific Windows job must build exact candidate packages and produce `pkg03-0317-uninstall-cleanup` containing at minimum:
- `evidence.json`
- `evidence.json.sha256`
- per-format package hashes/ProductCode where applicable
- pre/post owned-artifact inventories
- preserved marker paths, bytes, SHA-256 and ACL/security metadata
- network/trust non-mutation snapshot hashes
- service/ARP/shortcut cleanup observations
- machine security-state classification
- NSIS UI/action evidence and MSI verbose uninstall log
- exact source/toolchain metadata
- zero tracked repository drift proof
