# PKG-02 02.23 Development Preflight

Feature ID: `pkg02-0223-test-dns`  
Version: `1.0.0`  
Live canonical HEAD before feature branch: `94feeb8e67dad96ac6a384a8517229ba2c5c38f5`  
Canonical state: `PKG-02 22/27 = 81.48%, active 02.23`  
Candidate: `c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`  
Product version: `0.38.1`

## Frozen plan verification

Plan: `.ai/plans/pkg02-0223-test-dns-v1.md`  
SHA-256: `cc9b7b503c87d4ede7fb625e080500049fd0d3c4f0d8cdd956f2d7747c3db9ed`  
Approval: canonical frozen `docs/MASTER-EXECUTION-PLAN.md` task 02.23.

The plan bytes must continue to match this digest before every implementation mutation. A mismatch blocks development.

## Completed prerequisite stages

- research artifact `.ai/features/pkg02-0223/research.md`: `05a7a1116eedf9308abf6bd8852a7369134b0c5db473ce884e3fc25fb3a3a71d`
- architecture/data-flow/security/design/QA/performance review bundle `.ai/features/pkg02-0223/lifecycle-review.md`: `3012cef4a49d218ceaf5d75434c8f828d802afa2e1184b14f198c2ab247d95ff`

Every required stage is explicitly present as a section in the lifecycle review bundle; none is silently skipped.

## Research freshness

Last reviewed: `2026-08-24`  
Market/protocol delta: `none`  
Result: no scope or acceptance change required.

## Expected mutations

Primary:
- `scripts/self-hosted/pkg02-0223.ps1`
- `.github/workflows/pkg02-0223-test-dns-responder.yml`
- `.ai/manifests/pkg02-0223-test-dns.v1.json`

Conditional minimum bug-fix files only if certification exposes a mapped AC failure:
- `crates/vsn-core/src/lib.rs`
- `crates/vsn-network/src/lib.rs`
- `crates/vsn-system/src/lib.rs`
- `apps/agent/src/main.rs`

No 02.24+ file or behavior may be implemented.

## Allowed execution/network/privilege class

- GitHub-hosted Windows/X64 acceptance;
- local authenticated IPC `127.0.0.1:39731`;
- ephemeral/high loopback UDP DNS listener only;
- normal repository CI dependency/toolchain access;
- no public listener, upstream DNS forwarding, port 53 requirement, or OS resolver mutation.

## Privileged/external actions

No privileged OS mutation is authorized or required by 02.23. User authorization covers safe repository PR merges, not bypassing frozen task boundaries or performing machine-level resolver/hosts/CA mutation.

## Acceptance

Use AC-01..AC-12 from the frozen plan and the required exact-head regression list recorded in the manifest. Artifact integrity and cleanup must be independently inspected before state projection or merge.
