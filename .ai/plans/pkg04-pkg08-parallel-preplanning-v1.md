# PKG-04..PKG-08 Parallel Preplanning v1

Status: PREPARED / DORMANT. This is a future-package execution map, not canonical activation.

Canonical base: `67e9a64da07ae36646cef7f95e343a069b4da5bf`

## Portfolio invariants

- Fixed remaining denominator: **108 tasks** = PKG-04 18 + PKG-05 23 + PKG-06 20 + PKG-07 22 + PKG-08 25.
- Canonical activation order remains `PKG-03 -> PKG-04 -> PKG-05 -> PKG-06 -> PKG-07 -> PKG-08`.
- All five downstream packages may be planned/researched in parallel now; no downstream product task is active or DONE now.
- Within one activated package, at most five dependency-ready implementation tasks may run concurrently.
- Every package begins with `NN.01` activation/reconciliation and ends with an exact-head final gate.
- Before activation, live-main delta research may refine acceptance wording but must not silently alter denominator/order/task IDs.
- No package may self-approve material scope expansion, new privilege, secret handling or master-sequence change.

# PKG-04 — Updater & Recovery (18)

Activation prerequisite: PKG-03 COMPLETE with accepted Windows installer/layout/service/signable-artifact handoff.

1. `04.01` Activate PKG-04 authority and reconcile the accepted PKG-03 installer handoff.
2. `04.02` Freeze signed update manifest, version, channel, platform and artifact identity contract. Depends: 04.01.
3. `04.03` Freeze update endpoint, TLS, trust-root, replay and anti-downgrade policy. Depends: 04.01.
4. `04.04` Bounded update discovery, download, resume/cache and artifact verification. Depends: 04.02, 04.03.
5. `04.05` Updater-helper protocol, install-root containment and exclusive update-lock lifecycle. Depends: 04.01.
6. `04.06` Multi-component staged transaction plan for Agent, CLI, Desktop and updater helper. Depends: 04.01.
7. `04.07` Safe process/service quiesce, locked-file handling and restart coordination. Depends: 04.05, 04.06.
8. `04.08` Atomic verified apply while preserving mutable user/machine state. Depends: 04.04–04.07.
9. `04.09` Interrupted/failed apply restoration and crash-safe recovery. Depends: 04.08.
10. `04.10` Verified rollback to the previous accepted release without state corruption. Depends: 04.08.
11. `04.11` Concurrent updater exclusion plus stale-lock detection and explicit recovery. Depends: 04.05.
12. `04.12` Desktop/Tauri update bridge and deterministic available/download/install/restart/error UX states. Depends: 04.02–04.04, 04.07.
13. `04.13` CLI update check/status/apply/rollback operator workflow. Depends: 04.02–04.05.
14. `04.14` Channel/version eligibility including downgrade, replay and invalid-metadata rejection. Depends: 04.02, 04.03.
15. `04.15` Offline, partial, corrupt, tampered and interrupted update negative matrix. Depends: 04.09–04.11, 04.14.
16. `04.16` Installed Windows end-to-end update/rollback over the accepted PKG-03 MSI/NSIS boundary. Depends: 04.08–04.13.
17. `04.17` Release/update metadata, checksums, provenance and PKG-05 cross-platform handoff. Depends: 04.02, 04.04, 04.16.
18. `04.18` Fresh-state PKG-04 final gate with exact-head updater/recovery regression evidence. Depends: 04.02–04.17.

Primary implementation waves after activation: `{04.02,04.03,04.05,04.06}` -> `{04.04,04.07,04.11,04.14}` -> `{04.08,04.12,04.13}` -> `{04.09,04.10}` -> `{04.15,04.16}` -> `04.17` -> `04.18`.

Scope guard: existing `vsn-update`/`vsn-updater-helper` is the starting boundary. Tauri updater adoption, if used, cannot create a competing trust/version authority or weaken signature/TLS verification.

# PKG-05 — Linux + macOS Release (23)

Activation prerequisite: PKG-04 COMPLETE with accepted updater/recovery and update-manifest handoff.

