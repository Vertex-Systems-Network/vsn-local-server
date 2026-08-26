# PKG-04..PKG-08 parallel preplanning research

Status: planning-only. Canonical execution state remains PKG-03. This artifact does not activate or complete any PKG-04..PKG-08 task.

Canonical research base: `67e9a64da07ae36646cef7f95e343a069b4da5bf`.

## Repository facts

- `docs/MASTER-EXECUTION-PLAN.md` fixes the remaining denominators: PKG-04=18, PKG-05=23, PKG-06=20, PKG-07=22, PKG-08=25.
- Master execution remains package-sequential: later work may be prepared but cannot be counted DONE before prerequisites.
- `apps/updater-helper` already exists. Its bounded stdin JSON protocol exposes apply, rollback, status and stale-lock recovery through `vsn_update`.
- `crates/vsn-update` already verifies Ed25519-signed manifests, HTTPS artifact URLs, SHA-256 artifact digests, safe relative update targets, install-root containment, staged replacement, previous-file backup, rollback state and an update lock. PKG-04 therefore certifies and extends an existing updater boundary rather than replacing it with an unrelated updater.
- `apps/desktop/src-tauri/tauri.conf.json` currently has bundling enabled for all targets, product version `0.38.1`, and a restrictive CSP, but no updater plugin configuration. PKG-04 must make an explicit single-source integration decision rather than silently creating two independent update authorities.
- Accepted PKG-02 capabilities include authenticated IPC, workspace/file/binary/terminal/PTY, preview, DNS/domain/HTTPS, SQLite/native DB adapters, runtime/service/container paths and Desktop/CLI bridges. PKG-06..08 must test these accepted boundaries rather than inventing a second product architecture.

## Current external constraints reviewed 2026-08-26

### Tauri v2 updater

Official Tauri v2 updater documentation states that update signatures are mandatory and cannot be disabled. Tauri supports Windows, Linux and macOS updater targets; production endpoints use TLS. On Windows the documented updater install modes include passive/basic UI/quiet, with quiet unable to request elevation by itself. `createUpdaterArtifacts` can generate platform-specific signed update bundles.

Implication: PKG-04 may integrate Tauri's Desktop updater only if it remains subordinate to one frozen VSN update trust/version/channel policy. Existing `vsn-update` Ed25519 verification cannot be weakened or bypassed merely to adopt a frontend plugin.

Reference: https://v2.tauri.app/plugin/updater/

### Linux distribution

Tauri v2 documents Linux distribution through AppImage, Debian, RPM and other formats. Its Debian guidance warns that glibc compatibility is determined by the build baseline and recommends building on the oldest supported system; Ubuntu 22.04 and Debian 12 are cited as suitable WebKitGTK 4.1 baselines.

Implication: PKG-05 must freeze an explicit supported OS/architecture matrix before packaging, and CI artifacts must be built on compatible baselines rather than assuming a newer runner is portable.

References:
- https://v2.tauri.app/distribute/
- https://v2.tauri.app/distribute/appimage/
- https://v2.tauri.app/distribute/debian/

### macOS distribution

Tauri v2 documents direct macOS distribution through an app bundle/DMG and states that code signing is required and direct distribution outside the App Store also requires notarization.

Implication: PKG-05 owns the signable/notarizable artifact boundary and secretless CI contract. Signing credentials remain external secret handles; repository planning/evidence must never embed private keys, certificates or notarization credentials.

Reference: https://v2.tauri.app/distribute/

### Security verification baseline

OWASP ASVS latest stable is 5.0.0. It is web-application oriented, so PKG-06 uses only applicable controls for the Tauri/webview/dashboard/API-like surfaces and records non-applicable controls; it must not claim blanket ASVS compliance for native desktop/service behavior.

NIST SP 800-218 SSDF 1.1 remains the final baseline. NIST lists SP 800-218 Rev.1 / SSDF 1.2 as a draft (released 2025-12-17). PKG-06 therefore maps evidence to final SSDF 1.1 and records relevant 1.2 draft deltas as research only unless the revision becomes final before activation.

References:
- https://owasp.org/www-project-application-security-verification-standard/
- https://csrc.nist.gov/pubs/sp/800/218/final
- https://csrc.nist.gov/projects/ssdf/publications

## Parallelism conclusion

There are two distinct kinds of parallelism:

1. **Planning parallelism — allowed now.** PKG-04, PKG-05, PKG-06, PKG-07 and PKG-08 can all be researched and preplanned concurrently because this does not mutate product state or claim acceptance.
2. **Implementation/acceptance parallelism — package-gated.** Canonical product execution remains PKG-03 -> PKG-04 -> PKG-05 -> PKG-06 -> PKG-07 -> PKG-08. Within an activated package, its frozen DAG can expose up to five independent runnable tasks.

This avoids two failure modes: silently advancing a later package before its predecessor is accepted, and delaying all future architecture work until the last moment.

## Preplanning rule

The task IDs and denominators in this portfolio plan are prepared early to make work resumable. Before each package activates, development must perform fresh delta research against live canonical `main`, reconcile the predecessor handoff, and pass package planning governance. If upstream implementation invalidates a prepared acceptance statement, update it through change control without silently changing the denominator/order.
