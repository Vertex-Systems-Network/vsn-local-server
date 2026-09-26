# PKG-04 Dormant Research — Updater & Recovery

Reviewed: 2026-09-05
Canonical source audited: `79812eafdead24de88d8b3fafd19f1bfc0e1435c`
Status: **RESEARCH-ONLY / BLOCKED ON PKG-03 COMPLETE**

## Current baseline

The canonical 18-task PKG-04 sequence in `.ai/plans/pkg04-pkg08-parallel-preplanning-v1.md` remains unchanged. This file does not activate 04.01 and does not authorize updater implementation.

## Current-doc findings

- Tauri v2 updater artifact signatures remain mandatory; signature verification cannot be disabled.
- The verification public key is distributable configuration; the updater private signing key is secret trust material and must remain outside repository content/evidence.
- `createUpdaterArtifacts` remains the v2 bundling control for updater artifacts.
- Production updater endpoints are HTTPS/TLS-bound by default; insecure transport is an explicitly dangerous opt-in and is not an acceptable production default.
- Existing VSN update/helper code remains the starting boundary; Tauri updater adoption must not create competing version, trust, signature or rollback authorities.

Official reference:
- https://v2.tauri.app/plugin/updater/

## Canonical source audit — current main

The current repository already contains a meaningful updater/recovery primitive; PKG-04 should extend and certify it rather than create a second updater engine.

### `crates/vsn-update`

Current updater-core dependencies include `ed25519-dalek`, `sha2`, `base64`, `serde` and `serde_json`. There is no HTTP client dependency in this crate, which matches the current separation where the core verifies and applies a pre-downloaded artifact rather than owning remote discovery/download.

`UpdateManifest` currently binds `version`, `product`, `release`, `channel`, publication time, artifact metadata and an Ed25519 signature. Each artifact binds OS, architecture, HTTPS URL, SHA-256 and byte count. `verify_manifest` rejects unsupported/incomplete manifests, non-HTTPS artifact URLs and malformed SHA-256 values, then verifies the Ed25519 signature over the unsigned manifest fields. `verify_artifact` hashes the actual file bytes with SHA-256 and rejects mismatch.

The existing apply primitive is deliberately narrow and useful:
- `ApplyFileRequest` binds install root, safe relative target, staged artifact, expected SHA-256, release and explicit confirmation;
- apply requires `confirm_apply=true`;
- absolute, parent-traversal, root and platform-prefix target escapes are rejected;
- the canonical target parent must stay inside the canonical install root;
- the staged file is copied into `.vsn-update/pending`, fsynced and re-hashed before replacement;
- an existing target is moved to `.vsn-update/previous` before the pending file replaces it;
- replacement failure attempts restoration of the previous target;
- release state is persisted through a temporary state file and rename;
- the primitive does not download or execute the staged artifact and is explicitly intended for an out-of-process helper.

Rollback already exists with explicit confirmation. It moves the current target into `.vsn-update/failed-current`, restores the previous backup, attempts restoration of the current file if rollback replacement fails, and updates persisted state. Lock/status/stale-lock recovery primitives also exist around apply/rollback.

### `apps/updater-helper`

A dedicated `vsn-updater-helper` binary already wraps `vsn-update`. Its bounded stdin JSON protocol currently exposes exactly four operations: `apply`, `rollback`, `status` and `recover_lock`. The helper canonicalizes the install root and delegates to locked updater-core operations. Input is bounded to 2 MiB. Successful operations emit machine-readable JSON; top-level failures currently exit non-zero and write a plain error to stderr rather than returning the same structured response envelope.

This is a strong starting boundary for future tasks 04.05, 04.07 and 04.11, but bounded JSON stdin is not by itself an authorization boundary. Activation must explicitly freeze which trusted process may spawn/invoke the helper, how privilege/elevation is obtained, how request authenticity/identity is established, and how an untrusted local process is prevented from abusing a privileged helper.

### Desktop/Tauri current state

The current Desktop Rust dependencies include Tauri and `vsn-ipc`, but no `tauri-plugin-updater`. The current Desktop npm dependencies likewise contain no updater plugin package. Therefore Tauri updater is not an existing product authority today. Any future adoption is a new integration decision that must be reconciled during 04.01–04.03 and must not replace or compete with the VSN manifest/version/trust/rollback authority by accident.

