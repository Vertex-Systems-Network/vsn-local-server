# PKG-03 03.12 Security Amendment

Date: 2026-08-29
Approval ref: `conversation:user-2026-08-29-continue-security-amendment`
Scope: PKG-03 task 03.12 only
Classification: `PLAN_REALITY_MISMATCH` + `SECURITY_ASSUMPTION_CHANGE`

## Triggering evidence

Exact failed source head: `c9792a7e5ab890c162ffb62ab3121cb0d9f4074f`
Workflow run: `33222396953`
Job: `99019015513`
Diagnostics artifact: `9705943122`
Artifact digest: `sha256:d4e7a0e055eeabeccec7962cfc4444f018eb29e5cca108fdc62f0827361270a8`
Exact failure: `msi key SYSTEM missing FullControl.`

The current-user and per-machine NSIS lanes, accepted Agent service lifecycle, ProgramData IPC contract under an ordinary creator, LocalService ProjectDirs observation, and mutable-state/install-root separation all passed before the MSI assertion. MSI/WiX build and installation also passed. The failure is therefore a real runtime ACL semantics defect, not an installer-build or harness-only issue.

## Approved bounded correction

This amendment admits exactly one previously frozen product/security implementation surface for 03.12:

- `crates/vsn-security/src/lib.rs`

The correction must preserve the existing shared IPC secret location and all accepted integration surfaces. Creator-aware ACL construction must satisfy these non-negotiable floors:

- `SYSTEM=FullControl`
- `Administrators=FullControl`
- `LocalService=Read`
- an ordinary creator retains directory FullControl plus key Read
- when the creator SID duplicates a privileged baseline SID, the creator-specific grant must be non-destructive
- SYSTEM must never be downgraded by a duplicate creator Read grant
- Administrators must never be downgraded by creator handling
- LocalService must never gain write/full-control through creator handling

## Non-goals / preserved boundaries

- no Users/Everyone broad grants
- no IPC secret relocation
- no installer-owned duplicate ACL writer
- no ProjectDirs redesign or hard-coded service-user path
- no Agent service identity/account change
- no accepted 03.05/03.10 payload ownership change
- no PATH, resolver, firewall, trust-store, signing, updater, rollback or recovery expansion
- comprehensive dirty-data uninstall preservation remains owned by `03.17`

## Required implementation evidence

The amended exact-head slice must include focused tests proving SYSTEM creator preservation, LocalService non-escalation, and ordinary creator rights, then rerun the genuine current-user NSIS, per-machine NSIS, MSI/WiX, Agent service, ACL, LocalService ProjectDirs, state/config separation, exact evidence binding and zero tracked drift lifecycle.

Canonical PKG-03 progress remains 11/25 = 44% until that exact-head evidence succeeds and a separate accepted-state projection is committed.
