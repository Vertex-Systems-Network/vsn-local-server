# PKG-04 Dormant Research — Updater & Recovery

Reviewed: 2026-09-05
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

## Activation-time freeze targets

04.01–04.03 should freeze:
- exact accepted PKG-03 Windows release subjects/hashes and installer/service ownership handoff;
- version/channel/platform/artifact identity schema;
- update public-key identity and private-key custody boundary;
- endpoint/TLS trust policy and bounded failover semantics;
- anti-downgrade, replay and stale-metadata rejection;
- key rotation/recovery and lost-key operational response;
- exact updater/helper authority split.

## Negative matrix carried forward

Future acceptance must fail closed for invalid/wrong-key signatures, replayed or downgraded metadata, corrupt/truncated/partial artifacts, mismatched hashes, interrupted resume state, stale/concurrent locks, endpoint/TLS policy violations and artifact substitution.

## Stop conditions

Stop rather than implement if PKG-03 is not canonically COMPLETE, the accepted signed Windows subjects are unavailable, updater trust would require private material in mutable PR code, or adoption would create two competing trust/version authorities.