## PKG-03 -> PKG-04 helper bootstrap boundary

The accepted PKG-03 Windows owned-payload contract deliberately excludes `apps/updater-helper and updater/recovery payloads`. Its exact durable executable ownership set is Desktop, CLI and Agent only, and the companion contract states that PKG-04 owns updater/recovery.

Therefore PKG-04 must not assume a trusted updater helper is already installed by the accepted Windows installer. Activation must explicitly choose and certify the helper bootstrap model. At minimum it must freeze:
- who owns placement/removal/repair of the helper binary;
- how the first trusted helper reaches an already-installed system;
- whether helper delivery is an installer revision, a signed embedded sidecar, a separately signed package component, or another single-authority mechanism;
- the exact helper path, ACL/owner/inheritance and code-signing identity;
- how the helper itself is upgraded without creating a circular self-replacement dependency;
- which component is allowed to launch/elevate it and how caller/request authenticity is proven;
- how uninstall/repair handles updater state, helper binaries and interrupted transactions without expanding ownership to user data.

This bootstrap choice must reconcile with the then-accepted PKG-03 signed package subjects/hashes and ownership semantics rather than silently broadening the old installer contract.

## Current updater-helper CI coverage baseline

The existing PKG-01 `01.11 Updater Helper Release` workflow is a useful artifact-release gate, but its accepted scope is intentionally narrower than future PKG-04 certification:
- it runs on `ubuntu-latest`;
- it builds `vsn-updater-helper` with Rust `1.97.1` and `--locked`;
- it verifies the release binary exists/is executable and emits SHA-256/file/evidence artifacts;
- it does not exercise Windows helper behavior, elevation, ACLs, reparse points, locked-file replacement or restart/reboot semantics;
- it does not run crash-phase/fault-injection recovery, lock-owner replacement, tampered-backup rollback, helper authorization or multi-component transaction tests.

PKG-04 should preserve this release evidence while adding Windows-native integration and deterministic fault-injection certification. A build-success signal alone must not be treated as proof of production updater safety.

## Second-pass failure-mode audit

This audit does not declare current PKG-01 primitives defective for their accepted narrow scope. It identifies failure modes that must be resolved before those primitives are promoted into a production remote/package updater under PKG-04.

### P0 — deterministic crash journal is missing

Current single-file apply has meaningful crash windows around `target -> backup`, `pending -> target`, and the later `state.json.tmp -> state.json` projection. A process/power failure after moving the old target but before installing the pending target can leave the canonical target absent. A failure after installing the new target but before projecting state can leave file reality newer than `state.json`. Stale-lock recovery only removes an old lock; it does not reconcile pending, backup, failed-current, target and state into a provable transaction outcome.

Activation requirement: introduce a transaction/journal state machine with durable phase markers and idempotent recovery. Recovery must distinguish at least prepared, old-target-staged, new-target-installed, state-committed, rollback-started and rollback-committed outcomes. Package-wide work must recover all participating components from the same transaction identity.

### P0 — lock recovery is time-based, not ownership-safe

`apply.lock` is created atomically with `create_new`, which is a useful exclusion primitive. However stale recovery permits deletion after ten minutes using age alone; it does not prove that the recorded process is dead. The lock record has PID/time/version but no unique ownership nonce/heartbeat. The guard's `Drop` removes the path unconditionally.

This creates a future package-update race: a long-running legitimate updater could exceed the stale threshold, an operator could explicitly recover its lock, a second updater could acquire a new lock at the same path, and the first guard could later drop and remove the second updater's lock.

Activation requirement: lock ownership must carry a cryptographically random or otherwise collision-resistant transaction/owner token and release must verify ownership before deletion. Recovery must require stronger stale evidence than elapsed time alone (platform liveness where reliable, heartbeat/lease semantics, or an equivalent deterministic ownership protocol). Long downloads must not hold an apply lock unnecessarily; download/staging and mutation locks should be separated if the architecture permits it.

### P0 — rollback backup identity is not re-verified before restore

