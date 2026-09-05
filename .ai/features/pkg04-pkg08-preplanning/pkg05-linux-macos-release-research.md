# PKG-05 Dormant Research — Linux + macOS Release

Reviewed: 2026-09-05
Canonical source audited: `79812eafdead24de88d8b3fafd19f1bfc0e1435c`
Status: **RESEARCH-ONLY / BLOCKED ON PKG-04 COMPLETE**

## Current baseline

The canonical 23-task PKG-05 sequence remains unchanged. No Linux/macOS release implementation, signing, notarization, package-state projection or secret provisioning is authorized here.

## Current-doc findings

### macOS

Apple's current direct-distribution guidance continues to require an appropriate Developer ID signature for software distributed outside the Mac App Store. Notarized apps require Hardened Runtime, secure timestamping and valid signatures. Apple documents `notarytool` / the Notary API for automated notarization and supports stapling the returned ticket to distributed software.

Official references:
- https://developer.apple.com/documentation/security/hardened-runtime
- https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution
- https://developer.apple.com/documentation/security/customizing-the-notarization-workflow

### Linux

Linux acceptance must remain package-format and compatibility explicit. Activation must freeze supported distro/version/architecture matrix, glibc/WebKitGTK baseline, AppImage/DEB/RPM identities, package dependencies, install/service ownership and deterministic artifact naming/hashes before acceptance claims.

## Canonical source audit — current main

The current repository has useful cross-platform source primitives, but it does not yet contain accepted Linux/macOS distribution authority.

### CI and packaging surface

- `installer/` currently contains only `installer/windows/`; there is no accepted Linux or macOS installer/package ownership tree.
- Current PKG-01 foundation CI runs on Ubuntu and installs GTK/WebKitGTK/appindicator/Rsvg/OpenSSL/patchelf dependencies before workspace Clippy/tests. This is useful Linux source-compatibility evidence, but it does not build or certify AppImage, DEB or RPM lifecycle artifacts.
- The current workflow set contains no macOS-hosted release/certification lane. Therefore macOS compilation, app bundle, hardened runtime, signing, notarization, stapling, Gatekeeper and launchd claims remain entirely future PKG-05 work.
- Current Desktop Tauri configuration sets `bundle.targets = "all"`, but accepted CI is presently Windows-installer-centric and there is no deterministic PKG-05 artifact manifest for Linux/macOS. Generic Tauri target capability must not be treated as accepted cross-platform release evidence.

### Desktop bundle assets/configuration

Current `tauri.conf.json` is dominated by the accepted Windows release contract: publisher, Windows downgrade policy, WiX upgrade code and NSIS current-user mode. The configured bundle icon list currently names only `icons/icon.ico`.

The source tree also contains `icons/icon.png`, but no `.icns` asset is present in the current icon directory. PKG-05 must explicitly freeze Linux and macOS icon/bundle metadata, application/category identity, minimum OS/runtime baseline and any platform-specific Tauri configuration rather than relying on Windows configuration to project correctly onto other targets.

### Agent/service portability

`vsn-agent` is largely portable Rust, but its only explicit service-manager dependency is `windows-service` behind `cfg(windows)`. The executable can run in foreground mode on non-Windows platforms, yet there is no current accepted systemd unit, launchd plist, Linux/macOS service install/remove lifecycle or package ownership contract.

This means tasks 05.07 and 05.15 are real platform integrations, not simple repackaging of the Windows service implementation. They must freeze user/system service scope, executable/config/state ownership, restart behavior, privilege boundary, log destination and clean removal semantics independently for systemd and launchd.

### Non-Windows secure-store / IPC boundary — P0 activation risk

`vsn-security` already has a non-Windows portability path: device identity and IPC secrets use the OS `keyring` abstraction, while application data paths come from `ProjectDirs`. `vsn-ipc` uses authenticated loopback TCP at `127.0.0.1:39731`, so there is no Windows-only named-pipe dependency to replace.

