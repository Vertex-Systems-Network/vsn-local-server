# PKG-03 03.19 Lifecycle Review — Running Desktop, CLI and Agent handling

Reviewed: 2026-08-30
Canonical base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`
Task: `03.19`
Linear: `ABD-94`

## Process matrix

| Resource | Current-user NSIS | Per-machine NSIS | MSI/WiX |
| --- | --- | --- | --- |
| Desktop | running exact installed executable | running exact installed executable | running exact installed executable; observe Restart Manager path |
| CLI | deterministic long-running exact installed CLI invocation | same | same |
| Agent | no machine service may exist | `VSN-Agent` running under accepted LocalService identity | `VSN-Agent` running; observe service/Restart Manager coordination |

## Safe outcome contract

For each running-resource operation the installer must either:
- complete through bounded, observable coordination without corrupting/duplicating package state; or
- block deterministically before destructive mutation and preserve a coherent installed state for retry.

An indefinite wait, silent force kill, post-operation partial payload, or unexplained nonzero result is a failure.

## Harness integrity

Before invoking the installer, the harness records exact PIDs, executable paths and hashes. It does not pre-stop/pre-kill Desktop/CLI/Agent to obtain a pass. Any harness cleanup after a failed/blocked case is separately recorded and cannot be counted as installer coordination.

## Service rule

For per-machine operations, `VSN-Agent` starts from the accepted 03.11 configuration. Installer behavior may stop/start the service only as required by the operation. Service name, account, start mode and executable path must remain invariant if the product remains installed; after accepted uninstall, the service must be absent.

## Nonclaims

03.19 does not certify reboot policy (03.20), silent deployment (03.21), signing, updater recovery, or cross-platform behavior.