1. `05.01` Activate PKG-05 authority and reconcile the accepted PKG-04 update/recovery handoff.
2. `05.02` Freeze supported Linux/macOS OS-version, architecture and release-channel matrix. Depends: 05.01.
3. `05.03` Freeze cross-platform immutable/mutable install, config, data, cache and log layout. Depends: 05.01.
4. `05.04` Locked Linux Rust release payload for Agent, CLI and updater helper. Depends: 05.02, 05.03.
5. `05.05` Tauri Linux Desktop production build and WebKitGTK/glibc compatibility baseline. Depends: 05.02, 05.03.
6. `05.06` Linux CLI install/PATH exposure and shell-environment behavior. Depends: 05.02–05.04.
7. `05.07` Linux Agent systemd lifecycle, permissions and clean removal. Depends: 05.02–05.04.
8. `05.08` Deterministic Linux AppImage bundle and runtime acceptance. Depends: 05.02, 05.03, 05.05.
9. `05.09` Deterministic Debian package, dependencies and lifecycle scripts. Depends: 05.02, 05.03, 05.05.
10. `05.10` Deterministic RPM package, dependencies and lifecycle scripts. Depends: 05.02, 05.03, 05.05.
11. `05.11` Linux clean install, upgrade, repair/uninstall and mutable-state preservation matrix. Depends: 05.06–05.10.
12. `05.12` Locked macOS Rust release payload and x86_64/aarch64/universal architecture decision. Depends: 05.02, 05.03.
13. `05.13` Tauri macOS app bundle, hardened-runtime and entitlement boundary. Depends: 05.02, 05.03, 05.12.
14. `05.14` macOS CLI install/PATH exposure and shell-environment behavior. Depends: 05.02, 05.03, 05.12.
15. `05.15` macOS Agent launchd lifecycle, permissions and clean removal. Depends: 05.02, 05.03, 05.12.
16. `05.16` Deterministic macOS DMG/direct-distribution bundle. Depends: 05.13.
17. `05.17` macOS code-signing, notarization and stapling boundary with secretless repository/CI evidence. Depends: 05.13, 05.16.
18. `05.18` macOS clean install, upgrade and uninstall with mutable-state preservation. Depends: 05.14–05.17.
19. `05.19` PKG-04 updater parity on accepted Linux and macOS release artifacts. Depends: 05.11, 05.18.
20. `05.20` Cross-platform checksums, SBOM/provenance and release manifest for all accepted targets. Depends: 05.11, 05.18, 05.19.
21. `05.21` Reproducible multi-platform CI matrix and deterministic artifact naming/retention. Depends: accepted Linux/macOS bundles and 05.20.
22. `05.22` Clean Linux/macOS runner or VM end-to-end CLI/Desktop/Agent/update smoke matrix. Depends: 05.11, 05.18, 05.21.
23. `05.23` Fresh-state PKG-05 final gate and PKG-06 security-certification handoff. Depends: 05.20–05.22.

Primary implementation waves: `{05.02,05.03}` -> `{05.04,05.05,05.12}` -> Linux `{05.06–05.10}` and macOS `{05.13–05.15}` with max-5 scheduling -> `{05.11,05.16}` -> `{05.17,05.18}` -> `05.19` -> `{05.20,05.21}` -> `05.22` -> `05.23`.

Scope guard: support matrix must be explicit. Linux builds must use a compatible baseline; macOS signing/notarization secrets stay outside repository content/evidence.

# PKG-06 — Security Certification (20)

Activation prerequisite: PKG-05 COMPLETE with accepted Windows/Linux/macOS release matrix/artifacts.

