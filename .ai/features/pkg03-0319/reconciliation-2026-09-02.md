# PKG-03 03.19 — Current-Main Reconciliation

Date: 2026-09-02
Task: `03.19`
Linear: `ABD-94`
Immutable activation witness: `f3afb66e588d01ff2e8cb37273ad413862a4edaf` (`15/25`)
Current canonical execution base: `9910223a5c5c154c98846c1e091d51ae0acf4847` (`18/25`)
Superseded historical PR: `#149`
Superseded branch head: `aa945d62c5938728c1553099a90f29b831f80f44`

## Reconciliation decision

The frozen 03.19 planning bundle remains an immutable activation witness. It is not rewritten to pretend that 03.19 was originally activated at the current package state.

Canonical main has since accepted 03.16, 03.17 and 03.18, and now records PKG-03 at `18/25 = 72%` with deterministic cursor `03.19`. Current diff authorization therefore begins at `9910223a5c5c154c98846c1e091d51ae0acf4847`; already accepted changes before that SHA are not attributable to 03.19.

## Historical failure carried forward as evidence only

PR #149 is closed without merge. Its final branch is retained only as historical evidence. That stale branch replaced the then-current service lifecycle hook with an older Agent CLI stop/uninstall path. Retry after separately recorded operator cleanup then surfaced `VSN Agent service stop failed with exit code 1`.

This is a regression relative to accepted current main, where per-machine uninstall uses native SCM stop/delete handling and explicitly accepts only the idempotent native states required by earlier accepted lifecycle work:

- service already stopped: `1062`;
- service already absent: `1060`;
- service already marked for deletion: `1072`.

The clean 03.19 continuation MUST preserve those semantics.

## Evidence-bound product exception

The historical change-control artifact `.ai/features/pkg03-0319/change-control-2026-08-31.md` remains authoritative for the minimum product exception: prepend Tauri's existing `CheckIfAppIsRunning` guards for the exact Desktop and CLI process names before any `VSN-Agent` service mutation.

Authorized product path remains exactly:

- `apps/desktop/src-tauri/windows/pkg03-0311-agent-service.nsh`

Within that path, the clean continuation may add only the two running-process guards and explanatory comments around them. Existing accepted service registration/start behavior, `SetAutoClose`, native SCM stop/delete commands, idempotence codes, current-user post-uninstall registry cleanup, service identity, package identity, ACL/security boundaries and payload behavior remain unchanged.

## Acceptance state

03.19 remains `READY`, not `DONE`. No canonical progress projection is authorized until a fresh exact-head Windows 03.19 run succeeds and its artifact is independently verified for exact source/run binding, installer hashes, running-resource identity, coordination or safe block, retry completion, MSI Restart Manager evidence, protected-state equality and zero tracked repository drift.

Reboot semantics remain 03.20; silent/passive deployment remains 03.21; signing/provenance/updater/later-package work remains out of scope.
