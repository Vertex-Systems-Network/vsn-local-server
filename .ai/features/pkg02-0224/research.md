# PKG-02 02.24 Research — Local Domain/HTTPS and Privileged Network Boundary

Feature ID: `pkg02-0224-domain-https`  
Version: `1.0.0`  
Canonical repository HEAD reviewed: `265bd17895231fc145ccd435c48def0a38bfd98d`  
Reviewed: `2026-08-25`

## Canonical task

`02.24 — Local domain/HTTPS planning and privileged network boundary: domain plan, hosts apply/remove/reload behavior and fail-closed elevation requirements.`

Canonical PKG-02 state at review: `23/27 = 85.19%`, active `02.24`. Tasks `02.01`–`02.23` are integrated DONE. `02.25+` remain blocked.

## Existing VSN capability inventory

Fresh canonical-source audit found an existing local-domain/HTTPS baseline:

- `crates/vsn-network` validates `.test` domains, creates `DomainPlan`, mutates a VSN-managed block in the OS hosts file, renders Caddy configuration, generates mkcert leaf certificates, and runs bounded Caddy validate/reload helpers.
- `crates/vsn-core` exposes `domain_plan` under `NetworkView`; hosts apply/remove and Caddy reload/proxy/CA operations require `NetworkManage`.
- `crates/vsn-policy::Principal::local_authenticated()` intentionally lacks high-risk `NetworkManage`; `Principal::local_network_admin()` owns it.
- `apps/agent` exposes ordinary authenticated IPC commands `domain.plan`, `domain.apply-hosts`, `domain.remove-hosts`, and `domain.reload`, but apply/remove/reload fail policy for the ordinary local principal.
- `apps/agent network-admin ...` is the separate OS-elevated operator boundary.
- `apps/cli` exposes domain plan/apply/remove/reload as authenticated IPC calls. Their presence is not authority to mutate: Core policy remains decisive.

This inventory is source readiness only, not acceptance evidence.

## Current source defects / safety gaps

### 1. Hosts read failures are not fail-closed

Both hosts apply and remove currently use `fs::read_to_string(...).unwrap_or_default()`. An unreadable or invalid-UTF-8 hosts file can therefore be treated as empty content, creating a destructive overwrite risk.

Required correction: any read/decoding failure must return an error and preserve original bytes.

### 2. Hosts replacement has a destination-disappearance window

Current `atomic_write` syncs a temporary file, then explicitly deletes the destination before rename. A failure/crash between delete and rename can leave the hosts file absent.

Required correction: never pre-delete the destination. Replacement must be a same-directory, replace-existing operation with failure behavior that leaves the original target present. On Windows, preserve original file security/metadata where the native replacement primitive supports it.

### 3. Removal lacks a path-injected test seam

Apply has `apply_hosts_domain_at(path, ...)`; remove targets only the real system hosts path. Deterministic acceptance must not mutate the machine hosts file.

Required correction: introduce a path-scoped remove seam used by production and sandbox acceptance.

### 4. HTTPS local-CA trust must not expand privilege silently

`render_caddyfile` can emit `tls internal`. Caddy documents that its internal CA attempts to install its root into trust stores unless the global `skip_install_trust` option is configured.

Required correction: VSN-generated Caddy configuration must explicitly suppress automatic trust-store installation. Explicit CA trust actions (`caddy trust`, `mkcert -install`, equivalent OS trust-store mutation) remain privileged external actions and are not performed by normal 02.24 acceptance.

### 5. Elevation detection is fail-closed but heuristic

Windows `network-admin` currently uses `net.exe session` success as its elevation test. Command failure maps to deny, so it is fail-closed, but Microsoft exposes `TOKEN_ELEVATION` as the direct token-elevation signal.

02.24 must prove no high-risk mutation is reachable through ordinary authenticated IPC. Replacing the heuristic is optional unless acceptance demonstrates a correctness defect; any permission broadening is forbidden.

## Historical preparation review

Open PR #58 contains useful historical 02.24 harness/safety ideas, including the two hosts-file safety defects above. It was prepared against stale stacked state and is not current acceptance authority. In particular, its assumption that ordinary CLI must not contain apply/remove commands is superseded by current architecture: those commands exist but are denied by `NetworkManage` policy for `local_authenticated`.

## Official primary-source review

Reviewed `2026-08-25`:

- IANA Special-Use Domain Names / RFC 6761: `test.` and names beneath `.test.` are special-use.
  - https://www.iana.org/assignments/special-use-domain-names/special-use-domain-names.xhtml
  - https://www.rfc-editor.org/rfc/rfc6761.html
- Rust `std::fs::rename`: replaces an existing destination; on Windows it uses `MoveFileExW` with a fallback to `SetFileInformationByHandle`, with Windows 10 1607+ Unix-like replacement semantics where supported.
  - https://doc.rust-lang.org/stable/std/fs/fn.rename.html
- Microsoft `ReplaceFileW`: combines replacement steps and preserves original-file metadata including DACLs/security attributes where supported.
  - https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew
- Microsoft `TOKEN_ELEVATION`: directly indicates whether a token has elevated privileges.
  - https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-token_elevation
- Caddy `tls internal`: uses Caddy's internal CA and may attempt root trust installation.
  - https://caddyserver.com/docs/caddyfile/directives/tls
- Caddy global `skip_install_trust`: suppresses attempts to install the local CA root into system/Java/Firefox trust stores.
  - https://caddyserver.com/docs/caddyfile/options
- mkcert: `-install` installs a local CA into trust stores; its local root key is high impact and must not be exposed.
  - https://github.com/FiloSottile/mkcert/blob/master/README.md

## Market / tooling delta

`none`

No reviewed official-source change requires widening the frozen 02.24 task. The findings above are current-source safety gaps and privilege-boundary clarifications inside the frozen task.

## Constraints

- `.test` remains the only accepted local domain suffix.
- Domain planning remains non-mutating and available through authenticated IPC.
- Ordinary authenticated IPC must remain unable to obtain `NetworkManage`.
- System hosts, trust stores, resolver policy, CA installation, or other OS-global network state must not be mutated during normal certification.
- Hosts semantics are certified against a disposable path using the production mutation implementation.
- Caddy validation/reload is certified with a deterministic fake/local helper seam and VSN-owned sandbox configuration, not by trusting a machine-global Caddy state.
- Any actual privileged system mutation requires a separate explicit operator approval.
- No port-53/NRPT/resolver acceptance is added to 02.24.
- No 02.25+ work.
- Windows installer/signing remains PKG-03.

## Recommendation

`proceed_to_frozen_plan`