1. `06.01` Activate PKG-06 authority and freeze security verification baseline, assets and trust boundaries.
2. `06.02` System threat model covering data flows, privilege boundaries and attack surfaces. Depends: 06.01.
3. `06.03` Machine identity and authenticated IPC replay/nonce/frame/response-binding certification. Depends: 06.02.
4. `06.04` Authorization, least privilege and workspace/VSN-managed-resource containment certification. Depends: 06.02.
5. `06.05` Secrets, credentials, key material, storage, redaction and log-leakage certification. Depends: 06.02.
6. `06.06` Workspace files, binary transfer, archive/path traversal, symlink and integrity certification. Depends: 06.02.
7. `06.07` Direct terminal, pipe and PTY command/session/input/output/resource-boundary certification. Depends: 06.02.
8. `06.08` Preview, DNS, local domain and HTTPS SSRF/rebinding/header/trust-boundary certification. Depends: 06.02.
9. `06.09` SQLite/native database query, mutation, credential, TLS and capability-boundary certification. Depends: 06.02.
10. `06.10` Runtime, project bootstrap, service and container execution/trust-boundary certification. Depends: 06.02.
11. `06.11` Desktop/Tauri CSP, capability, IPC and webview/frontend trust-surface certification. Depends: 06.02.
12. `06.12` Windows installer/updater signature, privilege, repair, rollback and TOCTOU certification. Depends: 06.02.
13. `06.13` Linux/macOS package, service, code-signing/notarization and permission certification. Depends: 06.02.
14. `06.14` Dependency, SBOM, advisory, provenance and software-supply-chain certification. Depends: 06.01.
15. `06.15` Static analysis, unsafe-code review, secret scan and pinned security-tool baseline. Depends: 06.14.
16. `06.16` Dynamic negative, abuse, property/fuzz and malformed-protocol security matrix. Depends: 06.03–06.13.
17. `06.17` Audit/security-event integrity, observability and sensitive-data exclusion certification. Depends: 06.03–06.13.
18. `06.18` NIST SSDF 1.1 and applicable OWASP ASVS 5.0.0 evidence mapping with explicit exceptions. Depends: 06.15–06.17.
19. `06.19` Security remediation and exact-head retest with zero unresolved Critical/High findings in scope. Depends: 06.15–06.18.
20. `06.20` Fresh-state PKG-06 security certification final gate and PKG-07 handoff. Depends: 06.19.

Primary implementation waves: `06.01` -> `{06.02,06.14}` -> certification slices `06.03–06.13` scheduled max five at a time + `06.15` -> `{06.16,06.17}` -> `06.18` -> `06.19` -> `06.20`.

Scope guard: ASVS is applied only where relevant; NIST SSDF 1.1 is the final baseline while SSDF 1.2 remains draft. Certification claims must match captured evidence exactly.

# PKG-07 — Production Resilience (22)

Activation prerequisite: PKG-06 COMPLETE with accepted security-remediated candidate.

1. `07.01` Activate PKG-07 authority and freeze resilience SLOs, budgets and deterministic fault model.
2. `07.02` Startup, readiness, shutdown and restart idempotency under normal/degraded conditions. Depends: 07.01.
3. `07.03` Crash/abrupt-termination recovery and persistent-state consistency. Depends: 07.01, 07.02.
4. `07.04` Installer/updater interruption, reboot and recovery resilience. Depends: 07.01, 07.02.
5. `07.05` Disk-full, low-space, read-only and filesystem-permission failure handling. Depends: 07.01.
6. `07.06` Config/control-state corruption detection, fail-closed behavior and bounded recovery. Depends: 07.01.
7. `07.07` Audit/log rotation, retention bounds and disk-growth resilience. Depends: 07.01.
8. `07.08` IPC saturation, disconnect/reconnect, timeout and backpressure resilience. Depends: 07.01.
9. `07.09` Direct terminal/pipe/PTY runaway process, output-pressure and cleanup resilience. Depends: 07.01.
10. `07.10` Workspace file/binary transfer interruption, resume and partial-state cleanup resilience. Depends: 07.01.
11. `07.11` Preview/DNS/domain network outage, address/port conflict and recovery resilience. Depends: 07.01.
12. `07.12` Database unavailable, timeout, transaction/error and local-state recovery resilience. Depends: 07.01.
13. `07.13` Runtime/container/service provider unavailable and partial-operation recovery. Depends: 07.01.
14. `07.14` Desktop offline/reconnect, stale-view and long-running operation state resilience. Depends: 07.01, 07.02.
15. `07.15` CPU, memory, handle/thread/process ceilings and leak/resource-budget verification. Depends: core runtime resilience slices.
16. `07.16` Concurrency, race, lock-contention and mutation-idempotency stress matrix. Depends: 07.03–07.14.
17. `07.17` Sleep/resume, logout/login, reboot and service/app relaunch lifecycle recovery. Depends: 07.02–07.04, 07.14.
18. `07.18` Long-duration soak plus repeated start/stop/install/update/session lifecycle cycles. Depends: 07.15–07.17.
19. `07.19` Critical local state snapshot/recovery procedure and restoration verification. Depends: 07.03, 07.06.
20. `07.20` Post-failure diagnostics/support evidence with bounded size and secret exclusion. Depends: all failure-mode slices through 07.19.
21. `07.21` Exact-head Windows/Linux/macOS resilience regression matrix. Depends: 07.18–07.20.
22. `07.22` Fresh-state PKG-07 final resilience gate and PKG-08 pentest/stable-1.0 handoff. Depends: 07.21.

