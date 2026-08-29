# PKG-03 03.12 — Development Preflight

Status: planning-only; product/config/certification mutation remains blocked until the exact planning head passes all five required governance gates.

## Canonical entry

- base: `0eaa4abb7c5e817334f13672952a5901fbbc8fa9`
- task: `03.12`
- Linear: `ABD-87`
- state: READY
- dependencies: `03.07=DONE`, `03.10=DONE`
- PKG-03: `11/25 = 44%`

## Planned implementation surfaces after gate authorization

Task-owned/new:
- `apps/desktop/src-tauri/windows/pkg03-0312-acl-state.nsh`
- `apps/desktop/src-tauri/windows/fragments/pkg03-0312-acl-state.wxs`
- `scripts/ci/pkg03-0312-acl-state-lifecycle.ps1`
- `scripts/ci/validate-pkg03-0312.py`
- `.github/workflows/pkg03-0312-acl-state-lifecycle.yml`

Shared/minimum:
- `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh` — include/invoke bounded 03.12 hook logic without changing accepted 03.11 service identity/order.
- `apps/desktop/src-tauri/tauri.windows.conf.json` — add only 03.12 WiX fragment/feature refs while preserving 03.10 resources and 03.11 hook/feature.

Post-evidence projection only:
- `certification/pkg03-windows-installer-v1.json`
- `docs/MASTER-EXECUTION-STATUS.json`
- designated live projections only if repository convention requires them.

## Forbidden implementation surfaces without change control

- `crates/vsn-security/**`
- `crates/vsn-config/**`
- `crates/vsn-core/**`
- `apps/agent/**`
- accepted 03.10 payload behavior or 03.11 service identity/runtime
- full NSIS or WiX template fork
- PATH/environment mutation
- firewall/hosts/resolver/trust-store mutation
- repair/rollback/reboot/unattended/signing/updater/recovery
- PKG-03 denominator/order/DAG/dependencies/product version
- accepted 03.01–03.11 evidence.

## Scope budget

Implementation slice: maximum 9 changed files, maximum 5 new files, maximum 2 shared surfaces. Any requirement outside this budget or any runtime/security-source mutation triggers `STOP_AND_REASSESS`.

## Exact planning authorization gates

1. AI Planning Governance
2. Repository Governance
3. PKG-03 Acceptance Sequence
4. Engineering Contract Governance
5. Operational Governance

All five must pass on the exact planning head before product/config/certification mutation.

## Acceptance direction

Certification must cover current-user negative machine-state boundary, per-machine NSIS and MSI machine-shared IPC ACLs, SID-level rights, actual LocalService-context mutable/config path resolution, install-root separation, 03.10/03.11 regressions, zero tracked drift, and truthful deferral of comprehensive dirty-data uninstall to 03.17.
