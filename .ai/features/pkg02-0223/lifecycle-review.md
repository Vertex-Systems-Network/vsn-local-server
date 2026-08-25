# PKG-02 02.23 Lifecycle Review Bundle

Feature ID: `pkg02-0223-test-dns`  
Version: `1.0.0`  
Canonical base: `94feeb8e67dad96ac6a384a8517229ba2c5c38f5`

## Architecture

### Existing seams reused

- CLI `dns` group -> authenticated IPC client.
- Agent local authenticated dispatcher -> `vsn_core` DNS functions.
- Core -> `vsn_network::dns_resolver_plan` and VSN managed-process primitives.
- Managed child -> current `vsn-agent dns-server --listen <loopback>` entrypoint.
- `vsn_network::run_dns_server` -> loopback UDP socket and bounded DNS response builder.

No new provider, daemon, protocol library, privileged helper, persistence model, or public network surface is introduced unless acceptance exposes a minimum bug fix.

### Ownership and contracts

- `vsn-network`: DNS wire parsing/response and listen validation.
- `vsn-core`: policy check and lifecycle ownership of managed process `vsn-dns`.
- `vsn-system`: managed child state/start/stop persistence.
- `vsn-agent`: authenticated local command boundary and child entrypoint.
- `vsn` CLI: user operator surface.
- A = type 1 / class IN / 4-byte `127.0.0.1`.
- AAAA = type 28 / class IN / 16-byte `::1`.
- non-`.test` = RCODE 5, zero answers.

### Failure modes

- invalid/non-loopback listen -> reject before spawn;
- occupied UDP endpoint -> responder cannot be certified healthy;
- malformed/compressed query name -> no successful answer;
- external name -> REFUSED, no forwarding;
- stopped process -> no response;
- IPC unavailable/authentication failure -> CLI nonzero failure.

### Expected implementation files

Primary certification:
- `scripts/self-hosted/pkg02-0223.ps1`
- `.github/workflows/pkg02-0223-test-dns-responder.yml`

Conditional bug-fix files only if a mapped AC fails:
- `crates/vsn-core/src/lib.rs`
- `crates/vsn-network/src/lib.rs`
- `crates/vsn-system/src/lib.rs`
- `apps/agent/src/main.rs`

## Data Flow

### Operator path

1. Local user invokes `vsn dns plan/start/status/stop`.
2. CLI sends bounded parameters through authenticated local IPC `127.0.0.1:39731`.
3. Agent authenticates and assigns local authenticated principal.
4. Core enforces `NetworkView`/`NetworkManage`.
5. `dns start` owns VSN data-directory state and spawns current Agent executable as `dns-server --listen <loopback>`.
6. Child binds validated loopback UDP and answers bounded DNS datagrams.
7. `dns stop` terminates the managed child.
8. Generic Agent IPC audit records operations.

### DNS packet path

Local UDP probe -> loopback high port -> bounded 4096-byte receive buffer -> exact-one-question parser -> local policy -> A/AAAA response or REFUSED -> same peer.

No packet is forwarded upstream.

### Persistence / secrets / cleanup

Managed process state and `dns.log` stay under VSN-owned data. No new secret is introduced. Existing IPC key bytes are never emitted; certification may compare only pre/post existence and SHA-256. Acceptance stops DNS/Agent, restores `LOCALAPPDATA` and IPC key state, releases ports, deletes sandbox, and retains bounded evidence.

## Security

### Assets and controls

- local namespace integrity -> `.test` only, no recursion/forwarding;
- public exposure -> loopback-only bind;
- parser abuse -> one question, label <=63 bytes, encoded name <=255 bytes, no compressed incoming names;
- process spoof/stale state -> fixed managed ID `vsn-dns`, lifecycle/occupied-port/cleanup gates;
- privilege creep -> no `network-admin`, port 53, NRPT, hosts, resolver files, CA or Caddy in 02.23;
- IPC bypass -> existing authenticated IPC/policy;
- evidence spoofing -> exact `EXPECTED_SHA`, no synthetic merge SHA authority, artifact/internal digest recomputation;
- secret leakage -> no IPC key content in logs/evidence.

### Negative tests

- `0.0.0.0:<port>` rejected;
- `127.0.0.1:0` rejected;
- non-`.test` -> RCODE 5, zero answers;
- occupied UDP port cannot produce accepted healthy lifecycle;
- post-stop query fails/times out.

Residual risk: no recursion, DNSSEC, TCP DNS, EDNS, incoming compression, or OS resolver integration is certified by 02.23.

## Design / Operator Contract

No new Desktop/web UI.

Commands:
- `vsn dns plan [loopback:port]`
- `vsn dns start [loopback:port]`
- `vsn dns status`
- `vsn dns stop`

Default remains `127.0.0.1:53535`. Plan is review-before-mutate and reports the privileged OS-resolver boundary. Errors remain standard nonzero CLI failures with structured Agent error text.

## QA & Evidence Map

- AC-01 -> exact checkout/runner/toolchain binder.
- AC-02 -> `dns-plan.json`.
- AC-03 -> invalid-listen outputs + proof no `network-admin`.
- AC-04 -> start/status/stop/restart outputs and UDP readiness/post-stop probes.
- AC-05 -> `dns-a.json`.
- AC-06 -> `dns-aaaa.json`.
- AC-07 -> `dns-external.json`.
- AC-08 -> network/core unit tests and source invariants.
- AC-09 -> occupied-port negative transcript + final managed-state/port cleanup.
- AC-10 -> `audit.json`, valid/nonzero.
- AC-11 -> `cleanup.json`, every field true.
- AC-12 -> exact-source `evidence.json`, digest file, workflow binder, independent artifact verification.

Build/test commands:
- `cargo fmt --all -- --check`
- strict Clippy for `vsn-network`, `vsn-core`, `vsn-ipc`
- locked tests for those packages
- `git diff --check`
- release build `vsn-agent` + `vsn`
- fresh Windows E2E harness.

A new source head invalidates previous acceptance; required exact-head gates must rerun.

## Performance

- Agent readiness <=25s.
- DNS responder readiness/restart <=5s.
- individual UDP probe timeout <=1200ms.
- normal A/AAAA response <1000ms on GitHub-hosted Windows.
- stop -> non-response verification <=2s.
- receive buffer remains 4096 bytes.
- no unbounded logs/output.

Any leftover responder process/listener or budget violation fails acceptance.
