# PKG-03 Windows Installer — Frozen Execution Plan v1

Status: frozen package execution contract. Product mutation is prohibited until this planning bundle is merged and 03.01 activates PKG-03 from live canonical main.

Feature: `pkg03-windows-installer`
Version: `1.0.0`
Canonical base: `67e9a64da07ae36646cef7f95e343a069b4da5bf`
Decision/approval: `conversation:user-2026-08-26-pkg03-plan-align-then-start`
Denominator: exactly **25** acceptance tasks (`03.01`–`03.25`).

## Outcome

Deliver a Windows installer for VSN Local Server that installs the accepted Desktop/CLI/Agent payloads safely, supports governed user/machine install contexts, service lifecycle, repair/uninstall/rollback, unattended deployment, signing verification and exact-head Windows acceptance without crossing into PKG-04 updater/recovery or PKG-05 cross-platform release.

## Frozen product boundary

In scope:
- Tauri 2 Windows packaging using supported NSIS and MSI/WiX outputs;
- deterministic GitHub-hosted Windows builds;
- installer identity/version/publisher/upgrade metadata;
- per-user and per-machine/elevated install boundaries;
- owned payload/resources, Desktop integration, CLI/Agent placement and Windows service install lifecycle;
- ACL/state separation, non-mutation boundaries, repair/uninstall/rollback, running-process/reboot semantics;
- unattended deployment, Authenticode integration/verification, package hashes/SBOM/provenance handoff;
- fresh/dirty Windows acceptance and final exact-head package gate.

Explicit non-goals:
- no automatic updater/update feed, differential update, self-update or recovery orchestration: PKG-04;
- no Linux/macOS release implementation: PKG-05;
- no broad security certification/pentest program: PKG-06/PKG-08;
- no production signing-secret material in repository or CI evidence;
- no firewall/hosts/resolver/trust-store mutation unless a later independently approved plan addendum explicitly changes the contract.

## Current primary-source constraints (reviewed 2026-08-26)

- Tauri 2 supports Windows `.msi` via WiX Toolset v3 and `-setup.exe` via NSIS; MSI builds require Windows. Source: https://v2.tauri.app/distribute/windows-installer/
- Tauri NSIS supports current-user, per-machine and selectable install modes; per-machine requires administrator privileges. Same source.
- Microsoft `msiexec` defines quiet/passive UI, install/uninstall, restart controls and logging semantics used by enterprise acceptance. Source: https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/msiexec
- Windows Installer uses Restart Manager to reduce reboots and coordinate files/services in use. Sources: https://learn.microsoft.com/en-us/windows/win32/rstmgr/about-restart-manager and https://learn.microsoft.com/en-us/windows/win32/msi/using-windows-installer-with-restart-manager
- Microsoft currently recommends Artifact Signing for non-Store Windows distribution; signing identity/secrets remain external trust material. Sources: https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation and https://learn.microsoft.com/windows/apps/package-and-deploy/code-signing-options

## Parallel execution model

Parallelism is a **DAG**, never 25 uncontrolled branches. At most five implementation lanes mutate independent surfaces concurrently. A task may start only when every `depends_on` task is canonically `DONE` on `main`.

Machine-readable rules:
- `active_task` is the deterministic resume cursor: the lowest-ID non-DONE task that is READY/IN_PROGRESS, or null before activation/completion.
- `ready_tasks` is the full set whose dependencies are DONE and which are not DONE/IN_PROGRESS.
- `active_tasks` is the set actually being implemented; maximum 5.
- branch pattern: `pkg03/<task-id-without-dot>-<slug>`, e.g. `pkg03/0306-nsis-user-install`.
- one authoritative implementation/certification PR per task; stale/superseded PRs are closed unmerged.
- no dependent branch is allowed to claim acceptance based on another branch projection. Only integrated canonical `main` evidence unlocks dependencies.

Waves:
- Wave 0: `03.01`
- Wave 1: `03.02`, `03.03`, `03.04`, `03.05`
- Wave 2: `03.06`, `03.07`, `03.08`, `03.09`, `03.10`
- Wave 3: `03.11`, `03.12`, `03.13`, `03.14`, `03.15`
- Wave 4: `03.16`, `03.17`, `03.18`, `03.19`, `03.20`
- Wave 5: `03.21`, `03.22`, `03.23`
- Wave 6: `03.24`
- Wave 7: `03.25`

## Frozen task graph

- **03.01 — Activate PKG-03 execution authority and freeze Windows installer architecture, format, identity and ownership contract**  
  Wave `0` · lane `control` · depends on: `none`
- **03.02 — Deterministic GitHub-hosted Windows bundle build and artifact manifest**  
  Wave `1` · lane `build` · depends on: `03.01`
- **03.03 — Package identity, version, publisher and upgrade metadata contract**  
  Wave `1` · lane `identity` · depends on: `03.01`
- **03.04 — Install-scope and elevation contract for per-user and per-machine modes**  
  Wave `1` · lane `scope` · depends on: `03.01`
- **03.05 — Owned payload/resource manifest and install-root containment**  
  Wave `1` · lane `ownership` · depends on: `03.01`
- **03.06 — NSIS current-user interactive install and uninstall lifecycle**  
  Wave `2` · lane `build` · depends on: `03.02, 03.03, 03.04, 03.05`
- **03.07 — NSIS per-machine elevated install and uninstall lifecycle**  
  Wave `2` · lane `scope` · depends on: `03.02, 03.03, 03.04, 03.05`
