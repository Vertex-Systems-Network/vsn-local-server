# PKG-03 03.01 — Windows Installer Architecture Contract v1

Status: frozen task execution contract.
Canonical base: `4606579e07ae57785d1bc1dc12073ea1d036ab4d`.
Parent package plan: `.ai/plans/pkg03-windows-installer-v1.md`.
Task: `03.01`.
Linear: `ABD-76`.

## Acceptance criteria

1. Canonical PKG-03 authority can transition from dormant PKG-02-complete state without changing the 25-task denominator/order.
2. Windows package families are frozen to Tauri v2 NSIS and MSI/WiX for PKG-03.
3. Existing product identity source remains `apps/desktop/src-tauri/tauri.conf.json`; 03.01 must not invent competing product/version identifiers.
4. Ownership boundaries distinguish installer-owned payload/registration artifacts from mutable user/project/config/state data.
5. Later task ownership remains explicit: 03.02 build/artifacts, 03.03 publisher+upgrade metadata, 03.04 detailed scope/elevation, 03.05 exact payload manifest.
6. No firewall/hosts/resolver/trust-store side effects, production signing secrets, updater implementation, Linux/macOS release or pentest work is introduced.
7. Task-specific GitHub-hosted Windows certification emits evidence bound to exact source SHA.
8. Only after task evidence passes may canonical state reconcile `03.01=DONE` and expose dependency-ready `03.02`–`03.05`.

## Frozen architecture decisions

- Packaging framework: existing Tauri v2 Desktop packaging boundary.
- Package families: `NSIS` and `MSI/WiX`.
- Product identity source: Tauri config (`productName`, `version`, `identifier`).
- Accepted current identity observed at task base: `VSN Dev Platform`, `0.38.1`, `dev.vsn.platform`.
- Publisher and upgrade identifiers: unresolved here by design; owned by 03.03.
- Install-scope/elevation behavior: unresolved here by design; owned by 03.04.
- Exact installed-file/resource paths: unresolved here by design; owned by 03.05.
- Installer-owned classes: packaged application binaries, installer-created registrations, shortcuts and service registration metadata once introduced by their owning tasks.
- Non-owned mutable classes: user/project/config/state data unless a later frozen task explicitly declares an owned generated artifact.

## Evidence

Required workflow: `PKG-03 03.01 Architecture Contract`.
Required validator: `python scripts/ci/validate-pkg03-0301.py`.
Required governance: AI Planning Governance, Repository Governance, PKG-03 Acceptance Sequence.

## Exit state

After genuine 03.01 acceptance and state reconciliation:
- `done=1`, `percent=4.0`, `complete=false`;
- `03.01=DONE`;
- `03.02`, `03.03`, `03.04`, `03.05=READY`;
- no task is automatically IN_PROGRESS;
- deterministic resume cursor is `03.02`;
- maximum active task count remains 5.
