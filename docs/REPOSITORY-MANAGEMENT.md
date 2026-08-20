# Repository Management

Canonical repository: `Vertex-Systems-Network/vsn-local-server`

## Branch roles
- `main`: integration/stable source only. No package-transfer chunks, caches, build output, or partial generated artifacts.
- `chore/*`: repository hygiene and governance.
- `pkg01/*` … `pkg08/*`: package-scoped implementation/certification.
- `import/*`: temporary import provenance only; never merge transport chunks into `main`.

## Source hygiene
Never commit `target/`, `node_modules/`, generated `dist/`, Python caches, local PKG toolchains/assets, temporary archives, or `repo-import/` transfer data.

## Completion policy
A green helper test does not close a package. A task is DONE only when its acceptance evidence is genuine and, where required, bound to the current release candidate.

## Release identity
`docs/release-candidate-current.json` is the declared release candidate identity. Candidate-bound evidence for another candidate remains historical evidence and does not automatically satisfy the current candidate.

## Pull request discipline
Implementation and cleanup work happens on scoped branches. `main` should receive reviewed, coherent changes. Historical commit messages are not rewritten solely to fix old repository-name references.

## Evidence
Durable certification evidence belongs under `certification/` and must contain enough provenance to resolve the runner, repository, source/candidate identity, and exact tool versions used.
