# VSN-DEVIFY-PRODUCT-TRACK-V1

Date: 2026-10-17
Disposition: APPROVED
Engineering change classification: CONTRACT CHANGE (product scope addition only)
Approval scope: PROJECT
Approval reference: `conversation:user-2026-10-17-vsn-devify-product-vision`
Source canonical main: `635e77afa107bf2a01c70a443a0c054226fb1ec6`

## Problem

The owner's product vision (isolated per-project environments, every framework and runtime version, mail testing, API client, tunnels, FTP/SFTP, version control, teams, analytics, AI) is wider than the frozen hardening packages PKG-03 … PKG-08, which only certify installer, updater, platforms, security and resilience.

## Approved change

1. The product is named **VSN Devify** in the user interface. The installer identity frozen by PKG-03 ("VSN Dev Platform") is renamed only through a governed change at public release (DVF-14).
2. A **product track** DVF-01 … DVF-14 is added, defined in `docs/VSN-DEVIFY-PRODUCT-PLAN.md`. DVF-01 (Spaces) is first.
3. DVF work runs in the `NON_ACCEPTANCE_PREIMPLEMENTATION` lane (`.ai/governance/FAST-DEVELOPMENT.md`) until each package receives its own acceptance contract.

## What does not change

- The canonical PKG-03 tracker, dependency DAG, denominators and statuses (PKG-03 stays 21/25 with 03.22 READY).
- No DVF task is marked DONE without its own acceptance evidence.
- Frozen manifests, workflows and hash-pinned files (`tauri.conf.json`, package locks) are not edited by this change.
