# PKG-02 02.27 Research — Fresh-State Local Beta Final Gate

Reviewed: 2026-08-26
Canonical main: `e6e981f106ff3685ab1694261991e5e97a3b738d`
Feature: `pkg02-0227-fresh-state-local-beta-final-gate`

## Canonical findings

- Live machine state is PKG-02 `26/27 = 96.30%`, `complete=false`, active `02.27`.
- The package tracker contains 02.01–02.26 as DONE and 02.27 as IN_PROGRESS.
- Product remains `0.38.1`; candidate remains `c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474`.
- `.ai/state.json` explicitly makes live main authoritative and requires fresh canonical refresh, market-delta review, a frozen feature manifest and stop-and-reconcile on mismatch.

## Stale preparation PR #61

PR #61 is useful historical preparation but not an execution baseline:
- base SHA `5854227e7810348b95848bd0e25b21a53c652082`, which predates current main;
- head `dda32203672543dd7dcf3daa9e479469d1f8febb`;
- only two files: a 02.27 workflow and PowerShell scaffold;
- correctly checks tracker 26/27, clean checkout, Rust full-workspace commands, Desktop locked build, CLI version/help and post-run drift;
- insufficient for the frozen final task because `version`/`help` do not prove CLI + Desktop end-to-end smoke over all accepted local capabilities and it does not bind the hardened predecessor capability matrix.

Decision: do not rebase/merge PR #61 as implementation authority. Preserve it as historical research only.

## Existing certification architecture

Current main already has dedicated, exact-source GitHub-hosted Windows workflows/harnesses for the high-risk local capability families. The accepted sequence has repeatedly used a frozen same-head regression subset to avoid replacing hardened task-specific evidence with weaker duplicated checks.

For 02.27, the strongest final architecture is:
- one new dedicated integrated fresh-state gate for tracker/source/build/CLI/Desktop/cleanliness/evidence;
- same-head execution of the established hardened regressions for bootstrap, diagnostics, workspace files, binary transfer, terminal modes, preview, DNS, domain/HTTPS, SQLite and external/native databases.

This satisfies the breadth requirement without a monolithic script that reimplements security-sensitive harnesses.

## Market delta

Official current references reviewed:
- Laravel Herd for Windows: https://herd.laravel.com/windows
- DDEV: https://docs.ddev.com/en/stable/
- DDEV project configuration: https://docs.ddev.com/en/stable/users/configuration/config/
- GitHub-hosted runners: https://docs.github.com/en/actions/reference/runners/github-hosted-runners
- GitHub self-hosted runners: https://docs.github.com/en/actions/concepts/runners/self-hosted-runners
- GitHub secure use: https://docs.github.com/en/actions/reference/security/secure-use

Observed direction:
- modern local-dev environments emphasize managed local services and reproducible/shareable project configuration;
- DDEV persists project configuration in version-controlled files and treats start/stop/remove as reproducible project lifecycle;
- Herd exposes multiple runtime versions and local services including MySQL/MariaDB/PostgreSQL/Redis;
- GitHub-hosted standard runners provide a fresh VM per job, whereas self-hosted runners do not require a clean instance for each job and are weaker isolation for a public repository.

Decision: `certification_reproducibility_only_no_roadmap_expansion`.

No new VSN product capability is justified or permitted in 02.27. Fresh-state exact-source certification, clean-runner isolation, no-drift evidence and integrated proof of the already accepted capability breadth are the only material delta.

## Risks identified

- false completion from build-only or `version/help` smoke;
- stale branch/base assumptions from PR #61;
- accidental lockfile mutation during npm/cargo operations;
- hidden runner residue if certification moves to self-hosted infrastructure;
- a final gate that duplicates prior harnesses but weakens their negative/security cases;
- marking PKG-02 complete inside the implementation PR rather than through a separate state projection.

The frozen plan addresses each risk explicitly.
