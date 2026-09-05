# PKG-07 Dormant Research — Production Resilience

Reviewed: 2026-09-05
Canonical source audited: `79812eafdead24de88d8b3fafd19f1bfc0e1435c`
Status: **RESEARCH-ONLY / BLOCKED ON PKG-06 COMPLETE**

## Current baseline

The canonical 22-task PKG-07 sequence remains unchanged. No fault injection against production resources, resilience implementation or acceptance claim is authorized here.

## Resilience framing

NIST SP 800-160 Vol. 2 Rev. 1 frames cyber resiliency around the ability to anticipate, withstand, recover from and adapt to adverse conditions, stresses, attacks or compromises. PKG-07's existing task map already aligns with this lifecycle-oriented model and does not require denominator/task-order changes from this refresh.

Official reference:
- https://csrc.nist.gov/pubs/sp/800/160/v2/r1/final

## Canonical source audit — current main

Current source already contains multiple bounded/recovery-oriented primitives. PKG-07 should stress and compose them instead of treating resilience as a blank-slate feature.

### Service and health primitives

`vsn-system` already exposes a native service-provider abstraction for Windows, Linux and macOS and supports service state/start/stop/restart through `sc.exe`, `systemctl` and `launchctl`. The provider descriptor explicitly treats Windows/Linux/macOS as source-supported platforms.

The same crate already bounds several diagnostic/resource paths:
- diagnostic command timeout;
- bounded stdout/stderr capture;
- process/port item limits;
- TCP resolution/connect timeout and address count;
- log-tail input window/line count/response-size limits.

These are useful resilience baselines, but they are not lifecycle certification. Windows restart has an explicit wait-for-stopped loop; current Linux/macOS service actions return to a later state query after the command and therefore PKG-07 must test actual convergence, delayed activation, failed units/jobs, restart storms and manager-specific transient states on accepted targets.

### IPC resource bounds

`vsn-ipc` already limits frame size, connection count, clock skew, replay-cache capacity and client timeouts. These controls should become direct inputs to 07.08 and 07.16.

Resilience certification must test sustained saturation, connection churn, half-open/slow clients, replay-cache saturation, timeout storms, Agent restart while clients are active and deterministic recovery without leaking threads/sockets or leaving clients in a false-success state.

### Audit-log persistence — useful primitive, incomplete resilience policy

`vsn-audit` already serializes append with an exclusive file lock, chains every event hash to the previous event, signs event hashes and calls `sync_data()` after each appended event. Verification fails when the hash/signature chain is invalid.

That protects integrity of accepted events, but the current primitive by itself does not define production rotation/retention, disk-full behavior, partial-final-line crash recovery, maximum audit growth, archive continuity or what the Agent should do if the audit path becomes unwritable.

Fresh source review adds two continuity cases that future resilience/security acceptance must test explicitly rather than infer from the existing primitives:
- `append()` obtains the previous event hash through `last_hash_locked()` before writing the next event, but that helper parses existing records without first proving the complete pre-existing hash/signature chain. A pre-existing parseable but tampered predecessor therefore needs an explicit append-time negative test and a frozen policy for refusing or recovering from append onto an invalid chain.
- `read_events_after()` verifies each returned event with `verify_event()`, but per-event signature/hash verification alone does not establish `previous_hash` continuity between consecutive returned records. Pagination/support-export acceptance must mechanically prove cross-event chain continuity, including the boundary from the cursor event into the first returned event.

### Audit-chain continuity — mechanically proven on isolated GitHub runner

The two source-level concerns above are now machine-proven without touching product/main or production/user data:
- audit branch `audit/pkg07-audit-chain-continuity`, exact head `062e68f4bd21b90861400386a0698d79258c144e`;
- workflow `PKG-07 Audit Chain Continuity Audit`, run `33973856880` — PASS;
- Ubuntu 24.04 job `101327002905` — PASS;
- artifact `9971719773` (`pkg07-audit-chain-continuity`), GitHub digest `sha256:82c1eec80652156c03cc7732bc7202155982418615b3c890aed7eb52c1a0b83c`;
- independently downloaded ZIP SHA-256 exactly matches GitHub.

