# PKG-03 03.13 — Installer Non-Mutation Boundary Plan v1

## Goal

Certify that accepted Windows installers do not silently mutate firewall, hosts, resolver or certificate trust state during install or uninstall, without adding any product mutation to satisfy the test.

## Acceptance sequence

1. Validate canonical 03.13 authority, dependency state and frozen planning digests.
2. Build the accepted exact-head Windows installer formats using the canonical product configuration and locked toolchain.
3. Implement a deterministic read-only Windows state collector for:
   - persistent firewall profiles and rules;
   - hosts file exact SHA-256;
   - DNS client/server/global/NRPT configuration;
   - CurrentUser and LocalMachine trusted certificate store membership.
4. Normalize snapshots into stable JSON and hash each snapshot.
5. Exercise the accepted NSIS current-user interactive lifecycle with application launch disabled:
   - baseline snapshot;
   - install;
   - post-install snapshot and exact semantic equality assertion;
   - uninstall;
   - post-uninstall snapshot and exact semantic equality assertion.
6. Exercise the accepted NSIS per-machine lifecycle with the same three-snapshot equality contract.
7. Exercise the accepted MSI/WiX per-machine lifecycle with the same three-snapshot equality contract.
8. Fail closed on any protected-state difference. Do not attempt to restore or repair changed system state inside the certification harness.
9. Prove no task product/config/template/service/ACL/signing/updater mutation and zero tracked repository drift.
10. Bind evidence to exact source SHA, workflow run/job/artifact and snapshot digests.
11. Only after genuine acceptance, reconcile PKG-03 against live `main`; preserve all concurrent-lane evidence.

## Concurrency rule

03.13 starts from canonical `main` at 8/25 and depends only on canonically DONE 03.06, 03.07 and 03.08. It must not merge, copy or assume branch-local 03.09 or 03.10 results.

## Expected implementation surfaces after planning approval

- `scripts/ci/pkg03-0313-*`
- `.github/workflows/pkg03-0313-*`
- tracker/master state only after exact-head acceptance

No Tauri config, installer template, application source, firewall/hosts/DNS/certificate state, service, ACL, signing or updater source mutation is authorized.