Apply verifies the staged artifact before and after copy, but rollback restores `.vsn-update/previous/<name>.previous` without checking a persisted expected digest or signed artifact identity for that backup. `FileInstallState` stores release labels and target path, not the old/new artifact digests.

Activation requirement: transaction state must bind the exact previous and next artifact hashes, sizes, package/version identity and expected target. Rollback must verify the backup against recorded trusted identity immediately before restoration and fail closed on tamper/mismatch.

### P0 — rollback path containment is weaker than apply containment

Apply canonicalizes the target parent and verifies it remains under the canonical install root. Rollback reads `target_relative` from mutable state, runs lexical `safe_relative`, then joins it to the root without repeating the canonical-parent containment check used by apply. A symlink/reparse-point change in a target parent between apply and rollback must not allow rollback operations to resolve outside the install root.

Activation requirement: use one shared containment primitive for apply, rollback and recovery, with Windows reparse-point/symlink semantics explicitly tested. Treat updater state as untrusted input unless integrity and ACL guarantees are proven.

### P1 — previous rollback point is deleted before the new backup is secured

Before moving the current target to backup, apply removes an existing backup path. If the subsequent target-to-backup rename fails, the earlier rollback point has already been discarded. That may be acceptable for a deliberately one-level toy primitive, but it is not an acceptable package updater transaction policy without explicit evidence and state transition rules.

Activation requirement: rotate/replace backups transactionally. Never destroy the last known-good rollback point until the new rollback point is durably established and the policy explicitly authorizes retirement of the older one.

### P1 — backup/pending names are single-file scoped

Pending files are keyed by release and previous backups primarily by target file name. Different component paths with the same file name can collide once the updater becomes multi-component. A package transaction also needs per-component identity, not only one `target_relative` in state.

Activation requirement: namespace staging/backups by transaction ID + canonical component identity + target-path digest or another collision-resistant mapping. Persist a component manifest for the whole transaction.

### P1 — Windows crash durability and security-descriptor behavior require explicit proof

`fsync_file` syncs file contents. `fsync_dir` performs directory sync only on Unix and is intentionally a no-op on non-Unix platforms, including the primary Windows release target. Rust `set_permissions` also does not by itself establish a complete Windows ACL/owner/integrity-label inheritance contract.

Activation requirement: certify Windows rename/write-through/directory-metadata durability using Windows-native semantics where needed, and freeze exact ACL/owner/inheritance behavior for pending, backup, installed and failed-current artifacts. An update must not weaken the installed file's security descriptor.

### P1 — manifest freshness/channel/version policy is signed but not enforced here

Publication time and channel are signature-bound, but `verify_manifest` does not itself enforce freshness, monotonic version progression, allowed channel transition, expected current version or anti-replay state. That policy may correctly live above the cryptographic primitive, but PKG-04 must establish one authoritative enforcement layer before remote update activation.

Activation requirement: persist accepted update metadata and reject stale/replayed metadata, downgrade attempts and unauthorized channel transitions before download/apply. Explicit rollback is a separate operator-authorized path and must not weaken network anti-downgrade rules.

### P1 — artifact verification should scale to production package sizes

`verify_artifact` currently reads the complete artifact into memory before hashing. That is simple for current small binaries but is not the desired production behavior for larger installer/package artifacts.

Activation requirement: stream SHA-256 verification with bounded memory and validate the signed byte count as well as digest before mutation.

### P1 — structured helper failures are incomplete

The helper defines a response envelope containing `ok`, `result` and `error`, but failures currently bubble to `main`, print plain stderr and exit 1. Desktop/CLI recovery UX therefore cannot rely on one stable machine-readable response schema for both success and failure without an additional wrapper contract.

Activation requirement: freeze stable error codes/classes and a bounded structured response contract while retaining non-zero exit status for operational failure. Never put secret material or raw sensitive paths/tokens into error evidence.

## Source-to-PKG-04 gap map

The existing primitive materially reduces future implementation work, but it does not yet satisfy the 18-task package contract by itself.