The evidence records:
- `append_accepted_invalid_predecessor=true`;
- `full_verify_rejected_tampered_predecessor=true`;
- `cursor_read_accepted_broken_previous_hash_edges=true`;
- `full_verify_rejected_broken_cursor_chain=true`;
- `production_or_user_state_touched=false`.

Probe A first created a valid event, altered a signed field without recomputing its hash/signature, confirmed full verification rejected the file, then called the current `append()` implementation. Append succeeded because `last_hash_locked()` trusted the parseable predecessor's recorded `event_hash` without validating the existing chain first. The resulting file remained invalid under full verification.

Probe B created three valid events, then modified the second event's `previous_hash`, recomputed that event's hash/signature so it was individually valid, and rewrote the isolated audit file. Full verification correctly rejected the broken chain edges, while `read_events_after()` returned the two individually valid records successfully despite the broken predecessor relationships.

Therefore these are no longer hypothetical code-review concerns. Before PKG-07/08 acceptance, audit append must either validate the current chain/tail before extending it or use an equivalent integrity-preserving tail authority, and cursor/paged reads must prove the predecessor edge into the first returned record plus every subsequent `previous_hash` edge. A per-record signature alone is insufficient chain-continuity evidence.

These findings are research evidence only. They do not activate PKG-07, change accepted package state or authorize a product correction before prerequisites.

07.07 and 07.20 must freeze:
- retention/rotation size and count bounds;
- continuity proof across rotated segments;
- disk-full/read-only/unwritable behavior;
- truncated/partial-tail detection and recovery policy;
- append behavior when the existing chain is parseable but cryptographically invalid;
- paged-read/cursor continuity proof across returned event boundaries;
- bounded diagnostics/support export;
- secret/sensitive-data exclusion;
- cleanup evidence after stress/fault tests.

Security integrity must not be weakened merely to keep the product running: if an operation requires a durable security audit event and persistence fails, the owning security policy must define fail-closed vs degraded behavior explicitly.

### Updater crash continuity inherited from PKG-04

The PKG-04 preflight already identified crash windows, lock ownership, rollback identity and Windows durability as updater hardening requirements. PKG-07 must not re-implement that logic. Instead 07.04/07.17/07.18 should attack the accepted PKG-04 implementation with process kill, power/reboot, locked files, sleep/resume and repeated update/rollback cycles while proving transaction and mutable-state recovery.

### Cross-platform service lifecycle inherited from PKG-05

PKG-05 will own systemd/launchd installation and identity. PKG-07 should treat those accepted units/plists as inputs and test restart policies, user/session transitions, login/logout, headless startup, sleep/resume and upgrade/reboot behavior rather than changing service ownership during resilience certification.

## Source-to-PKG-07 gap map

Future resilience gaps that remain intentionally unaccepted:
- no frozen measurable startup/shutdown/restart SLOs;
- no deterministic whole-product fault-injection harness and seed model;
- no package-wide crash/power-loss recovery matrix;
- no accepted disk-full/read-only/filesystem-permission matrix across critical state paths;
- no production audit rotation/retention/growth policy and continuity evidence;
- no accepted append-on-invalid-chain policy or cursor-boundary chain-continuity enforcement despite isolated machine proof of the current gaps;
- no sustained IPC saturation/leak/backpressure matrix;
- no repeated systemd/launchd/Windows-service convergence matrix;
- no sleep/resume/logout/login lifecycle certification;
- no long-duration resource-growth/handle/thread/process soak gate;
- no cross-platform concurrency/race/idempotency stress campaign;
- no critical-state snapshot/restore procedure with unrelated-state nonmutation proof;
- no bounded post-failure support bundle acceptance;
- no exact-head Windows/Linux/macOS resilience matrix.

## Activation-time freeze targets

07.01 should freeze measurable SLOs/budgets and a deterministic fault model before running destructive or long-duration tests. Every injected failure must be bounded, reversible and attributable to a specific acceptance expectation.