However PKG-05 must not assume this automatically works once the Agent becomes a systemd/launchd service. On non-Windows, the Agent service context and interactive Desktop/CLI context may have different users, login sessions, credential-store availability or keyring namespaces. If they cannot resolve the same IPC HMAC secret, an otherwise healthy service and Desktop/CLI will fail mutual IPC authentication.

Activation requirement: 05.02–05.03 must freeze the non-Windows service/user identity model and IPC-secret custody model before 05.07/05.15. Acceptance must prove that the chosen service scope and interactive clients can access exactly the intended shared authentication material without broadening secret permissions. If the OS keyring cannot safely satisfy both contexts, PKG-05 must define a platform-native alternative rather than weakening IPC authentication.

Negative cases must include locked/unavailable keyring, headless service startup, logout/login, user switch, service restart, missing desktop session and incorrect service/user ownership.

### Linux `linux-native` keyutils reboot persistence — direct P0 lifecycle blocker

Fresh dependency/source audit tightens the Linux risk from a generic service-context concern to a concrete reboot-continuity problem in the current dependency selection.

Current repo facts:
- `vsn-security` requests `keyring = 3.6.3` with feature `linux-native` on non-macOS Unix targets;
- in keyring 3.6.3, `linux-native` selects the Linux kernel keyutils store, while the separate `linux-native-*-persistent` feature combinations add Secret Service persistence;
- VSN stores both `device-ed25519-v1` and `local-ipc-hmac-v1` through `Entry::new("vsn-agent", entry_name)` on non-Windows;
- `load_or_create_secret()` treats `keyring::Error::NoEntry` as first-run state and creates a new random secret;
- `device.json` is independently persisted under `ProjectDirs` and `DeviceIdentity::load_or_create()` rejects a regenerated private key when its derived public key/device ID no longer matches that persisted metadata.

The keyutils backend is memory-backed and upstream documentation explicitly requires clients to handle credentials disappearing; reboot clears the kernel keyrings. Therefore, with the current Linux feature selection, a normal reboot can remove the keyring-backed device private key and IPC HMAC secret while leaving `device.json` on disk. On next startup VSN can generate a new device private key, then fail closed with `IdentityMismatch` against the old persisted metadata; the IPC secret can also rotate, breaking continuity with any client/context that retained or expected the prior credential.

This is not an accepted production defect claim for a Linux release because PKG-05 is not activated and no Linux package is accepted. It is a **release-blocking activation finding**: the current secure-store choice cannot be promoted to an accepted persistent Linux Agent identity without a deliberate custody/persistence design.

Activation must choose and certify one coherent Linux model, for example a persistent Secret Service combination only where headless/service availability and least privilege are proven, or a dedicated machine/service-owned encrypted credential location with explicit ACL/ownership and migration semantics. The choice must not silently fall back to plaintext, weaken IPC authentication, or rotate device identity merely because the machine rebooted.

Linux negative/lifecycle acceptance must include:
- first boot -> identity creation -> reboot -> headless Agent start with exact same device ID/public key;
- exact IPC HMAC continuity across reboot for the intended Agent/Desktop/CLI principals;
- kernel keyring empty/expired while metadata remains;
- metadata missing while credential remains;
- credential missing while metadata remains;
- service-user change and interactive-user login after reboot;
- Secret Service unavailable/locked if a persistent Secret Service design is selected;
- package repair/update and uninstall/reinstall according to the frozen identity-retention policy.

A green compile/test on GitHub-hosted Ubuntu does not prove this lifecycle because a fresh runner does not exercise same-machine reboot persistence.

### Device identity continuity across service/user contexts — additional P0 risk

Fresh source audit shows the same service-context issue applies to machine identity, not only the IPC HMAC secret. `DeviceIdentity::load_or_create()` stores its Ed25519 private key through the same `keyring` abstraction and stores `device.json` metadata under the `ProjectDirs` local data path. `vsn_core::local_machine_identity()` directly derives the machine-facing `device_id` and public key from that `DeviceIdentity`.

