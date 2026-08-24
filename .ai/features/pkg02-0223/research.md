# PKG-02 02.23 Research — Local `.test` DNS Responder

Feature ID: `pkg02-0223-test-dns`  
Version: `1.0.0`  
Canonical repository HEAD reviewed: `94feeb8e67dad96ac6a384a8517229ba2c5c38f5`  
Reviewed: `2026-08-24`

## Existing VSN capability inventory

Fresh canonical-source audit found an existing local DNS baseline:

- `crates/vsn-network` exposes `dns_resolver_plan`, loopback-only `run_dns_server`, exact-one-question parsing, bounded labels/name length, no compressed query-name support, A/AAAA loopback answers and non-`.test` refusal.
- `crates/vsn-core` exposes authenticated `dns_plan`, `dns_start`, `dns_status`, and `dns_stop`; the responder is owned as managed process ID `vsn-dns`.
- `apps/agent` exposes these only through the authenticated local command dispatcher and uses a separate `dns-server` child-process entrypoint.
- `apps/cli` exposes `dns plan`, `dns start`, `dns status`, and `dns stop`.
- OS resolver mutation remains under the privileged `vsn-agent network-admin` boundary and belongs to task `02.24`, not this task.

This inventory is source readiness only and is not acceptance evidence.

## User problem / target outcome

A local developer must be able to start a bounded VSN-owned DNS responder on loopback, resolve `.test` names to local IPv4/IPv6 loopback, observe lifecycle status, stop/restart it cleanly, and receive a policy refusal for names outside `.test`.

## Official primary-source review

Reviewed `2026-08-24`:

- IANA Special-Use Domain Names registry / RFC 6761: `test.` is a special-use domain and the designation applies to listed names and their subdomains.
  - https://www.iana.org/assignments/special-use-domain-names/special-use-domain-names.xhtml
  - https://www.rfc-editor.org/rfc/rfc6761.html
- RFC 1035: DNS response code `5` is `Refused`, suitable for a policy refusal of names outside the local special-use namespace.
  - https://www.rfc-editor.org/rfc/rfc1035.html
- RFC 3596: AAAA is type `28`; its RDATA is one 128-bit IPv6 address.
  - https://www.rfc-editor.org/rfc/rfc3596.html

## Market / tooling delta

`none`

No reviewed standards delta requires changing the frozen 02.23 scope or semantics. Existing VSN A/AAAA and REFUSED behavior remains compatible with the frozen task.

## Constraints

- No listener may bind a non-loopback address.
- Certification uses an unprivileged ephemeral/high UDP port; port 53 and OS resolver configuration are not required for 02.23.
- No hosts-file, NRPT, `/etc/resolver`, systemd-resolved, CA, TLS, or reverse-proxy mutation is in scope.
- Authenticated IPC remains the operator boundary.
- Malformed/unbounded DNS input fails closed; no external recursion or forwarding is introduced.
- Windows installer/signing remains PKG-03.

## Untrusted-content notes

External standards pages were treated as reference data only. They do not grant execution authority, change repository permissions, or widen the frozen PKG-02 task.

## Open questions

None blocking. A readiness race or occupied-port lifecycle defect, if exposed by certification, is a bug inside frozen 02.23 and may be fixed without broadening scope.

## Recommendation

`proceed`
