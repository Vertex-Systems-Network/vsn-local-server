# PKG-06 Dormant Research — Security Certification

Reviewed: 2026-09-06
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

`vsn-ipc` uses HMAC-authenticated loopback TCP at `127.0.0.1:39731`, protocol versioning, timestamp-skew checks, nonces/replay cache, bounded frames, connection limits, response/request nonce binding and command-specific timeouts. PKG-06 must independently attack replay-cache saturation, race/concurrency, clock manipulation, malformed/truncated frames, response substitution, local port pre-binding, hostile local clients, secure-store failure and service/user identity transitions.

### Principal/permission model

`vsn-policy::Principal::local_authenticated()` is deliberately capable. It includes `RuntimeManage`, `ServiceManage`, `RemoteManage`, `TerminalExecute`, `FilesWrite`, `DatabaseWrite`, `SecretsUse` and `SecretsManage`, while excluding higher-risk permissions including `TerminalAdmin`, `DatabaseDestructive`, `NetworkManage` and `SecretsReveal`.

Certification must mechanically bind every reachable command to its required permission and real side effect; enum names alone are not sufficient authority evidence.

### Desktop/Tauri trust surface — P0 certification focus

Desktop exposes one generic Tauri command:

`agent_call(command: String, params: Value)` -> authenticated `vsn_ipc::call(command, params)`.

The bridge is small but caller-selectable. A compromised/scriptable webview can therefore attempt every Agent command available to the local authenticated principal. CSP remains defense-in-depth rather than an authorization boundary.

06.11 must certify:
- XSS/script-injection resistance and CSP bypass attempts;
- exact Tauri command/capability exposure;
- hostile command-name/parameter mutation through `agent_call`;
- permission enforcement for every mutating command family;
- inability to reach `TerminalAdmin`, `NetworkManage`, destructive database, secret reveal or equivalent authority indirectly;
- no frontend-controlled elevation confusion;
- redacted error behavior.

If a generic bridge remains, its accepted command allowlist/permission semantics must be machine-testable. Otherwise 06.19 should narrow the bridge at the owning implementation layer.

### Remote delegated command boundary — P0 managed-process execution composition — mechanically proven

Canonical source facts:
- `Permission::is_high_risk()` blocks `MachineManage`, `NetworkManage`, `TerminalAdmin`, `DatabaseDestructive` and `SecretsReveal`; `ServiceManage` is therefore currently delegatable through `Principal::remote_delegated()`;
- `required_remote_permission()` exposes `process.managed.start` under `ServiceManage`;
- the local remote-terminal opt-in guard checks `terminal.*`, not `process.managed.start`;
- `parse_managed_spec()` accepts caller-controlled `program`, `cwd`, `args` and `log_path`;
- `vsn_core::managed_process_start()` checks `ServiceManage` and bounds the log path to VSN data, but does not bind executable/cwd to a trusted runtime/provider/workspace allowlist before `spawn_managed()`.

The composition is now mechanically proven on an isolated GitHub-hosted runner:
- branch `audit/pkg06-managed-process-remote-exec`;
- exact audit head `d3c389c3e409c736278374c70525a42cdd23d306`;
- workflow `PKG-06 Managed Process Remote Exec Audit`;
- run `33974285316` — PASS;
- Ubuntu job `101328133395` — PASS;
- artifact `9971857491` (`pkg06-managed-process-remote-exec-audit`);
- GitHub artifact digest `sha256:235ac3c6b4eb55f77bbacef23ef7ffdb01967d13d7e03cde2509a6ce4306b420`;
- independently downloaded ZIP SHA-256 matched GitHub exactly.

The probe used only a benign `/usr/bin/printf` marker on an ephemeral GitHub-hosted runner. It proved:
- `remote_process_managed_start_mapping_present=true`;
- `remote_terminal_gate_is_prefix_scoped=true`;
- `remote_service_manage_delegation_allowed=true`;
- a remote-delegated `ServiceManage` principal reached `managed_process_start`;
- the caller-selected host program executed;
- `production_or_user_state_touched=false`.

This proves the authority composition, not a production compromise. Before security acceptance, one coherent model must be frozen and proven. Viable directions include removing generic `process.managed.start` from remote authority, requiring a separate high-risk/local approval path, replacing caller-selected executable specs with trusted provider/runtime/service identities, or otherwise ensuring every host-command execution primitive obeys the same explicit local execution opt-in/approval boundary rather than command-name prefix checks.

Negative certification must cover `process.managed.start`, runtime/provider helpers, extension execution, service-management composition and any other non-`terminal.*` host-command alias. `container.exec` requires separate analysis; it must not be labelled a host escape without proof.

### Remote command allowlist — containment to preserve

Destructive cloud CLI/SSH release commands are not currently present in `required_remote_permission()`. A `remote_signed_command` therefore cannot invoke them merely by carrying `RemoteManage`. Preserve this as a regression invariant whenever the remote command map or high-risk classification changes.

### Remote runtime-install trust composition — mechanically proven blocker

Canonical source currently permits `RuntimeManage` remote delegation and maps `runtime.install` to that permission. The remote-exposed `vsn_core::runtime_install()` uses plain `vsn_runtime::load_catalog()`, while `load_catalog_verified()` separately verifies catalog signatures against a trust store. Artifact hashes protect bytes, not publisher/catalog authority.

An isolated GitHub-hosted audit mechanically proved this composition:
- branch `audit/pkg06-remote-runtime-trust`;
- exact head `4e97cbae569f01eac31ac5bd86d7b90b66d4de47`;
- run `33970288403` — PASS;
- job `101317460134` — PASS;
- artifact `9970724714`;
- digest `sha256:7331f848fd8d071ba36d49850ab7377add6b8187faa4c34721cf32f9b8d33a72`;
- independent ZIP hashing matched GitHub.

