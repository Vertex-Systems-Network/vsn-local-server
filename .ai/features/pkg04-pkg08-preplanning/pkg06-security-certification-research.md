# PKG-06 Dormant Research — Security Certification

Reviewed: 2026-09-05
Canonical source audited: `79812eafdead24de88d8b3fafd19f1bfc0e1435c`
Status: **RESEARCH-ONLY / BLOCKED ON PKG-05 COMPLETE**

## Current baseline

The canonical 20-task PKG-06 sequence remains unchanged. This file does not activate security certification, widen scope, or claim compliance.

## Current standards findings

- NIST SP 800-218 Secure Software Development Framework (SSDF) Version 1.1 remains the final published NIST baseline reviewed for secure software-development practices.
- OWASP ASVS current stable release is 5.0.0 and remains suitable as an application-security verification requirements source where requirements actually apply to VSN's surfaces.
- Supply-chain provenance is evidence of build origin/integrity, not proof that the resulting software is secure; provenance verification must be combined with static, dynamic, abuse, dependency and trust-boundary testing.

Official references:
- https://csrc.nist.gov/pubs/sp/800/218/final
- https://owasp.org/www-project-application-security-verification-standard/
- https://docs.github.com/en/actions/concepts/security/artifact-attestations

## Canonical security source audit — current main

Current source already has substantial security primitives. PKG-06 should certify and attack these existing boundaries rather than assume security starts at 06.01.

### Authenticated local IPC

`vsn-ipc` currently uses authenticated loopback TCP at `127.0.0.1:39731`. Request/response envelopes are HMAC-bound, include protocol version, timestamp and request nonce, and enforce clock-skew/replay checks, bounded frame sizes, connection limits and response-to-request nonce binding.

This is meaningful prior control evidence, but PKG-06 must independently test replay-cache saturation, race/concurrency, clock manipulation, malformed/truncated frames, response substitution, local port pre-binding, hostile local clients, secret-store failure and service/user identity transitions on each accepted OS.

### Principal/permission model

`vsn-policy::Principal::local_authenticated()` is not unrestricted, but it is deliberately powerful. The current local IPC principal includes, among others:
- `RuntimeManage`;
- `ServiceManage`;
- `RemoteManage`;
- `TerminalExecute`;
- `FilesWrite`;
- `DatabaseWrite`;
- `SecretsUse` and `SecretsManage`.

It deliberately excludes several higher-risk authorities such as `TerminalAdmin`, `DatabaseDestructive`, `NetworkManage` and `SecretsReveal`. Remote delegated principals also reject permissions classified as high risk.

PKG-06 must therefore test actual command-to-permission mapping, not merely inspect enum names. A command that is reachable through the local principal must be treated as reachable from any process/webview that can legitimately obtain the local IPC secret or invoke the trusted Desktop bridge.

### Desktop/Tauri trust surface — P0 certification focus

The current Desktop Rust entry point exposes one generic Tauri command:

`agent_call(command: String, params: Value)` -> authenticated `vsn_ipc::call(command, params)`.

This keeps the Rust bridge small, but it intentionally forwards a caller-selected Agent command/parameters rather than exposing a narrowly typed Tauri command per operation. A compromised or unexpectedly scriptable webview could therefore attempt every Agent command available to the `local_authenticated` principal.

Current CSP is restrictive (`default-src 'self'`, restricted connect/img/script sources), but CSP is defense-in-depth and cannot be the sole authorization boundary. The current `src-tauri` tree also has no explicit `capabilities/` directory in its top-level source listing; activation must freeze the actual Tauri v2 generated/default capability surface and prove unused native/plugin APIs are unavailable.

06.11 must certify at minimum:
- frontend XSS/script-injection resistance and CSP bypass attempts;
- exact Tauri command exposure;
- hostile command-name and parameter mutation through `agent_call`;
- local-principal permission enforcement for every destructive/mutating command family;
- inability to reach `TerminalAdmin`, `NetworkManage`, destructive database, secret-reveal or equivalent privileged behavior indirectly through an allowed command;
- no frontend-controlled elevation or privilege-boundary confusion;
- stable redacted errors without leaking secrets/credentials.

If a generic bridge remains, its accepted command allowlist/permission semantics must be mechanically testable. If it cannot be bounded convincingly, 06.19 remediation should narrow the bridge rather than weakening Agent policy.

### Secret/key custody

`vsn-security` already uses Ed25519 device identity, HMAC IPC authentication, OS keyring storage on non-Windows and an ACL-protected Windows IPC-secret path. Device metadata is checked against derived public identity. This gives PKG-06 concrete custody primitives to test.

Certification must cover wrong/missing/tampered keyring entries, Windows ACL tamper attempts, key mismatch, backup/restore/copy between machines, user/service-context access, secret redaction and uninstall/repair/update interactions. Non-Windows service + interactive client custody identified by PKG-05 remains a prerequisite input to 06.05/06.13.

### Dependency / automated security tooling baseline

