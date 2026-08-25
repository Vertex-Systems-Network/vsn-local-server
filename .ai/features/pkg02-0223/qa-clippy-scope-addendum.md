# 02.23 QA Addendum — Direct-Package Clippy Scope

Feature: `pkg02-0223-test-dns`  
Addendum version: `1.0.1`  
Date: `2026-08-25`  
Class: `informational QA execution correction`  
Frozen plan: `.ai/plans/pkg02-0223-test-dns-v1.md`  
Frozen plan SHA-256: `cc9b7b503c87d4ede7fb625e080500049fd0d3c4f0d8cdd956f2d7747c3db9ed`

## Discovery

Exact-head Windows certification reached the strict Clippy stage after source, plan and lifecycle digest verification. `vsn-core` directly depends on many workspace crates, including `vsn-extension`. Cargo therefore compiled/linted unrelated dependency code while evaluating the task-scoped package set. The run failed on pre-existing Windows `needless_return` warnings in `vsn-extension`, which is outside the 02.23 DNS architecture, data flow, permissions, protocol behavior and expected mutation set.

The accepted 02.22 certification already established the repository pattern of keeping strict task Clippy focused on directly relevant packages while using tests, release builds and exact-head regression workflows for integration coverage.

## Correction

The frozen 02.23 behavioral acceptance criteria AC-01..AC-12 are unchanged. The Rust/Cargo version, `-D warnings`, direct package set and all functional/security tests remain unchanged.

Replace only the QA execution form:

`cargo clippy --locked --package vsn-network --package vsn-core --package vsn-ipc --all-targets -- -D warnings`

with:

`cargo clippy --locked --package vsn-network --package vsn-core --package vsn-ipc --all-targets --no-deps -- -D warnings`

`--no-deps` prevents unrelated dependency lint debt from becoming 02.23 acceptance authority; it does not disable Clippy or warnings for `vsn-network`, `vsn-core` or `vsn-ipc` themselves.

The following remain required and unchanged:

- `cargo fmt --all -- --check`;
- tests for `vsn-network`, `vsn-core` and `vsn-ipc`;
- locked release build of `vsn-agent` and `vsn`;
- exact-head GitHub-hosted Windows certification;
- full functional DNS lifecycle and protocol acceptance;
- required exact-head prior-task regression workflows;
- artifact, audit and cleanup verification.

## Scope / security effect

No product behavior, interface, permission, network reach, data flow, acceptance criterion, dependency version, task ordering, candidate or rollout changes. No 02.24+ implementation is authorized. The previously made `vsn-network` `needless_return` cleanup remains behavior-neutral and is still directly inside a package being linted.

This addendum exists to preserve scoped acceptance rather than normalize unrelated repository debt into the DNS task.