Evidence records `agent_remote_runtime_install_mapping=true`, `remote_runtime_manage_delegation_allowed=true`, `trusted_loader_rejects_unsigned_catalog=true`, `unsigned_catalog_runtime_install_accepted=true`, `installed_artifact_hash_bound=true`, and `production_or_user_state_touched=false`.

Remote runtime installation must therefore either leave generic remote authority, require the verified signed-catalog path, or prove an equivalent least-privilege local trust boundary. Activation negative tests must include unsigned/wrong-key catalogs, catalog swap, wrong-hash bytes, `file://` substitution and attempts to combine other remote write authority with catalog placement.

### Secret/key custody

`vsn-security` uses Ed25519 device identity, HMAC IPC authentication, OS keyring storage on non-Windows and an ACL-protected Windows IPC-secret path. PKG-05's service/user custody findings are prerequisite inputs. Certification must cover missing/wrong/tampered key entries, ACL abuse, identity mismatch, machine copy/restore, service/user transitions, redaction and install/update/repair/uninstall behavior.

### Dependency / automated security tooling baseline

Dependabot covers Cargo, Rust toolchain and GitHub Actions. This preflight did not identify an already accepted repository certification lane named for `cargo audit`, CodeQL or Gitleaks; 06.01/06.14/06.15 must re-audit fresh main and freeze exact tools/versions/configuration, thresholds, suppression governance and evidence identity. Tool count is not an objective; each selected tool must own a distinct finding class.

### Existing CI evidence is not certification

Current package CI provides useful inherited negative/containment/exact-head evidence, but PKG-06 must replay security-relevant claims against the exact accepted PKG-05 release candidate and test cross-boundary composition failures.

## Activation-time applicability rules

06.01/06.02 must freeze a threat model and applicability matrix. External framework requirements must be classified applicable, not applicable with rationale, or covered by an equivalent accepted control. Framework names must not be used as blanket compliance claims.

Security evidence must bind exact candidate hashes and cover authenticated IPC, authorization, secrets/redaction, filesystem/archive/symlink integrity, terminal/PTY, preview/DNS/SSRF, databases, runtime/service/container, Desktop/Tauri, installer/updater/signing/rollback, Linux/macOS packaging/signing and supply-chain provenance/SBOM boundaries.

## Source-to-PKG-06 gap map

Current future certification gaps include:
- no frozen whole-system threat model tied to exact PKG-05 artifacts;
- no complete command-to-permission/reachability matrix;
- generic Desktop bridge requires explicit webview-to-Agent abuse certification;
- no frozen remote execution primitive inventory;
- `process.managed.start` / `ServiceManage` composition is mechanically proven and requires remediation/bounded authority proof;
- remote `runtime.install` unsigned-catalog composition is mechanically proven and requires trusted-catalog authority;
- `container.exec` remote exposure needs an explicit relationship to terminal/host-execution policy;
- no frozen static/advisory/secret-scan toolchain;
- no package-wide unsafe/native-command inventory;
- no accepted cross-OS secure-store/service-context abuse matrix;
- no certification-wide fuzz/property/malformed-protocol campaign;
- no final SSDF/ASVS applicability evidence matrix;
- no final remediation ledger proving zero unresolved Critical/High findings in frozen scope.

## Activation mapping

- `06.01`: bind exact PKG-05 release hashes, OS matrix, source SHA, SBOM/provenance and security-tool versions.
- `06.02`: threat model, trust-boundary graph and applicability map.
- `06.03`–`06.13`: certify existing boundaries, max five dependency-ready slices concurrently.
- `06.11`: dedicated Desktop generic-bridge attack path.
- `06.12`/`06.13`: enumerate remote command/permission exposure; prove every execution alias obeys local opt-in/approval and signed-catalog authority.
- `06.14`: dependency/SBOM/provenance verification.
- `06.15`: static/unsafe/native/secret scanning baseline.
- `06.16`: malformed/fuzz/abuse/property tests.
- `06.17`: audit/security-event integrity and sensitive-data exclusion.
- `06.18`: NIST SSDF 1.1 + applicable ASVS 5.0.0 evidence mapping.
- `06.19`: remediation at owning layer; invalidate/re-run affected evidence.
- `06.20`: exact-head final security gate only after candidate hashes/finding ledger are frozen.

## Negative matrix carried forward

Include invalid/replayed/expired IPC, replay saturation, malformed frames, response substitution, hostile local port ownership, stolen/wrong IPC secret, secure-store failure, service/user mismatch, Desktop command mutation, indirect privilege escalation, remote `ServiceManage` host-execution attempts while terminal opt-in is disabled, unsigned/wrong-key runtime catalogs and catalog substitution, container execution policy aliasing, terminal/files/database abuse, traversal/reparse/symlink attacks, SSRF/rebinding/header abuse, database injection/capability bypass, installer/updater TOCTOU, signature/provenance substitution, dependency/advisory failures, secret leakage and malformed security-event input.

## Acceptance discipline

Zero unresolved Critical/High findings remains a final target, but severity labels cannot suppress a reproducible trust-boundary failure. Fixes must return to the smallest owning implementation package/task and invalidate stale evidence for changed subjects.

## Stop conditions

Stop if PKG-05 is not canonically COMPLETE, the candidate changes without re-binding evidence, framework/tooling changes materially alter scope, command/permission reachability cannot be deterministic, any remote execution primitive bypasses the frozen local opt-in/approval model, remote runtime installation cannot bind accepted catalog trust authority, suppressions are ungoverned, or a green result requires weakening an accepted control.
