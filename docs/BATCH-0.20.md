# VSN Batch 0.20 — Speed-first completion sprint

## Runtime audit

`runtime.audit` / `vsn runtime audit` performs a read-only registry health pass for missing installation directories/executables, executable path escape, invalid source SHA metadata and dangling project activations. It complements `runtime repair`; audit does not mutate state.

## Advanced DB models

`database.model.analyze` / `vsn db model-analyze` accepts bounded samples for document, key-value, vector and graph models. It reports type distributions, duplicate/dangling relationships, TTL usage, vector dimensionality/norms and heterogeneous document fields without guessing unknown database protocols.

## Container registry publish

`container.registry-publish` tags an existing Docker/Podman image and optionally pushes it. Source/target/backend are validated argv values, execution is bounded, shell interpolation is absent, and credentials stay in the container CLI's configured credential context.

## Marketplace publisher governance

Signed indexes may declare publishers with `active`, `suspended` or `retired` state and per-publisher allowed channels. Suspended/retired/disallowed-channel entries are excluded from search and update resolution. Legacy indexes without a publisher table remain backward compatible.

## AI candidate ToolPlan boundary

Externally produced structured ToolPlans are validated before execution: protocol version, intent, tool count, command/permission identifiers, parameter budgets, recursion prohibition, unrestricted-shell invariant and mutation confirmation are checked. A model response is never treated as raw shell.

## Shared Team Vault

The Control Plane adds a separate shared Vault trust domain:

- shared PostgreSQL stores only name, nonce, ChaCha20-Poly1305 ciphertext, creator and timestamp;
- `VSN_CONTROL_VAULT_KEY_B64` is separate from `VSN_CONTROL_AUTH_KEY_B64`;
- `control.vault.use`, `control.vault.manage` and `control.vault.reveal` separate metadata/write/reveal authority;
- Dashboard supports list/set/reveal/delete;
- no plaintext fallback is written into shared state.

Organization/HSM-backed automatic Team Vault key rotation is still pending.

## Release runner preflight

`scripts/release-preflight.py` inventories Rust/Node/package/signing/container/sandbox tooling for the current host. `--strict` can fail a runner with missing host-required build tools. It deliberately does **not** modify the 21-control release evidence ledger.