Primary implementation waves: `07.01` -> resilience slices `07.02–07.14` scheduled max five at a time -> `{07.15,07.16,07.17,07.19}` -> `07.18` -> `07.20` -> `07.21` -> `07.22`.

Scope guard: fault injection must be bounded/reversible and must not mutate remote production resources.

# PKG-08 — Pentest + Stable 1.0 (25)

Activation prerequisite: PKG-07 COMPLETE with accepted resilient release candidate.

1. `08.01` Activate PKG-08 authority; freeze pentest rules, target hashes and stable-1.0 release candidate.
2. `08.02` Independent attacker-view reconnaissance and final attack-surface inventory. Depends: 08.01.
3. `08.03` Authentication, machine-identity and IPC replay/session/binding penetration tests. Depends: 08.02.
4. `08.04` Authorization, policy bypass and privilege-escalation penetration tests. Depends: 08.02.
5. `08.05` Workspace filesystem traversal, symlink/hardlink and containment penetration tests. Depends: 08.02.
6. `08.06` Binary transfer integrity, offset/resume and resource-abuse penetration tests. Depends: 08.02.
7. `08.07` Direct command execution, injection and workspace-escape penetration tests. Depends: 08.02.
8. `08.08` Persistent pipe/PTY hijack, escape, resize/input and denial-of-service penetration tests. Depends: 08.02.
9. `08.09` Preview SSRF, header abuse, loopback-bypass and request-smuggling-oriented tests. Depends: 08.02.
10. `08.10` DNS rebinding, local-domain and HTTPS/trust-boundary penetration tests. Depends: 08.02.
11. `08.11` SQLite/native database injection, credential and capability-bypass penetration tests. Depends: 08.02.
12. `08.12` Runtime/project bootstrap/container/service execution and trust-bypass penetration tests. Depends: 08.02.
13. `08.13` Desktop/Tauri IPC, CSP, capabilities and webview/frontend penetration tests. Depends: 08.02.
14. `08.14` Windows MSI/NSIS install/repair/uninstall privilege and residue penetration tests. Depends: 08.02.
15. `08.15` Updater signature, replay, downgrade, rollback, TOCTOU and supply-path penetration tests. Depends: 08.02.
16. `08.16` Linux packaging/systemd permissions, install paths and service-boundary penetration tests. Depends: 08.02.
17. `08.17` macOS signing/notarization/entitlement/launchd/install-boundary penetration tests. Depends: 08.02.
18. `08.18` Secret, log, artifact, crash-output and release-metadata leakage campaign. Depends: 08.02.
19. `08.19` Dependency/supply-chain/SBOM/advisory and release-provenance adversarial recheck. Depends: 08.02.
20. `08.20` Targeted malformed-input, fuzz and resource-exhaustion/DoS attack campaign. Depends: 08.03–08.19.
21. `08.21` Finding normalization, severity/ownership/evidence ledger and remediation gate. Depends: 08.20.
22. `08.22` Remediation round with regression tests for every accepted finding. Depends: 08.21.
23. `08.23` Independent exact-head retest with zero unresolved Critical/High findings. Depends: 08.22.
24. `08.24` Stable 1.0 release-candidate reproducibility, signed provenance and clean cross-platform install/update smoke. Depends: 08.23.
25. `08.25` Final pentest + Stable 1.0 exact-head gate and canonical completion evidence. Depends: 08.24.

Primary implementation waves: `08.01` -> `08.02` -> attack slices `08.03–08.19` scheduled max five at a time -> `08.20` -> `08.21` -> `08.22` -> `08.23` -> `08.24` -> `08.25`.

Scope guard: pentest is limited to owned local test targets/fixtures and explicitly authorized infrastructure. No third-party or production attack activity is implied by this package.

## Durable resume rule

At any interruption, recover state from live GitHub rather than chat:

1. read `docs/MASTER-EXECUTION-STATUS.json` and `.ai/state.json`;
2. identify the single activated package;
3. read that package's frozen tracker/manifest and exact plan hashes;
4. enumerate open package PRs and exact-head CI/evidence;
5. select only dependency-ready tasks, max five;
6. reconcile Linear package parent/children and blockers;
7. continue from the earliest failed/incomplete accepted dependency path, never from a remembered guess.
