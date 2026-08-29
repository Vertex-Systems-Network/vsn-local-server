# PKG-03 03.12 — Installer ACLs and State Separation Plan v1

Status: frozen planning checkpoint; implementation is task-locally blocked until exact-head governance authorization.
Task: `03.12 — Installer ACLs, state/config directories and user-data separation`
Linear: `ABD-87`
Canonical base: `0eaa4abb7c5e817334f13672952a5901fbbc8fa9`
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`

## Objective

Certify and, only where necessary, integrate the Windows installer with VSN's already-existing storage security model so immutable package payload, machine-shared IPC security, and process-context mutable data/config remain explicitly separated.

## Frozen architecture

### A. Installer-owned immutable payload
Only accepted package-owned files live under the selected install root. 03.05/03.10 ownership is unchanged.

### B. Machine-shared IPC security
Canonical location: `%PROGRAMDATA%\VSN\security\ipc.key`.
The runtime `vsn-security` implementation owns secret generation and final ACL tightening. 03.12 may add only bounded installer integration that is compatible with, and never weaker than, the existing SID policy.

### C. Process-context state/config
`ProjectDirs("dev","VSN","VSN Platform")` remains authoritative for data/config. The installed Agent runs LocalService, so its paths must be measured in that execution context. No interactive-user path may be hardcoded as the service state path.

## Implementation slices after planning gates

1. **Static authority validator** — validate task manifest, accepted base/dependencies, immutable owned-payload set, required/forbidden surfaces, existing security constants, and 03.11 service contract.
2. **NSIS integration** — add one task-owned 03.12 hook fragment and minimally include/invoke it from the accepted 03.11 hook without changing service identity/order. Current-user path must remain a negative machine-state boundary.
3. **WiX integration** — add a task-owned fragment/feature and minimally append its refs to `tauri.windows.conf.json`; no full template fork and no duplicate executable ownership.
4. **Windows lifecycle harness** — build exact-head current-user NSIS, per-machine NSIS and MSI/WiX; inspect native ACLs by SID; record resolved state/config roots; prove separation and regressions.
5. **Exact evidence** — bind source SHA/run/job/artifact, hashes, UI/install results, ACL descriptor observations and zero tracked repository drift.

## Security acceptance

Required ProgramData security contract:
- directory inheritance removed;
- SYSTEM Full Control;
- Builtin Administrators Full Control;
- LocalService Read only;
- creating/current SID Full Control on the directory;
- secret file inheritance removed;
- SYSTEM/Admins Full Control;
- LocalService and creating/current SID Read only.

Do not infer rights from localized account display names; evidence should bind numeric SIDs/security descriptors.

## Current-user boundary

Installing/uninstalling the current-user package alone must not create or ACL `%PROGRAMDATA%\VSN\security`. If a bounded runtime probe is needed, pre/post evidence must distinguish package mutation from runtime-created state.

## Per-machine NSIS/MSI boundary

Machine installs must preserve 03.11 service behavior and prove shared IPC can be consumed by LocalService without granting it write access to the secret. Resolved mutable data/config paths must be recorded and must remain outside Program Files/install root.

## Ownership and preservation

03.12 freezes directory classes and ACL boundaries. It does not claim the comprehensive dirty-user-data cleanup/preservation matrix owned by 03.17. No uninstall step may delete arbitrary ProjectDirs or user workspace data under this task.

## Change-control triggers

Stop before mutation if evidence requires:
- modifying `vsn-security`, `vsn-config`, `vsn-core`, or Agent runtime;
- changing 03.11 service account/identity/order;
- changing accepted payload ownership;
- broadening ACL principals/rights;
- changing task acceptance, DAG/dependencies, or uninstall preservation ownership;
- more than 9 implementation files / 5 new / 2 shared surfaces.

## Required planning gates

The exact planning head must pass AI Planning Governance, Repository Governance, PKG-03 Acceptance Sequence, Engineering Contract Governance, and Operational Governance before implementation.

## Final task acceptance

03.12 becomes DONE only after exact-head Windows evidence proves:
1. canonical/dependency/manifest authority;
2. current-user install negative ProgramData mutation boundary;
3. per-machine NSIS shared IPC location and exact ACLs;
4. MSI/WiX shared IPC location and exact ACLs;
5. LocalService Agent remains healthy using the accepted service contract;
6. actual service-context data/config roots are recorded and outside install root;
7. executable payload ownership remains exactly the accepted 03.05/03.10 set;
8. no forbidden runtime/security/network/PATH/template mutation;
9. 03.17 preservation boundary is not overclaimed;
10. zero tracked repository drift and exact evidence binding.

Only after genuine evidence may canonical tracker/master projection advance from 11/25 to 12/25.
