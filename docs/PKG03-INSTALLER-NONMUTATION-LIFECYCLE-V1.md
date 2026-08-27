# PKG-03 Windows Installer Non-Mutation Contract v1

Task: `03.13`
Linear: `ABD-88`

## Boundary

The installer is not authorized to modify Windows firewall policy, hosts mappings, DNS resolver configuration, or certificate trust stores as an installation or uninstallation side effect.

## Required evidence

For NSIS current-user, NSIS per-machine and MSI/WiX per-machine lifecycles, capture normalized snapshots at:
- baseline before install;
- post-install after installer exit with application launch disabled;
- post-uninstall after uninstaller exit.

Both post snapshots must be semantically identical to baseline for every protected surface.

## Protected state

### Firewall
Persistent local firewall profile/rule state. Installer-created rules in the local persistent policy are forbidden.

### Hosts
`%SystemRoot%\System32\drivers\etc\hosts` existence and exact SHA-256/content identity.

### Resolver
DNS client interface settings, configured DNS server addresses, global DNS client settings and NRPT state where the platform exposes it.

### Trust
Certificate membership, by thumbprint, for `Root`, `CA`, `TrustedPublisher` and `TrustedPeople` under both `CurrentUser` and `LocalMachine`.

## Fail-closed rules

Evidence fails if:
- any required snapshot cannot be collected;
- any normalized protected surface differs from baseline after install or uninstall;
- the harness repairs or mutates protected state;
- application launch is allowed to contaminate installer-only attribution;
- task implementation changes product/Tauri/installer templates or crosses into service/ACL/signing/updater scope.

Unsupported optional NRPT cmdlets may be represented explicitly only when absent on the runner and must remain consistently absent across phases.