The Agent calls `local_machine_identity()` during initialization/startup, uses that device ID in audit events and binds IPC command audit targets to the Agent device ID. Therefore moving the Agent from an interactive foreground user into a systemd/launchd service account can create a different keyring namespace/data directory and silently create a second device identity unless the service/user custody model explicitly prevents it.

Activation requirements:
- freeze whether VSN device identity is truly machine-scoped or service-user-scoped on Linux/macOS;
- prove install -> first service start -> Desktop/CLI connection -> logout/login -> service restart -> package update preserves the exact expected `device_id`/public-key identity;
- prohibit accidental duplicate identities merely because the same executable starts under a different user/session;
- define migration/repair for an existing foreground-created identity when enabling the system service;
- keep the private signing key least-privilege while allowing only the component that genuinely owns device signing to use it;
- ensure audit-chain continuity and remote/device registration semantics remain bound to the same accepted identity across service lifecycle events.

Negative cases must include service-user change, user migration, keyring entry missing for one context, metadata present but private key absent, private key present but metadata from another context, and package reinstall/repair without unintended identity rotation.

### Runtime/library compatibility

The Ubuntu foundation lane proves the source can currently be linted/tested with one GitHub-hosted Ubuntu environment and its installed development packages. It does not freeze the minimum supported glibc/WebKitGTK/libayatana/OpenSSL/runtime dependency matrix for distributed Linux binaries.

PKG-05 must derive runtime dependencies from produced artifacts/packages and validate them on the oldest supported clean target, not merely the build image. AppImage portability, DEB dependency declarations and RPM dependency declarations require separate evidence.

### macOS architecture/signing boundary

No current source authority freezes x86_64 vs aarch64 vs universal output. No current repository workflow is authoritative for Developer ID signing/notarization, and no repository content should ever contain the private signing credential.

05.12–05.18 must bind exact architecture strategy, exact `.app`/DMG/update subjects, Developer ID identity, hardened-runtime/entitlement contract, secure timestamp, notarization submission/result/log identity, stapled ticket verification and final Gatekeeper-relevant verification to the exact accepted artifact hashes.

## Source-to-PKG-05 gap map

Current gaps that remain intentionally future work:
- no accepted Linux/macOS installer/package ownership tree;
- no frozen distro/macOS-version/architecture compatibility matrix;
- no deterministic Linux AppImage/DEB/RPM build/lifecycle authority;
- no accepted systemd service install/remove lifecycle;
- no accepted launchd service install/remove lifecycle;
- current Linux `linux-native` keyutils credential backend does not provide reboot persistence suitable for an accepted persistent Agent identity without redesign/reconciliation;
- no proven cross-context non-Windows IPC-secret custody model for service + Desktop/CLI;
- no proven machine/device identity continuity model across foreground/service users and sessions;
- no macOS build/sign/notarize/staple/Gatekeeper CI authority;
- no `.icns` bundle asset or frozen platform-specific icon/metadata contract;
- no accepted universal-binary decision/evidence;
- no Linux runtime dependency certification on oldest supported clean targets;
- no PKG-04 updater parity evidence against accepted Linux/macOS artifacts;
- no cross-platform release manifest/SBOM/provenance matrix bound to exact accepted bytes.

## Activation mapping