- **03.08 — MSI/WiX enterprise install, uninstall and Add/Remove Programs lifecycle**  
  Wave `2` · lane `enterprise` · depends on: `03.02, 03.03, 03.04, 03.05`
- **03.09 — Desktop Start Menu, shortcut and application-registration lifecycle**  
  Wave `2` · lane `desktop` · depends on: `03.03, 03.05`
- **03.10 — CLI and Agent payload placement, discovery and launch contract**  
  Wave `2` · lane `payload` · depends on: `03.02, 03.05`
- **03.11 — VSN Agent Windows service install, start, health and removal lifecycle**  
  Wave `3` · lane `service` · depends on: `03.07, 03.10`
- **03.12 — Installer ACLs, state/config directories and user-data separation**  
  Wave `3` · lane `security` · depends on: `03.07, 03.10`
- **03.13 — Firewall, hosts, resolver and trust-store non-mutation boundary**  
  Wave `3` · lane `boundary` · depends on: `03.06, 03.07, 03.08`
- **03.14 — Installed payload integrity and repair detection for missing or tampered owned files**  
  Wave `3` · lane `integrity` · depends on: `03.06, 03.07, 03.08, 03.10`
- **03.15 — Installer logging, deterministic exit codes, cancellation and operator diagnostics**  
  Wave `3` · lane `diagnostics` · depends on: `03.06, 03.07, 03.08`
- **03.16 — Idempotent reinstall and repair lifecycle**  
  Wave `4` · lane `repair` · depends on: `03.11, 03.12, 03.14, 03.15`
- **03.17 — Uninstall owned-artifact cleanup with user-data preservation**  
  Wave `4` · lane `uninstall` · depends on: `03.11, 03.12, 03.13`
- **03.18 — Transactional install failure rollback and interrupted-install recovery**  
  Wave `4` · lane `recovery` · depends on: `03.11, 03.12, 03.14, 03.15`
- **03.19 — Running Desktop, CLI and Agent handling with Restart Manager/service coordination**  
  Wave `4` · lane `runtime` · depends on: `03.11, 03.15`
- **03.20 — Reboot-required, no-restart and pending-reboot semantics**  
  Wave `4` · lane `reboot` · depends on: `03.15, 03.19`
- **03.21 — Unattended and silent NSIS/MSI deployment contract**  
  Wave `5` · lane `automation` · depends on: `03.16, 03.17, 03.20`
- **03.22 — Authenticode signing integration and signature-verification gate**  
  Wave `5` · lane `signing` · depends on: `03.02, 03.03, 03.14`
- **03.23 — Installer hash, SBOM/provenance manifest and PKG-05 release handoff**  
  Wave `5` · lane `provenance` · depends on: `03.02, 03.14, 03.22`
- **03.24 — Fresh and dirty Windows VM install/repair/uninstall acceptance matrix**  
  Wave `6` · lane `e2e` · depends on: `03.16, 03.17, 03.18, 03.19, 03.20, 03.21, 03.22, 03.23`
- **03.25 — Final Windows installer exact-head gate, full PKG-03 regression matrix and PKG-04 handoff**  
  Wave `7` · lane `final` · depends on: `03.02, 03.03, 03.04, 03.05, 03.06, 03.07, 03.08, 03.09, 03.10, 03.11, 03.12, 03.13, 03.14, 03.15, 03.16, 03.17, 03.18, 03.19, 03.20, 03.21, 03.22, 03.23, 03.24`

## Per-task lifecycle and merge gate

Before product mutation for every task:
1. Re-read live `main`, master status, PKG-03 tracker, open PKG-03 PRs and Linear mirror.
2. Verify package plan/manifest bytes and hashes.
3. Refresh only official-source market/platform delta; `change_required` blocks mutation.
4. Create task feature bundle/preflight derived from this package plan.
5. Confirm dependencies are canonically DONE, task is READY/IN_PROGRESS, and lane count <= 5.

Every task PR must:
- start from then-current canonical `main`;
- map every changed product file to task acceptance criteria;
- run AI Planning Governance, Repository Governance and PKG-03 Acceptance Sequence on exact final head;
- run task-specific GitHub-hosted Windows certification plus the frozen high-risk regression subset relevant to changed boundaries;
- upload evidence bound to source SHA/run/job/artifact and verify cleanup/non-mutation;
- have no unresolved review blocker;
- merge only with `expected_head_sha`;
- immediately re-read canonical state after merge before unlocking dependents.

State reconciliation belongs in the same task PR only after genuine task evidence is obtained; no optimistic DONE projection. Final package completion after 03.25 uses one separate state-only projection PR.

## Error and resume protocol

At every mutation checkpoint persist in GitHub + Linear:
- canonical main SHA;
- package tracker digest and plan digest;
- current task status, dependencies, wave/lane and authoritative PR;
- exact accepted/failed run IDs and artifact IDs;
- failure classification: product defect / certification defect / runner-infrastructure / governance-state;
- last verified step and exact next action.

On resume:
1. trust live GitHub, not chat/cache;
2. if main or frozen-plan bytes drift, stop and reconcile;
3. if a task PR is stale behind main, rebase/recreate from canonical main rather than force-moving accepted historical refs;
4. failures may change only the minimum AC-mapped scope; material contract changes require change control;
5. never mark a task DONE from a green run on a different source SHA.

## PR count target

Planned clean lifecycle: **27 PRs** = 1 package freeze PR + 25 task PRs + 1 final package completion projection. Defect/replacement PRs are exceptional and do not change the fixed 25-task denominator.
