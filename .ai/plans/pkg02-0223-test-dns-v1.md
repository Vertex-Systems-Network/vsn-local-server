# PKG-02 02.23 Frozen Plan — Local `.test` DNS Responder

Feature ID: `pkg02-0223-test-dns`  
Version: `1.0.0`  
Canonical base SHA: `94feeb8e67dad96ac6a384a8517229ba2c5c38f5`  
Approval reference: `docs/MASTER-EXECUTION-PLAN.md — frozen PKG-02 task 02.23`  
Approved date: `2026-08-24`

## Outcome

Genuinely certify the frozen task:

`02.23 — Local .test DNS responder lifecycle and protocol behavior: plan/start/status/stop, A/AAAA loopback answers and refusal of non-.test names.`

## In scope

- authenticated CLI/Agent DNS plan/start/status/stop;
- VSN-owned managed responder process;
- loopback-only UDP listener on a non-privileged certification port;
- `.test` A and AAAA loopback responses;
- policy refusal for non-`.test` names;
- bounded malformed-query behavior already represented by product unit tests;
- restart and post-stop unavailability;
- exact-source GitHub-hosted Windows evidence;
- audit-chain and cleanup proof;
- minimum bug fixes required if frozen acceptance exposes defects.

## Explicit non-goals

- no OS resolver apply/remove/status mutation acceptance;
- no port-53/elevated integration;
- no hosts-file changes;
- no local CA, TLS or Caddy acceptance;
- no DNS recursion, forwarding, caching or public listener;
- no 02.24+ product work;
- no task denominator/order changes.

## Dependencies

- canonical `02.01`–`02.22` integrated DONE;
- canonical PKG-02 state `22/27 = 81.48%`, active `02.23`;
- Rust/cargo exact `1.97.1`;
- authenticated IPC on `127.0.0.1:39731`;
- release candidate `c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`, product `0.38.1`.

## User-visible behavior

- `vsn dns plan [listen]` reports `.test`, IPv4 `127.0.0.1`, IPv6 `::1`, and that OS resolver integration requires privilege.
- `vsn dns start [listen]` starts the VSN-managed responder only for loopback/nonzero listen addresses.
- `vsn dns status` reports managed process state.
- `vsn dns stop` stops it.
- After start, `.test` A/AAAA queries become answerable within a bounded startup window; after stop, they do not answer.
- Restart is supported without stale managed-process state.

## Security / network constraints

- UDP bind target must remain loopback.
- External names must not be recursively resolved or forwarded.
- Non-`.test` names return policy refusal (`RCODE=5`) with zero answers.
- DNS query parsing remains bounded to one question, <=63-byte labels, <=255-byte encoded name, and rejects compressed query names in this baseline.
- Normal task acceptance never invokes privileged `network-admin` resolver mutation.
- A failed child bind/readiness must not be represented as a healthy long-lived responder.

## Acceptance criteria

- `AC-01 Exact source`: certification runs on GitHub-hosted Windows/X64 and verifies checkout HEAD equals `EXPECTED_SHA`; rustc/cargo are exactly 1.97.1.
- `AC-02 Plan`: authenticated CLI plan returns exact listen, suffix `.test`, IPv4 `127.0.0.1`, IPv6 `::1`, and `requires_admin_to_configure_os_resolver=true`.
- `AC-03 Listener boundary`: non-loopback and port-zero plans fail; no certification action configures the OS resolver.
- `AC-04 Lifecycle`: start -> bounded readiness -> status running -> stop -> status/non-response stopped -> restart -> bounded readiness -> final stop.
- `AC-05 A`: `demo.test` type A returns `RCODE=0`, one answer, `127.0.0.1`.
- `AC-06 AAAA`: nested `.test` name type AAAA returns `RCODE=0`, one answer, `::1`.
- `AC-07 External refusal`: `example.com` returns `RCODE=5` and zero answers.
- `AC-08 Parser safety`: unit/source gates cover exact-one-question, compressed-name rejection, label/name bounds and loopback-only bind.
- `AC-09 Occupied-port behavior`: starting against an already-occupied UDP endpoint must fail closed or become observably non-running within the bounded readiness check; it must not leave a stale healthy responder claim or unmanaged child.
- `AC-10 Audit`: authenticated DNS operations leave a valid audit chain with nonzero events.
- `AC-11 Cleanup`: final DNS child and Agent are stopped, responder UDP port and IPC TCP port are released, original IPC-key state/hash is restored, `LOCALAPPDATA` is restored, and sandbox is removed.
- `AC-12 Evidence integrity`: artifact binds feature/plan IDs, canonical base, exact source, candidate/product, runner, checks/measurements/cleanup; evidence SHA-256 is independently recomputable.

## Required regression gates on final exact head

- AI Planning Governance
- Repository Governance
- PKG-02 Acceptance Sequence
- 02.02 Authenticated IPC
- 02.08 Windows GitHub-Hosted Certification
- 02.14 Local Diagnostics
- 02.16 Workspace Text Files
- 02.17 Resumable Binary Workspace Transfer
- 02.18 Bounded Direct Terminal Execution
- 02.19 Persistent Pipe Terminal Sessions
- 02.20 PTY/ConPTY Lifecycle
- 02.21 Loopback Preview Fetch
- 02.22 Advanced Preview Requests
- 02.23 `.test` DNS Responder

## Evidence artifact

`pkg02-0223-test-dns-responder-github-hosted`

Expected contents include `evidence.json`, `evidence.json.sha256`, `cleanup.json`, DNS plan/lifecycle/probe outputs and bounded failure transcripts.

## Rollout / rollback

Rollout is the merge of a genuinely accepted 02.23 PR. Until merge, canonical state remains 22/27 active 02.23. If acceptance exposes a defect, patch only the minimum 02.23 boundary and rerun all required final-head gates. Rollback is PR closure/revert; no persistent OS resolver mutation is permitted by this task.

## Change control

This plan is frozen by its SHA-256 in the feature manifest. Do not edit this file in place after the manifest records its digest. Material scope/permission/acceptance changes require an approved addendum or new plan version.
