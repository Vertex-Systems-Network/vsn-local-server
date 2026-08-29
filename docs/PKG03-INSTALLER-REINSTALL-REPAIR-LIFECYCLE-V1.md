# PKG-03 03.16 Installer Reinstall / Repair Lifecycle Contract v1

Status: frozen
Task: `03.16`
Linear: `ABD-91`
Canonical planning base: `f3afb66e588d01ff2e8cb37273ad413862a4edaf`

## Purpose

This contract defines what counts as an accepted idempotent reinstall and repair for the three already accepted Windows installer lifecycles. It does not add a new product repair subsystem.

## Common invariants

Every lifecycle must preserve:
- exact candidate package/source binding;
- accepted install scope and install root;
- expected owned executable SHA-256 after every healthy/repair phase;
- one logical product installation, not duplicate ARP/shortcut/service registrations;
- accepted package identity/version behavior;
- zero tracked repository drift.

A successful process exit without exact byte restoration is not accepted repair evidence.

## Integrity states

The pre/post repair classifier is inherited from 03.14:
- `MATCH`
- `MISSING`
- `HASH_MISMATCH`

For a missing/tamper probe:
1. capture expected candidate SHA-256;
2. prove the damaged state classification;
3. invoke the genuine format-specific reinstall/repair path;
4. recompute SHA-256;
5. require `MATCH` and exact equality to the expected candidate hash.

## NSIS current-user

- use exact current-user candidate setup;
- no elevation to machine scope;
- no `VSN-Agent` Windows service creation;
- healthy rerun must be idempotent;
- missing/tampered repair may target Desktop, CLI, or Agent;
- same-version setup rerun must genuinely restore exact bytes.

## NSIS per-machine

- use exact elevated per-machine candidate setup;
- stop `VSN-Agent` before destructive repair probes;
- destructive missing/tamper targets are Desktop or CLI only;
- Agent payload is read-only in 03.16;
- after repair, verify accepted service identity/configuration and bounded start/health behavior;
- do not claim live-running service repair coordination.

## MSI/WiX per-machine

- use exact installed ProductCode from the candidate;
- native Windows Installer repair is authoritative;
- force-file reinstall semantics are allowed for damaged-file repair;
- retain verbose `/L*V` repair logs;
- stop `VSN-Agent` before destructive repair probes;
- destructive targets are Desktop or CLI only;
- after repair, verify accepted service identity/configuration and bounded start/health behavior.

## State / ACL invariants

Repair may not:
- broaden accepted SYSTEM/Administrators/LocalService permissions;
- move machine security state away from its accepted location;
- create machine-wide security state in current-user lifecycle;
- create duplicate service, shortcut, ARP, or install-root state.

03.16 does not certify comprehensive dirty-user-data uninstall behavior.

## Failure semantics

Any of the following is a task failure:
- damaged file remains missing or hash-mismatched after a claimed repair;
- healthy rerun changes expected candidate bytes without an approved package change;
- duplicate registration/service/install identity appears;
- service/ACL/security-state invariants are widened or relocated;
- harness mutates a forbidden product surface to make the test pass;
- evidence is not bound to the exact source head.

A failure may lead only to a separately governed minimum-scope amendment. It may not weaken this contract.

## Nonclaims

No claim is made for:
- running-process/Restart Manager repair;
- interrupted-install rollback;
- reboot semantics;
- unattended deployment;
- comprehensive dirty-user-data uninstall preservation;
- signing/updater/recovery/cross-platform behavior.