Current gaps that remain intentionally future work:
- no accepted PKG-03 ownership/placement for updater-helper, so helper bootstrap/delivery must be owned by PKG-04;
- no frozen remote discovery/download/resume/cache pipeline feeding verified staging;
- no package-wide multi-component transaction across Agent, CLI, Desktop and updater helper;
- no complete service/process quiesce and locked-file orchestration around the single-file primitive;
- helper caller authentication/authorization/privilege contract is not yet frozen as PKG-04 authority;
- no current Desktop updater plugin/UX integration;
- CLI check/status/apply/rollback product workflow is not yet PKG-04 accepted;
- current rollback state is centered on one target/previous backup, not an accepted atomic package-wide transaction;
- anti-downgrade/replay/channel eligibility, remote endpoint behavior and key rotation still require activation-time policy freeze and end-to-end evidence;
- interrupted multi-component apply, reboot/restart and recovery semantics require package-level certification rather than assuming the single-file primitive proves them;
- transaction lock ownership, rollback backup verification, Windows durability/ACL semantics and symlink/reparse containment require explicit PKG-04 hardening;
- existing updater-helper release CI is Linux build/evidence oriented, not Windows behavioral/fault-injection certification.

## Activation mapping

Likely minimum-conflict mapping when PKG-04 is legitimately activated:
- `04.01`: reconcile exact accepted PKG-03 signed Windows subjects, install layout/service ownership, explicit updater-helper exclusion and the then-current updater source baseline;
- `04.02` / `04.03`: preserve or explicitly reconcile the existing Ed25519/SHA-256 manifest authority, then freeze channel/version/platform identity, endpoint/TLS, replay, downgrade and key-rotation policy plus helper bootstrap ownership;
- `04.04`: add bounded discovery/download/resume/cache as a layer that produces pre-downloaded, checksum-bound staging for the existing verification/apply core;
- `04.05` / `04.11`: harden the existing helper protocol, trusted bootstrap/invocation authority and lock/stale-lock lifecycle rather than inventing a parallel helper;
- `04.06`–`04.10`: build a package-level transaction coordinator around the existing single-file verified swap/restore primitive, with deterministic journal/recovery evidence;
- `04.12`: add Desktop update states only after the single VSN updater authority is frozen; Tauri updater adoption remains optional;
- `04.13`: add the CLI operator path against the same authority;
- `04.14`–`04.18`: certify eligibility/negative cases, installed Windows update/rollback, helper bootstrap/repair, provenance/handoff and exact-head final regression evidence.

## Activation-time freeze targets

04.01–04.03 should freeze:
- exact accepted PKG-03 Windows release subjects/hashes and installer/service ownership handoff;
- explicit updater-helper bootstrap, ownership, signing, repair and self-update model;
- version/channel/platform/artifact identity schema;
- update public-key identity and private-key custody boundary;
- endpoint/TLS trust policy and bounded failover semantics;
- anti-downgrade, replay and stale-metadata rejection;
- key rotation/recovery and lost-key operational response;
- exact updater/helper authority split;
- helper invocation/authentication/elevation boundary;
- transaction journal, crash-recovery and rollback ownership model;
- lock ownership/lease/recovery protocol;
- rollback backup identity/hash verification;
- Windows durability + ACL/reparse-point containment contract;
- stable helper error/result protocol;
- Windows-native behavioral and deterministic fault-injection CI/evidence matrix.

## Negative matrix carried forward

Future acceptance must fail closed for invalid/wrong-key signatures, replayed or downgraded metadata, corrupt/truncated/partial artifacts, mismatched hashes, interrupted resume state, stale/concurrent locks, recovered-live-lock attempts, lock-owner replacement races, endpoint/TLS policy violations, artifact substitution, install-root escape, reparse/symlink path escape, unauthorized helper invocation, missing/untrusted helper bootstrap, helper self-update interruption, tampered rollback backup, partial multi-component replacement, crash at every transaction phase, weakened installed-file ACLs and failed recovery/rollback.

## Stop conditions

Stop rather than implement if PKG-03 is not canonically COMPLETE, the accepted signed Windows subjects are unavailable, updater trust would require private material in mutable PR code, helper bootstrap/ownership cannot be made single-authority and signed, helper privilege cannot be bounded, lock ownership cannot be made deterministic, rollback backup identity cannot be verified, exact package transaction recovery cannot be evidenced, Windows durability/ACL/reparse semantics are unresolved, or adoption would create two competing trust/version/rollback authorities.