Current repository configuration includes Dependabot for Cargo, Rust toolchain and GitHub Actions. That is useful dependency-maintenance coverage.

The current source/workflow search performed for this preflight did not find an accepted repository lane named for `cargo audit`, CodeQL or Gitleaks. This is a pre-activation observation, not a permanent absence claim: 06.01/06.14/06.15 must re-audit fresh main and freeze exact versions/configuration for dependency advisory scanning, static analysis, secret scanning and any unsafe-code review tooling actually selected.

Security tools must not be added merely to maximize tool count; each must own a distinct finding class, run deterministically enough for evidence, and have an explicit false-positive/suppression governance path.

### Existing CI evidence is not certification

Current package CI already has numerous negative matrices, containment tests, exact-head checks and signed-evidence flows. These are valuable inherited evidence, but they remain task-scoped acceptance. PKG-06 must replay the security-relevant claims against the exact accepted PKG-05 release candidate and attack cross-boundary composition failures that individual package tests may not cover.

## Activation-time applicability rules

06.01/06.02 should freeze an applicability matrix before testing. Every external framework requirement must be classified as applicable, not applicable with rationale, or covered by an equivalent accepted control/evidence path. Framework names must not be used as blanket certification claims.

Security evidence should bind exact release-candidate hashes and cover at minimum:
- authenticated IPC/session/replay boundaries;
- authorization and workspace/resource containment;
- secrets/key/log-redaction boundaries;
- filesystem/archive/symlink/integrity abuse;
- direct terminal/pipe/PTY injection and resource abuse;
- preview/DNS/domain/HTTPS SSRF/rebinding/trust boundaries;
- database mutation/query/credential/capability boundaries;
- runtime/bootstrap/service/container trust boundaries;
- Desktop/Tauri/webview/CSP/capability surfaces;
- installer/updater/signing/rollback/TOCTOU boundaries;
- Linux/macOS packaging/signing/notarization permissions;
- dependency/SBOM/provenance/supply-chain verification.

## Source-to-PKG-06 gap map

Current future certification gaps include:
- no frozen whole-system threat model tied to exact accepted PKG-05 artifacts;
- no certification-wide command-to-permission reachability matrix;
- generic Desktop `agent_call` bridge requires explicit webview-to-Agent abuse certification;
- no frozen static-analysis/advisory/secret-scan toolchain in this preflight baseline;
- no package-wide unsafe-code/native-command execution inventory;
- no accepted cross-OS secure-store/service-context abuse matrix;
- no certification-wide fuzz/property/malformed-protocol campaign;
- no final NIST SSDF/ASVS applicability evidence matrix;
- no final remediation ledger proving zero unresolved Critical/High findings in frozen scope.

## Activation mapping

Likely minimum-conflict mapping when PKG-06 is legitimately activated:
- `06.01`: bind exact PKG-05 release hashes, accepted OS matrix, source SHA, SBOM/provenance and security-tool versions;
- `06.02`: produce system threat model, trust-boundary graph and framework applicability map;
- `06.03`–`06.13`: certify existing boundaries independently, scheduling max five dependency-ready slices at once;
- `06.11`: treat Desktop generic Agent bridge + local principal as a dedicated attack path, not only a CSP checklist;
- `06.14`: verify dependencies/SBOM/provenance against exact artifacts;
- `06.15`: freeze and run static analysis, unsafe/native review and secret-scanning baseline;
- `06.16`: run malformed/fuzz/abuse/property tests targeted by the threat model and earlier findings;
- `06.17`: verify audit/security-event integrity and sensitive-data exclusion;
- `06.18`: map accepted evidence to NIST SSDF 1.1 and applicable ASVS 5.0.0 requirements with explicit exceptions;
- `06.19`: remediate at owning implementation layer and invalidate/re-run all stale evidence affected by each fix;
- `06.20`: exact-head final gate only when candidate hashes and finding ledger are frozen.

## Negative matrix carried forward

Future certification must include invalid/replayed/expired IPC frames, replay-window saturation, malformed frames, response substitution, hostile local port ownership, stolen/wrong IPC secret, secure-store unavailability, user/service identity mismatch, generic Desktop bridge command mutation, indirect privilege escalation through allowed local commands, terminal/files/database abuse, filesystem traversal/reparse/symlink attacks, SSRF/rebinding/header abuse, database injection/capability bypass, installer/updater TOCTOU, signing/provenance substitution, dependency/advisory failures, secret leakage in logs/evidence and malformed security-event input.

## Acceptance discipline

Zero unresolved Critical/High findings in frozen scope remains a final remediation target, but severity alone cannot suppress a reproducible trust-boundary failure. Fixes must return to the smallest owning implementation package/task and invalidate stale evidence for changed subjects.

## Stop conditions

Stop if PKG-05 is not canonically COMPLETE, the release candidate changes during certification without re-binding evidence, a framework/version change materially alters scope, command/permission reachability cannot be made deterministic, security-tool suppressions are ungoverned, or certification would require weakening an earlier accepted control to obtain a green result.
