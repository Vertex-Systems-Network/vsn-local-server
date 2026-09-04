# PKG-06 Dormant Research — Security Certification

Reviewed: 2026-09-05
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

## Activation-time mapping rules

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

## Acceptance discipline

Zero unresolved Critical/High findings in frozen scope remains a final remediation target, but severity alone cannot suppress a reproducible trust-boundary failure. Fixes must return to the smallest owning implementation package/task and invalidate stale evidence for changed subjects.

## Stop conditions

Stop if PKG-05 is not canonically COMPLETE, the release candidate changes during certification without re-binding evidence, a framework/version change materially alters scope, or certification would require weakening an earlier accepted control to obtain a green result.
