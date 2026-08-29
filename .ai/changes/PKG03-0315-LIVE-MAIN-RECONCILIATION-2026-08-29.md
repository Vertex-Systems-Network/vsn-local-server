# PKG-03 03.15 Live-Main Reconciliation — 2026-08-29

Task: `03.15 — Installer diagnostics and exit semantics`
Linear: `ABD-90`
Historical frozen planning base: `4f5e8ab30f030e758c52c4ca4ac08f73f896247a`
Historical branch head before reconciliation: `5cc0be73873e998ba33b0b8212e152bfcbc19603`
Refreshed live execution base: `0eaa4abb7c5e817334f13672952a5901fbbc8fa9`

## Classification

This is an evidence-only live-main reconciliation. It does not amend the frozen 03.15 diagnostics contract and does not authorize product/runtime/installer mutation.

The historical 03.15 run `33122006556` was genuine and green, but it is stale for current acceptance because the branch was 188 commits behind refreshed live main. Fresh current-main-composed exact-head evidence is mandatory.

## Preserved frozen contract

- MSI/WiX visible install/uninstall success exits `0` and emits native `/L*V` logs whose size and SHA-256 are evidence-bound.
- Genuine pre-commit MSI user cancellation returns `1602` and leaves no committed product state.
- NSIS successful setup/uninstall exits `0`.
- Genuine pre-commit NSIS setup cancellation returns `1`.
- Stock NSIS persistent logging is not claimed.
- Generated NSIS uninstaller cancellation exit-code propagation is not claimed.
- No silent/passive deployment, repair, reboot, rollback/recovery, running-process coordination, signing, updater, service, ACL, firewall/hosts/DNS/trust, Tauri config, installer-template or toolchain mutation is authorized.

## Live-main audit

The accepted installer identity/configuration inputs relevant to this task remain compatible with the frozen contract. In particular, `apps/desktop/src-tauri/tauri.conf.json`, `apps/desktop/src-tauri/tauri.per-machine.conf.json`, and `installer/windows/owned-payload.v1.json` were already proven byte-identical from the historical task base to refreshed live main during adjacent PKG-03 reconciliation.

Current main additionally contains accepted 03.09–03.11 behavior. Those changes must be exercised by a fresh 03.15 lifecycle run rather than assumed compatible from historical evidence.

## Reconciliation mechanics

- preserve the historical 03.15 head as first parent;
- preserve refreshed live main as second parent;
- use refreshed live-main repository content plus only the frozen task-owned 03.15 planning/certification files and this reconciliation record;
- do not force-rewrite history;
- validate changed paths against refreshed live main, while continuing to verify the frozen manifest against its historical planning base;
- do not project tracker/master completion until genuine current-main-composed evidence passes;
- do not merge automatically.

## Acceptance firewall

Fresh exact-head acceptance still requires the full 03.15 Windows diagnostics workflow, exact evidence binding, native exit-code/log assertions, zero tracked drift and required governance gates. Historical green evidence remains context only.