Freeze at minimum:
- startup/readiness/shutdown/restart deadlines and allowed transient states;
- CPU/memory/handle/thread/process/disk-growth budgets;
- exact fault IDs, seeds, injection points and cleanup requirements;
- critical persistent-state inventory and consistency invariants;
- expected fail-open/fail-closed/degraded behavior for each fault class;
- OS/VM identity and same-machine requirements for reboot/sleep persistence claims;
- audit/log retention and support-evidence size bounds;
- soak duration/cycle counts and acceptable growth slope;
- criteria distinguishing product defect from harness/infrastructure failure.

Evidence should distinguish at least:
- expected degraded behavior;
- recoverable product defect;
- persistent-state corruption;
- infrastructure/test-harness failure;
- security control correctly failing closed;
- unrecoverable state requiring explicit operator action.

## Activation mapping

Likely minimum-conflict mapping when PKG-07 is legitimately activated:
- `07.01`: bind exact PKG-06 remediated candidate, SLOs, resource budgets, fault catalog and seed/evidence schema;
- `07.02`–`07.04`: certify lifecycle plus inherited installer/updater interruption recovery;
- `07.05`–`07.07`: filesystem/config/audit growth, chain continuity and corruption behavior;
- `07.08`–`07.14`: stress each accepted IPC/terminal/file/network/database/runtime/Desktop subsystem without changing its authority;
- `07.15` / `07.16`: measure ceilings/leaks and deterministic race/contention/idempotency stress;
- `07.17`: same-machine sleep/resume/logout/login/reboot/service/app relaunch matrix;
- `07.18`: long-duration soak and repeated lifecycle cycles with bounded growth and final residue cleanup;
- `07.19`: critical local-state snapshot/restore with unrelated-state nonmutation proof;
- `07.20`: bounded redacted post-failure support evidence with cursor/chain continuity proof;
- `07.21`: exact Windows/Linux/macOS regression matrix;
- `07.22`: final exact-head resilience gate and PKG-08 handoff.

## High-risk continuity rules

- Reboot/sleep/resume claims must use same-machine continuity evidence where the test depends on machine persistence.
- Disk-full/read-only/permission tests must prove cleanup and must not damage unrelated user/system state.
- Concurrency/race tests must retain exact seed/workload identity and not accept a rerun as erasure of a genuine product failure.
- Long-duration soak tests need bounded resource-growth metrics plus final cleanup/residue evidence.
- Recovery/snapshot procedures must prove restoration of critical local state without silently reverting unrelated state.
- Audit pagination/export claims must bind the cursor predecessor and prove every subsequent `previous_hash` edge, not only verify each event in isolation.
- A security control failing closed under a fault is not a resilience defect unless the frozen contract says the operation must degrade safely instead.

## Negative matrix carried forward

Future acceptance should exercise abrupt Agent/Desktop/helper termination, reboot/power interruption at updater phases, disk-full, read-only paths, permission denial, corrupt/truncated control state, audit partial-tail and unwritable-log conditions, parseable-but-invalid audit predecessor before append, cursor-boundary/stale-chain insertion, IPC saturation/timeout/disconnect/reconnect, runaway terminal output/processes, interrupted file transfer, DNS/network outage, port conflicts, database unavailability/timeouts, provider/service failures, Desktop stale/offline state, concurrent mutations, sleep/resume, logout/login, restart storms, repeated install/update cycles and long-duration resource growth.

Every fault run must retain exact candidate hash, OS/VM identity, fault ID/seed, pre-state, observed degraded state, recovery action, post-state and cleanup residue evidence.

## Stop conditions

Stop if PKG-06 is not canonically COMPLETE, the security-remediated candidate changes without re-binding evidence, a fault cannot be injected/recovered safely, machine continuity cannot be proven for a persistence claim, critical-state invariants are undefined, resource/SLO thresholds are unfrozen, audit chain extension/pagination cannot be made continuity-safe, or a green result would require weakening an accepted security/integrity boundary.
