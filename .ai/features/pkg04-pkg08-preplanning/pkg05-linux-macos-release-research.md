# PKG-05 Dormant Research — Linux + macOS Release

Reviewed: 2026-09-05
Status: **RESEARCH-ONLY / BLOCKED ON PKG-04 COMPLETE**

## Current baseline

The canonical 23-task PKG-05 sequence remains unchanged. No Linux/macOS release implementation, signing, notarization, package-state projection or secret provisioning is authorized here.

## macOS current-doc findings

Apple's current direct-distribution guidance continues to require an appropriate Developer ID signature for software distributed outside the Mac App Store. Notarized apps require Hardened Runtime, secure timestamping and valid signatures. Apple documents `notarytool` / the Notary API for automated notarization and supports stapling the returned ticket to distributed software.

Official references:
- https://developer.apple.com/documentation/security/hardened-runtime
- https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution
- https://developer.apple.com/documentation/security/customizing-the-notarization-workflow

## Activation-time macOS freeze targets

05.12–05.18 should freeze:
- architecture strategy and exact produced app/DMG/update subjects;
- Developer ID certificate type/identity and external credential custody;
- minimum-scope hardened-runtime entitlements/exceptions;
- secure timestamp expectations;
- notarization submission/result/log identity;
- ticket/stapling verification;
- Gatekeeper-relevant verification and final artifact hashes;
- mutable-state preservation and clean uninstall boundaries.

## Linux release boundary

Linux implementation remains package-format and compatibility explicit. Activation must freeze supported distro/version/architecture matrix, baseline runtime/library assumptions, AppImage/DEB/RPM identities, service/install ownership and deterministic package naming/hashes before acceptance claims.

## Exact-artifact rule

Signing, notarization or packaging evidence must remain bound to the exact accepted artifact bytes. A rebuilt or differently hashed app/DMG/package may be comparison evidence only and cannot silently substitute for an accepted release subject.

## Stop conditions

Stop if PKG-04 is not canonically COMPLETE, Apple requirements materially change, signing/notarization credentials would enter repository evidence, the Linux compatibility baseline cannot be made explicit, or the updater handoff cannot bind the same accepted release subjects.
