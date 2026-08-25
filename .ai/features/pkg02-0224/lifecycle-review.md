# PKG-02 02.24 Lifecycle Review Bundle

Feature ID: `pkg02-0224-domain-https`  
Version: `1.0.0`  
Canonical base: `265bd17895231fc145ccd435c48def0a38bfd98d`

## Architecture

### Existing seams reused

- CLI `domain plan/apply/remove/reload` -> authenticated IPC.
- Agent local authenticated dispatcher -> `vsn_core` domain/network functions.
- Core -> `vsn_network` domain planning, hosts mutation, Caddy render/reload.
- Policy -> `NetworkView` for plan and high-risk `NetworkManage` for mutation.
- Separate `vsn-agent network-admin` process entrypoint -> OS-elevation check -> `local_network_admin`.
- `vsn_network::run_network_command_bounded` -> bounded Caddy validate/reload child execution.

No new public listener, remote provider, daemon, trust service, or privileged IPC permission is introduced.

### Ownership

- `vsn-network`: `.test` validation, hosts block transformation/replacement, Caddy rendering/reload helper.
- `vsn-policy`: permission classification and principals.
- `vsn-core`: policy gate and VSN-owned network paths.
- `vsn-agent`: authenticated IPC dispatcher and explicit elevated command boundary.
- `vsn` CLI: operator request surface only; it does not confer privileges.

### Planned source corrections

1. Fail closed on hosts read/UTF-8 errors.
2. Add path-scoped hosts removal for deterministic testing.
3. Replace delete-before-rename with failure-safe replace-existing semantics; Windows implementation must preserve original file security metadata where supported.
4. Add `skip_install_trust` to VSN-rendered Caddy global options.
5. Add focused policy/hosts/reload safety tests and exact-source 02.24 certification.

The Windows elevation heuristic is not changed unless acceptance demonstrates a mapped defect. Any such change must preserve fail-closed behavior and cannot broaden `NetworkManage`.

## Data Flow

### Non-mutating plan path

1. User invokes `vsn domain plan <name.test> <port>`.
2. CLI sends bounded params through authenticated IPC on `127.0.0.1:39731`.
3. Agent assigns `local_authenticated`.
4. Core requires `NetworkView`.
5. Network validates `.test`, checks target-port conflicts, and returns loopback/TLS/admin-required plan.
6. Generic IPC audit records the operation.

### Ordinary mutation-denial path

1. User invokes ordinary CLI domain apply/remove/reload.
2. Authenticated IPC reaches Agent with `local_authenticated`.
3. Core requires `NetworkManage`.
4. Policy denies before network mutation.
5. Agent returns failure and audit records denied/failed result.

### Privileged production path

1. Operator separately launches `vsn-agent network-admin ...`.
2. Agent verifies OS elevation.
3. Only after successful elevation check is `local_network_admin` created.
4. Core again enforces `NetworkManage`.
5. Network operation runs.

Normal 02.24 certification does not execute steps 4–5 against OS-global hosts/trust/resolver state.

### Hosts sandbox path

Disposable hosts bytes -> strict UTF-8 read -> parse only VSN-managed block -> update/remove requested `.test` loopback mapping -> write/sync same-directory temporary -> replace-existing operation -> resulting bytes. Read/parse/write/replace failure returns error and must not intentionally delete the original.

### HTTPS reload path

VSN-owned Caddyfile -> absolute/canonical file validation -> locate deterministic Caddy helper -> bounded `validate` -> only on success bounded `reload` -> truthful result. VSN-rendered configuration suppresses automatic local-root trust installation.

### Persistence / secrets / cleanup

- No new secret format.
- Sandbox hosts/Caddy/helper files live only in the certification sandbox.
- Existing IPC key content is never emitted; only existence/hash may be compared.
- System hosts content is never copied into evidence; only a pre/post SHA-256 or unreadable status may be recorded.
- Trust-store/NRPT content is not exported or changed.
- Audit remains in existing VSN data area.
- Final cleanup stops Agent, restores `LOCALAPPDATA` and IPC-key state, removes sandbox/helper, and proves global-state non-mutation.

## Security

### Threats and controls

- destructive hosts overwrite after read/decode failure -> strict propagated error, unchanged-byte negative test;
- hosts disappearance during replacement -> no destination pre-delete, replacement-failure preservation test;
- ACL/security metadata drift on Windows -> use a replacement primitive with original-metadata preservation where supported;
- injected hostname -> strict `.test` label validation;
- non-loopback hosts mapping/upstream -> reject;
- privilege escalation through ordinary IPC -> `NetworkManage` absent from `local_authenticated` and remote high-risk delegation;
- elevation-check failure -> deny before elevated principal/mutation;
- Caddy implicit root trust -> global `skip_install_trust`;
- explicit CA trust -> separate operator approval, absent from normal acceptance;
- command hang/output flood -> existing bounded Caddy helper limits;
- validation bypass -> reload never runs after failed validate;
- evidence leakage -> no hosts contents, private keys, IPC key bytes or trust-store contents in artifacts.

### Residual risk / later scope

- actual administrator interaction and UX for OS-global hosts/trust mutation may require installer/platform packaging work later;
- resolver/NRPT application is not certified here;
- public/prod TLS is out of scope;
- certificate lifecycle beyond local dev semantics is not production-certification scope.

## Design / Operator Contract

No new Desktop/web UI.

Existing operator surface is retained:

- `vsn domain plan <name.test> <port>` — review-only.
- ordinary `vsn domain apply/remove/reload` — request surface; denied for normal authenticated principal because it lacks `NetworkManage`.
- `vsn-agent network-admin ...` — explicit elevated boundary.

Errors remain nonzero and must communicate permission/elevation/validation failures without claiming mutation success.

Generated Caddy configuration remains deterministic and human-reviewable. It must visibly contain the trust-suppression global option.

## QA / Evidence Map

- AC-01 -> exact SHA/runner/toolchain binder.
- AC-02 -> plan positive/negative JSON/transcripts.
- AC-03 -> policy unit tests + ordinary CLI denial transcripts.
- AC-04 -> sandbox hosts before/apply/reapply checks.
- AC-05 -> path-scoped remove checks.
- AC-06 -> invalid-UTF-8/read-failure preservation test.
- AC-07 -> source assertion + deterministic replacement-failure preservation.
- AC-08 -> Caddy rendering unit/source assertions.
- AC-09 -> fake helper invocation log proving validate-before-reload and failure short-circuit.
- AC-10 -> elevation-boundary source/unit/CLI proof + `privileged_system_mutation_performed=false`.
- AC-11 -> audit and cleanup JSON + system-host hash non-mutation.
- AC-12 -> exact-source evidence/digest verification.

A new source head invalidates previous acceptance. Final required regressions run on the exact final PR head.

## Performance

- Agent readiness <=25s.
- domain-plan roundtrip <=5s.
- hosts sandbox operations should complete <=2s each on GitHub-hosted Windows.
- Caddy helper subprocess limit remains <=30s per validate/reload invocation; certification fake helper should complete <=5s.
- Caddy stdout <=1 MiB and stderr <=256 KiB.
- evidence files remain bounded and exclude raw system-host/trust-store contents.
- cleanup/non-mutation proof completes before artifact upload.

Any timeout, output overflow, leaked privileged process, leftover sandbox/helper, or global-state hash drift fails acceptance.
