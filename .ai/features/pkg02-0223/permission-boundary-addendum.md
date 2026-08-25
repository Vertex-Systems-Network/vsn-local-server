# PKG-02 02.23 Permission Boundary Addendum

Feature ID: `pkg02-0223-test-dns`  
Addendum version: `1.0.0`  
Canonical base: `94feeb8e67dad96ac6a384a8517229ba2c5c38f5`  
Frozen plan SHA-256: `cc9b7b503c87d4ede7fb625e080500049fd0d3c4f0d8cdd956f2d7747c3db9ed`  
Approved: `2026-08-25` by explicit repository-operator continuation after the recorded 02.23 permission blocker.

## Reason

Exact-head acceptance run `32780523514` proved the CLI/harness reached the real product boundary and `vsn dns start` failed with `permission denied: network.manage`.

The repository security model intentionally keeps `NetworkManage` high-risk and outside `Principal::local_authenticated()`. That permission is carried by the OS-elevated `local_network_admin()` path used for resolver/hosts/CA/Caddy mutations. Granting it to ordinary authenticated IPC would widen unrelated privileged capabilities and violate 02.23's explicit non-goals.

VSN-owned managed process lifecycle already uses `ServiceManage` / `ServiceView`. The local DNS responder on a non-privileged loopback UDP port is a VSN-managed child process and does not mutate the OS resolver.

## Approved boundary

For task 02.23 only:

- `dns plan` remains `NetworkView`.
- `dns status` remains `NetworkView`.
- `dns start` requires `NetworkView` and `ServiceManage`.
- `dns stop` requires `ServiceManage`.
- `NetworkManage` remains required for OS resolver mutation and other elevated network-admin operations.
- `Principal::local_authenticated()` is not granted `NetworkManage`.
- No `network-admin`, port 53, hosts, resolver, CA, TLS, Caddy, recursion, forwarding, cache, or public-listener behavior is added.

## Acceptance impact

Frozen behavioral criteria `AC-01` through `AC-12` are unchanged. Task order, denominator, product version, candidate ID, required regression list, and 02.24+ scope are unchanged.

This addendum authorizes only the minimum permission-boundary correction needed for the already-frozen authenticated DNS lifecycle. Any broader permission change requires separate approval.

## Verification

Acceptance must prove on the final exact head that:

- ordinary authenticated IPC can complete `dns start/status/stop/restart`;
- the responder remains loopback-only and non-privileged;
- the ordinary local principal still does not possess `NetworkManage`;
- elevated OS/network mutation paths remain outside 02.23;
- all existing AC-01..AC-12 evidence, regression gates, cleanup, and artifact-integrity requirements pass.
