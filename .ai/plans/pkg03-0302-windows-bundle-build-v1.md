# PKG-03 03.02 — Deterministic Windows Bundle Build Plan v1

Linear issue: `ABD-77`
Canonical base: `9d33682f7c0cc30080792493c8f760f3fd120759`
Parent frozen plan: `.ai/plans/pkg03-windows-installer-v1.md`
Parent plan SHA-256: `9de2c38412813907637e01d4ce75869033ba5b02e3bbd4588342f09e1062a16e`

## Objective

Prove that the current accepted source can produce the two frozen Windows installer families on a clean GitHub-hosted Windows x64 runner using locked repository inputs, and emit a machine-readable manifest binding those installer bytes to source, candidate, toolchain and input digests.

## Frozen acceptance criteria

1. **AC-01 Authority binding** — exact canonical base, parent plan digest, 03.01 architecture contract and task identity are validated before build.
2. **AC-02 Runner boundary** — acceptance runs on GitHub-hosted Windows 2025 x64 with read-only repository permissions.
3. **AC-03 Locked npm input** — Node `22.12.0` is used; `apps/desktop/package-lock.json` must have SHA-256 `b2f41ab8c7a116cb9c78d41fd8036e7e1b1307bc3b78cd9a33ef37d5911c0aa6`; `npm ci --no-audit --no-fund` is used and may not rewrite package/lock files.
4. **AC-04 Locked Rust input** — Rust/Cargo `1.97.1` is installed and the Cargo graph is consumed with `--locked`.
5. **AC-05 Repository-local Tauri** — the Tauri CLI comes from the committed Desktop npm graph; no globally installed or ad-hoc installer CLI is authoritative.
6. **AC-06 Frozen bundle command** — build exactly `nsis,msi` from `apps/desktop` using the accepted Tauri v2 configuration.
7. **AC-07 Real bundle outputs** — the build must produce exactly one NSIS `*-setup.exe` and exactly one `.msi` beneath the Cargo release bundle tree.
8. **AC-08 Byte evidence** — each installer must be non-empty and its original relative path, stable evidence filename, byte size and SHA-256 are recorded.
9. **AC-09 Artifact manifest** — `artifact-manifest.json` binds source head, product version, release candidate, runner/tool versions, input digests, build command and both installer entries.
10. **AC-10 Evidence binding** — `evidence.json` records all acceptance booleans and the SHA-256 of `artifact-manifest.json`; both real installer files are uploaded in the task CI artifact.
11. **AC-11 No unintended mutation** — tracked repository content, package manifests and lockfiles are unchanged after build; no installer is executed and no privileged/system/signing mutation occurs.
12. **AC-12 Final exact-head gate** — AI Planning Governance, Repository Governance, PKG-03 Acceptance Sequence and the dedicated 03.02 Windows build workflow must pass on the final PR head before merge.

## Explicit non-goals

03.02 does not define publisher/upgrade identity (03.03), elevation/install scope (03.04), exact payload ownership (03.05), install/uninstall lifecycle (03.06–03.08), Authenticode signing (03.22), or updater behavior (PKG-04).

## Exit state

Only after genuine exact-head task evidence passes may canonical state advance to `done=2/25`, `03.02=DONE`, with `03.03`–`03.05` remaining READY and deterministic cursor `03.03`.
