# PKG-03 MSIX / Microsoft Store preimplementation

Status: **NON_ACCEPTANCE_PREIMPLEMENTATION / NO STORE IDENTITY / NO PACKAGE / NO SIGNING / NO SUBMISSION**

Reviewed: 2026-09-16.

## Current repository boundary

The Tauri desktop process is a current-user UI shell and delegates control operations to the authenticated VSN Agent over IPC. The Agent owns privileged Windows capabilities, including service management and explicit elevated network administration for hosts, local CA and resolver operations. Therefore this lane does not reinterpret the existing app as a standalone Store-only product.

The safe distribution model remains:

- **Store shell:** current-user MSIX UI package, package identity, no fabricated production identity, dependent on an accepted external/pre-provisioned authenticated Agent boundary;
- **Direct lane:** existing NSIS current-user, NSIS per-machine and MSI packages remain available for the privileged Agent/service lifecycle and enterprise installation semantics.

## Current Microsoft packaging path used for preimplementation

Microsoft's current WinApp CLI Tauri guide documents a Tauri-to-MSIX flow in which a package manifest and assets are prepared, the release executable is staged, and WinApp CLI packages the staged application. Microsoft also documents full-trust desktop MSIX manifests with the `runFullTrust` restricted capability.

References re-checked on 2026-09-16:

- https://github.com/microsoft/WinAppCli/blob/main/docs/guides/tauri.md
- https://learn.microsoft.com/windows/msix/desktop/desktop-to-uwp-manual-conversion
- https://learn.microsoft.com/windows/msix/msix-container

These references inform packaging mechanics only. Final Store eligibility/certification remains an external acceptance gate and must be re-checked when real submission begins.

## Files

- `apps/desktop/src-tauri/msix/store-packaging-inputs.json.template` — fail-closed packaging inputs with real Partner Center identity/publisher intentionally absent.
- `apps/desktop/src-tauri/msix/Package.appxmanifest.template.xml` — deterministic full-trust desktop manifest template.
- `scripts/ci/pkg03-msix-store-compat.py` — proves the current desktop/Agent privilege split from repository source.
- `scripts/ci/pkg03-msix-manifest-preimplementation.py` — fixture-only renderer that produces a deterministic manifest and staging plan without accepting real production identity.

## Activation requirements

Before a real Store package can be treated as acceptance evidence, a later FULL GATE must supply and independently verify at least:

1. accepted Partner Center package identity and publisher values;
2. production-quality Store asset set and exact asset hashes;
3. exact final release executable bound to the accepted source/candidate chain;
4. an accepted Agent distribution/provisioning model that preserves required privileged capabilities;
5. Windows packaging validation using the chosen current Microsoft tooling;
6. Store certification/signing evidence for the exact submitted package;
7. clean install/launch/uninstall and IPC-to-Agent compatibility evidence;
8. no regression in the direct NSIS/MSI lane.

## Explicit non-authority

The fixture identity `VSN.FastEpochFixture` and publisher `CN=VSN Fast Epoch Fixture` are synthetic test values only. They are never written into the production input template, never count as Partner Center identity, and cannot satisfy signing, Store, SmartScreen, release or canonical task acceptance.
