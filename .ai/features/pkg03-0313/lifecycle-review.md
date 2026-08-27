# PKG-03 03.13 Lifecycle Review — Installer non-mutation

## Lifecycle coverage

03.13 reuses the already-accepted installer families and measures system state around each lifecycle:

1. NSIS current-user install -> snapshot compare -> uninstall -> snapshot compare.
2. NSIS per-machine install -> snapshot compare -> uninstall -> snapshot compare.
3. MSI/WiX per-machine install -> snapshot compare -> uninstall -> snapshot compare.

For every lifecycle:
- capture baseline before installer launch;
- keep post-install application-launch options disabled;
- capture post-install snapshot after the installer process terminates;
- require baseline == post-install for every protected surface;
- perform the accepted uninstall flow;
- capture post-uninstall snapshot;
- require baseline == post-uninstall;
- never auto-repair a mismatch.

## Protected surfaces

- Windows Firewall persistent profiles/rules.
- `%SystemRoot%\System32\drivers\etc\hosts` exact content hash.
- DNS client/interface/server/global/NRPT configuration.
- CurrentUser and LocalMachine certificate trust-store membership.

## Normalization rules

- JSON object keys and collections are deterministically sorted.
- Firewall rule comparison uses semantic fields, not display ordering.
- DNS interface collections are keyed by stable interface identity/address family and sorted server-address sets.
- Certificate collections are keyed by location/store and sorted thumbprints.
- No timestamp, process ID, formatting order or transient cache content is included.
- Snapshot collector failures are fatal unless a specifically optional cmdlet is absent and the snapshot records that absence identically across phases.

## Nonclaims

03.13 does not certify:
- service registration/removal (03.11);
- ACL/state separation (03.12);
- repair/integrity (03.14);
- installer diagnostics/cancellation semantics (03.15);
- silent deployment, signing, updater or recovery behavior.

## Exit rule

Only exact-head evidence proving all three lifecycles preserve all protected surfaces can mark 03.13 DONE. State reconciliation must re-read live `main` and must not assume concurrent 03.09/03.10 outcomes.
