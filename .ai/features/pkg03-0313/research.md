# PKG-03 03.13 Research — Installer non-mutation boundary

Reviewed: 2026-08-27
Canonical base: `4f5e8ab30f030e758c52c4ca4ac08f73f896247a`
Linear: `ABD-88`
Change required: **false**

## Canonical findings

- 03.06, 03.07 and 03.08 are canonically DONE, so 03.13 is dependency-satisfied and READY on the independent `boundary` lane.
- The parent PKG-03 guardrails explicitly state that firewall/hosts/resolver/trust-store mutation is not an installer side effect.
- 03.13 is a certification boundary task: it must observe and compare Windows system state around accepted installer lifecycles, not add product behavior.
- Windows Firewall persistent policy is the correct high-signal store for installer-created local rules. Microsoft documents `PersistentStore` as the local computer's persistent policy, including rules created manually or programmatically during application installation.
- The Windows hosts file is `%SystemRoot%\System32\drivers\etc\hosts`; exact byte/hash comparison is appropriate because any installer edit is out of scope.
- DNS resolver state is broader than server IPs alone. Snapshot normalized `Get-DnsClientServerAddress`, `Get-DnsClient`, global settings and NRPT state so interface/global resolver mutation cannot be hidden.
- PowerShell's `Cert:` provider exposes `CurrentUser` and `LocalMachine` certificate stores and identifies certificates by thumbprint. Trusted-root/intermediate/publisher/people store membership can therefore be compared semantically.
- Active application launch is not required for this boundary and must remain disabled during installer completion pages to avoid attributing runtime behavior to the installer.
- 03.13 must not repair, reset or mutate any observed system state. A mismatch is evidence of failure, not permission for cleanup.

Official references:
- https://learn.microsoft.com/en-us/powershell/module/netsecurity/get-netfirewallrule
- https://learn.microsoft.com/en-us/powershell/module/dnsclient/
- https://learn.microsoft.com/en-us/powershell/module/dnsclient/get-dnsclientserveraddress
- https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.security/about/about_certificate_provider
- https://learn.microsoft.com/en-us/troubleshoot/windows-server/networking/cannot-modify-hosts-lmhosts-files

## Snapshot model

Each lifecycle records a canonical JSON snapshot before install, after install, and after uninstall.

1. **Firewall**
   - persistent firewall profiles normalized by profile name and relevant policy properties;
   - persistent firewall rules normalized and sorted by stable semantic fields;
   - no rule/profile mutation is permitted.
2. **Hosts**
   - path, existence, byte length and SHA-256;
   - if present, exact SHA-256 must remain identical.
3. **Resolver**
   - interface DNS client settings;
   - interface DNS server addresses;
   - global DNS client settings;
   - NRPT global/rule state where available;
   - normalize ordering and omit volatile formatting-only fields.
4. **Trust**
   - `CurrentUser` and `LocalMachine` stores `Root`, `CA`, `TrustedPublisher`, and `TrustedPeople`;
   - normalize store/location + certificate thumbprint sets;
   - no certificate membership mutation is permitted.

The harness must fail closed on snapshot collection errors for a required surface. An unavailable optional NRPT sub-surface may be represented explicitly as unsupported only if the corresponding cmdlet is genuinely absent on the runner.

`change_required=false`
