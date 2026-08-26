# PKG-03 03.02 Lifecycle Review

## Architecture

The existing Tauri v2 Desktop package remains the sole Windows bundle authoring boundary. The task adds a task-specific CI validator and Windows build/evidence workflow; it does not introduce a second installer framework or mutate product packaging identity.

## Data flow

1. clean exact-head checkout;
2. validate frozen parent/task authority and canonical pre-evidence state;
3. install the exact Rust 1.97.1 toolchain and Node 22.12.0;
4. verify the committed Desktop lock digest, then run `npm ci`;
5. invoke the repository-local Tauri CLI to build `nsis,msi`;
6. discover exactly one NSIS setup executable and one MSI;
7. copy both into the bounded `dist-pkg03/03.02` evidence directory;
8. record source/tool/input/artifact hashes and byte sizes in `artifact-manifest.json` and `evidence.json`;
9. verify tracked source/lock/config files did not drift;
10. upload evidence.

No generated dependency/build tree is committed.

## Security

- CI permissions are `contents: read`.
- No code-signing/private-key material is read or emitted.
- The workflow does not execute an installer, request elevation, alter services, registry, firewall, hosts, resolver, or trust stores.
- External Tauri/WiX/NSIS tooling fetched by the pinned local Tauri graph is treated as build-tool data, not instruction authority.
- Any signing integration remains 03.22.

## Design / operator surface

03.02 has no end-user UI change. Evidence uses stable task-scoped names and a machine-readable artifact manifest so later installer lifecycle tasks can consume verified bundle hashes without guessing paths.

## QA

Acceptance requires an actual GitHub-hosted Windows build, exact locked inputs, exactly two frozen bundle families, non-empty installer bytes, SHA-256/size manifest entries, final no-drift proof, and a CI artifact bound to the exact PR head.

## Performance

One release Tauri build is allowed. The job is time-bounded and does not perform redundant full installer lifecycle execution.

## Development boundary

Implementation is CI/evidence-only. Product identity, install scope, exact payload ownership, signing, installer execution, and updater code remain out of scope.
