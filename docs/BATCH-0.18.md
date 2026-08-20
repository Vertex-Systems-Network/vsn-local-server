# VSN Batch 0.18

Completion sprint additions:

- **Vault rotation** — Vault v2 stores a secure-store `key_id`; `vault rotate` decrypts every entry with the current key, generates a fresh random key ID/key, re-encrypts all entries, and commits the new vault atomically. The old secure-store key remains available for recovery rather than being destructively retired in the same transaction.
- **AI execution** — `ai execute` consumes the deterministic VSN plan, validates every command/declared permission pair, refuses recursive AI calls and unrestricted shell, requires `confirm_mutations=true` before any mutating plan begins, stops on first failed tool, and caps result bytes.
- **Marketplace revocation/update** — signed indexes can carry version revocations. Revoked versions disappear from search and `marketplace resolve-update` returns only non-revoked candidates.
- **Declarative extension providers** — `extension providers` re-verifies the installed extension manifest against the signer recorded at installation and resolves bounded provider JSON paths that remain inside the extension root.
- **Local .test DNS** — a loopback-only UDP responder answers `.test` A/AAAA queries and refuses external domains. `dns plan/start/status/stop` manages the responder; actual OS resolver routing remains an explicit privileged step.
- **Control Plane readiness/ops** — `/ready` checks shared PostgreSQL heartbeat/cluster visibility when configured; `/v1/admin/ops` exposes bounded operational counts and configured SLO targets to authorized audit principals.
- **P30 release evidence** — `scripts/release-evidence.py` defines 21 mandatory stable-release evidence controls and can init/record/merge/evaluate evidence. The shipped template is intentionally uncertified.