Likely minimum-conflict mapping when PKG-05 is legitimately activated:
- `05.01`: reconcile exact accepted PKG-04 update manifest/recovery authority and current cross-platform source baseline;
- `05.02`: freeze OS versions, distro families, CPU architectures, release channels and oldest-supported compatibility targets;
- `05.03`: freeze immutable/mutable/config/data/cache/log layout plus service-user, IPC-secret custody and machine-identity custody models, including Linux reboot-persistent secure-store semantics;
- `05.04` / `05.12`: produce locked platform Rust payloads and exact architecture identities;
- `05.05`: build Linux Desktop against the frozen WebKitGTK/glibc baseline and derive actual runtime requirements;
- `05.06`–`05.10`: implement Linux CLI/PATH, systemd, AppImage, DEB and RPM against one ownership model;
- `05.11`: clean Linux lifecycle matrix on actual minimum supported targets, including same-machine reboot + exact device-ID/IPC-secret continuity;
- `05.13`–`05.18`: freeze macOS bundle metadata/assets, hardened runtime, CLI/PATH, launchd, DMG, Developer ID signing, notarization/stapling and lifecycle evidence;
- `05.19`: exercise the single accepted PKG-04 updater authority over exact Linux/macOS release subjects;
- `05.20`–`05.22`: bind checksums/SBOM/provenance, reproducible CI identities and clean runner/VM end-to-end evidence;
- `05.23`: exact-head final gate and PKG-06 handoff only after all accepted artifact identities are mechanically reconstructable.

## Activation-time freeze targets

05.02–05.03 should freeze:
- exact Linux distro/version/architecture and macOS version/architecture matrix;
- glibc/WebKitGTK/runtime dependency baseline;
- immutable vs mutable path ownership per platform;
- Linux systemd scope/user/group/unit identity;
- macOS launchd domain/label/user identity;
- Linux reboot-persistent credential backend and recovery/migration behavior;
- non-Windows IPC secret/keyring ownership and service/interactive access model;
- device-identity key + metadata ownership, service/user scope and migration/repair rules;
- CLI PATH/shell integration ownership and rollback/removal rules;
- package/application identifiers and deterministic artifact naming;
- Linux/macOS icon/bundle metadata and platform config split;
- updater/helper placement and handoff from PKG-04.

05.12–05.18 should additionally freeze:
- x86_64/aarch64/universal architecture strategy;
- exact produced `.app`, DMG and updater subjects;
- Developer ID certificate type/identity and external credential custody;
- minimum-scope hardened-runtime entitlements/exceptions;
- secure timestamp expectations;
- notarization submission/result/log identity;
- ticket/stapling verification;
- Gatekeeper-relevant verification and final artifact hashes;
- mutable-state preservation and clean uninstall boundaries.

## Negative matrix carried forward

Future acceptance must fail closed for unsupported OS/architecture, missing runtime library, incompatible glibc/WebKitGTK baseline, broken package dependency declarations, service-user/permission mismatch, inaccessible/wrong keyring secret, Linux reboot with keyutils credentials cleared while persistent metadata remains, IPC auth failure between service and interactive client, headless startup failure, duplicate/rotated device identity caused solely by reboot/service-user/session changes, identity metadata/private-key mismatch, stale service state after uninstall, PATH pollution, package ownership collision, wrong bundle identity, unsigned/adhoc-signed macOS release, hardened-runtime/entitlement mismatch, failed notarization, unstapled/invalid ticket where required, Gatekeeper rejection, wrong architecture slice, artifact hash substitution and updater handoff to bytes other than the exact accepted release subject.

## Exact-artifact rule

Signing, notarization or packaging evidence must remain bound to the exact accepted artifact bytes. A rebuilt or differently hashed app/DMG/package may be comparison evidence only and cannot silently substitute for an accepted release subject.

## Stop conditions

Stop if PKG-04 is not canonically COMPLETE, Apple requirements materially change, signing/notarization credentials would enter repository evidence, the Linux compatibility baseline cannot be made explicit, Linux credential persistence cannot survive the frozen lifecycle without identity/IPC-secret rotation, service/interactive IPC-secret custody cannot be made least-privilege and deterministic, device-identity custody/continuity cannot be made deterministic across service lifecycle, the updater handoff cannot bind the same accepted release subjects, or a package/service layout would create competing platform authorities.
