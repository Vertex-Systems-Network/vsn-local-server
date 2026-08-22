# Code Quality Gate

VSN keeps code-quality checks separate from package acceptance evidence. Package certification proves task behavior; this gate continuously detects implementation and dependency quality regressions.

## Blocking CI checks

- Existing build-foundation Rust checks remain authoritative for formatting and compiler lint quality: `cargo fmt --all -- --check` and workspace-wide Clippy with `-D warnings`.
- `cargo-deny` enforces dependency source policy and rejects wildcard dependency specifications. Duplicate versions are surfaced as warnings until the existing dependency graph is deliberately consolidated.
- `actionlint` validates every GitHub Actions workflow, including shell-expression integration.
- Desktop uses the committed npm lockfile, strict TypeScript typechecking, and a production build.

## Report-only debt checks

The following checks run on every relevant pull request but are initially report-only so newly published advisories or legacy dependency debt cannot silently halt unrelated package certification:

- RustSec advisories through `cargo-deny check advisories`.
- unused direct Rust dependencies through `cargo-machete`.
- high-severity production npm advisories through `npm audit --omit=dev --audit-level=high`.

These checks must remain visible. Once the current baseline is clean and exceptions are explicitly documented, each report-only check should be promoted to blocking rather than ignored.

## Local Windows execution

Run:

```powershell
pwsh -NoProfile -File scripts/code-quality.ps1
```

To make debt-reporting checks blocking locally:

```powershell
pwsh -NoProfile -File scripts/code-quality.ps1 -StrictAdvisories -StrictUnusedDependencies -StrictNpmAudit
```

The local script requires the repository Rust toolchain (`rustc`/`cargo` 1.97.1), `cargo-deny`, and `cargo-machete`. If `actionlint` is installed locally it is also enforced; CI always runs the pinned actionlint container.

## Scope

Quality tooling does not change the frozen PKG task denominator and does not by itself certify a PKG task. A package task is counted only after its own evidence-bound acceptance workflow passes.
