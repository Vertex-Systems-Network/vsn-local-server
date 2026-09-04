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

A dedicated `vsn-updater-helper` binary already wraps `vsn-update`. Its bounded stdin JSON protocol currently exposes exactly four operations: `apply`, `rollback`, `status` and `recover_lock`. The helper canonicalizes the install root and delegates to locked updater-core operations. Input is bounded to 2 MiB and responses are machine-readable JSON.

This is a strong starting boundary for future tasks 04.05, 04.07 and 04.11, but bounded JSON stdin is not by itself an authorization boundary. Activation must explicitly freeze which trusted process may spawn/invoke the helper, how privilege/elevation is obtained, how request authenticity/identity is established, and how an untrusted local process is prevented from abusing a privileged helper.

### Desktop/Tauri current state

The current Desktop Rust dependencies include Tauri and `vsn-ipc`, but no `tauri-plugin-updater`. The current Desktop npm dependencies likewise contain no updater plugin package. Therefore Tauri updater is not an existing product authority today. Any future adoption is a new integration decision that must be reconciled during 04.01–04.03 and must not replace or compete with the VSN manifest/version/trust/rollback authority by accident.

## Source-to-PKG-04 gap map

The existing primitive materially reduces future implementation work, but it does not yet satisfy the 18-task package contract by itself.

Current gaps that remain intentionally future work:
- no frozen remote discovery/download/resume/cache pipeline feeding verified staging;
- no package-wide multi-component transaction across Agent, CLI, Desktop and updater helper;
- no complete service/process quiesce and locked-file orchestration around the single-file primitive;
- helper caller authentication/authorization/privilege contract is not yet frozen as PKG-04 authority;
- no current Desktop updater plugin/UX integration;
- CLI check/status/apply/rollback product workflow is not yet PKG-04 accepted;
- current rollback state is centered on one target/previous backup, not an accepted atomic package-wide transaction;
- anti-downgrade/replay/channel eligibility, remote endpoint behavior and key rotation still require activation-time policy freeze and end-to-end evidence;
- interrupted multi-component apply, reboot/restart and recovery semantics require package-level certification rather than assuming the single-file primitive proves them.

## Activation mapping

Likely minimum-conflict mapping when PKG-04 is legitimately activated:
- `04.01`: reconcile exact accepted PKG-03 signed Windows subjects, install layout/service ownership and the then-current updater source baseline;
- `04.02` / `04.03`: preserve or explicitly reconcile the existing Ed25519/SHA-256 manifest authority, then freeze channel/version/platform identity, endpoint/TLS, replay, downgrade and key-rotation policy;
- `04.04`: add bounded discovery/download/resume/cache as a layer that produces pre-downloaded, checksum-bound staging for the existing verification/apply core;
- `04.05` / `04.11`: harden the existing helper protocol, invocation authority and lock/stale-lock lifecycle rather than inventing a parallel helper;
- `04.06`–`04.10`: build a package-level transaction coordinator around the existing single-file verified swap/restore primitive, with deterministic journal/recovery evidence;
- `04.12`: add Desktop update states only after the single VSN updater authority is frozen; Tauri updater adoption remains optional;
- `04.13`: add the CLI operator path against the same authority;
- `04.14`–`04.18`: certify eligibility/negative cases, installed Windows update/rollback, provenance/handoff and exact-head final regression evidence.

## Activation-time freeze targets

04.01–04.03 should freeze:
- exact accepted PKG-03 Windows release subjects/hashes and installer/service ownership handoff;
- version/channel/platform/artifact identity schema;
- update public-key identity and private-key custody boundary;
- endpoint/TLS trust policy and bounded failover semantics;
- anti-downgrade, replay and stale-metadata rejection;
- key rotation/recovery and lost-key operational response;
- exact updater/helper authority split;
- helper invocation/authentication/elevation boundary;
- transaction journal, crash-recovery and rollback ownership model.

## Negative matrix carried forward

Future acceptance must fail closed for invalid/wrong-key signatures, replayed or downgraded metadata, corrupt/truncated/partial artifacts, mismatched hashes, interrupted resume state, stale/concurrent locks, endpoint/TLS policy violations, artifact substitution, install-root escape, unauthorized helper invocation, partial multi-component replacement and failed recovery/rollback.

## Stop conditions

Stop rather than implement if PKG-03 is not canonically COMPLETE, the accepted signed Windows subjects are unavailable, updater trust would require private material in mutable PR code, helper privilege cannot be bounded, exact package transaction recovery cannot be evidenced, or adoption would create two competing trust/version/rollback authorities.
