# PKG-04..PKG-08 development preflight

Status: planning-only portfolio preflight.

## Preconditions checked

- Canonical `main`: `67e9a64da07ae36646cef7f95e343a069b4da5bf`.
- PKG-01 and PKG-02 are COMPLETE in master execution state.
- PKG-03 is the only next package and is being frozen separately in PR #106.
- PR #106 exact head has successful AI Planning Governance, Repository Governance and PKG-03 Acceptance Sequence runs.
- Remaining master denominators are fixed: 18 / 23 / 20 / 22 / 25.
- Existing updater primitives and Desktop bundling configuration were inspected before defining PKG-04/05.
- Current official Tauri distribution/updater requirements and current OWASP/NIST security baselines were reviewed.

## Allowed now

- Create/review planning artifacts for PKG-04..PKG-08.
- Create dormant Linear package parents and dependency relations.
- Run static plan/DAG validation.
- Refresh external research without product mutation.

## Forbidden now

- Change `.ai/state.json` away from PKG-03.
- Mark PKG-04..PKG-08 tasks active/DONE.
- Merge product mutations justified only by a downstream package.
- Add production signing/notarization/update private keys or credentials.
- Weaken accepted PKG-02 security/containment boundaries.
- Use a future-package plan to bypass PKG-03 acceptance.

## Activation preflight required for every package

Before `NN.01` can move from dormant to active:

1. predecessor package is COMPLETE on live canonical `main`;
2. predecessor final evidence/artifact digest is available;
3. all open product PRs affecting this package's surfaces are reconciled;
4. external requirements are delta-researched again;
5. prepared task wording/dependencies are reconciled without hidden denominator/order changes;
6. exact plan/manifest hashes are frozen;
7. package planning governance is green;
8. Linear parent/children match the frozen GitHub task IDs and blockers;
9. only then may the activation task project the package into canonical state.

## Package-specific preflight focus

### PKG-04 Updater & Recovery
Reconcile PKG-03 install roots, per-user/per-machine permissions, service lifecycle, MSI/NSIS artifact/version identity, signable boundary and mutable-state contract with existing `vsn-update`/`vsn-updater-helper` primitives.

### PKG-05 Linux + macOS Release
Reconcile PKG-04 update manifest/channel/artifact format; freeze supported distro/macOS/architecture baselines and code-signing/notarization secret boundaries before building release packages.

### PKG-06 Security Certification
Reconcile all accepted platform artifacts. Freeze threat model, security control matrix, scanner/tool versions and severity policy. Use NIST SSDF 1.1 as final baseline and applicable OWASP ASVS 5.0.0 controls; do not claim compliance beyond evidence.

### PKG-07 Production Resilience
Reconcile security-remediated candidate. Freeze measurable startup/recovery/resource/soak budgets and deterministic fault-injection methods before resilience changes.

### PKG-08 Pentest + Stable 1.0
Reconcile resilience-certified candidate. Freeze pentest rules, target hashes, severity model, independent retest requirements and stable-1.0 release candidate before testing/remediation.

## Stop conditions

Stop and classify rather than improvising when predecessor evidence is missing, a task needs new privileges/scope, a prepared assumption no longer matches live main, or an external platform requirement materially changed. Such changes require explicit plan/change-control before development.
