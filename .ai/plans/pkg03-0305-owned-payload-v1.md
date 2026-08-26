# PKG-03 03.05 — Owned Payload and Install-Root Containment v1

Status: frozen task execution contract.
Canonical base: `7cd671de8af410ee348083c42c716cce1dd22543`.
Parent package plan: `.ai/plans/pkg03-windows-installer-v1.md`.
Parent package plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`.
Task: `03.05`.
Linear: `ABD-80`.

## Acceptance criteria

1. A machine-readable Windows ownership manifest exists and contains exactly three durable executable entries: Desktop, CLI and Agent.
2. Exact canonical root-relative paths are `VSN Dev Platform.exe`, `bin/vsn.exe`, and `bin/vsn-agent.exe`.
3. The manifest is scope-neutral and anchored only to `${INSTALL_ROOT}`; 03.04 remains authoritative for current-user vs per-machine root selection.
4. CLI/Agent placement is declared as downstream 03.10 authority rather than being prematurely added to Tauri bundling by 03.05.
5. `apps/updater-helper`, user projects/workspaces, mutable config/state, database content, external credentials/certificates and undeclared logs/data are explicitly not installer-owned by this contract.
6. Path validation fails closed for absolute/drive/UNC/device/traversal/ADS/control/reserved-name/trailing-dot-space/empty-segment/non-canonical-separator/case-collision inputs.
7. Locked Cargo metadata proves package identity/version for `vsn-desktop`, `vsn` and `vsn-agent`; Windows certification builds CLI/Agent and verifies expected executable filenames without installation.
8. Evidence is bound to exact source SHA, ownership-manifest digest and locked Cargo/toolchain inputs.
9. No installer execution, privileged mutation, service registration, ACL mutation, signing, updater mutation or external ownership occurs.
10. Accepted state advances only 03.05 from canonical `4/25` to `5/25`, then exposes 03.06–03.10 as READY with cursor 03.06.

## Frozen ownership map

| ID | Relative path | Source authority | Placement authority |
| --- | --- | --- | --- |
| desktop | `VSN Dev Platform.exe` | `apps/desktop/src-tauri` / package `vsn-desktop` | Tauri/03.02 existing bundle |
| cli | `bin/vsn.exe` | `apps/cli` / package `vsn` | 03.10 |
| agent | `bin/vsn-agent.exe` | `apps/agent` / package `vsn-agent` | 03.10 |

The manifest reserves ownership at these paths; it does not claim CLI/Agent are already present in the accepted 03.02 installers.

## Containment contract

The only ownership namespace is `${INSTALL_ROOT}/<relative_path>`.
All manifest paths use `/` as the canonical separator and are interpreted case-insensitively under Windows semantics.
No owned path may resolve outside the installer-selected root, directly or through a reparse point during downstream lifecycle execution.

## Planned repository realization

After planning-gate acceptance:
- add `installer/windows/owned-payload.v1.json`;
- add a validator implementing the exact manifest and lexical containment rules;
- add GitHub-hosted Windows evidence that validates Cargo package identities, builds CLI/Agent binaries, exercises malicious path vectors and proves non-mutation.

No Tauri `externalBin`, `resources`, custom NSIS/WiX template or real install lifecycle change is authorized here.

## Exit state

After genuine 03.05 evidence and reconciliation:
- `done=5`, `percent=20.0`, `complete=false`;
- `03.05=DONE`;
- `03.06–03.10=READY`;
- deterministic cursor `03.06`.
