# Local `.test` DNS responder

VSN 0.18 can run a loopback-only UDP DNS responder, default `127.0.0.1:53535`. It accepts exactly one uncompressed question, answers `.test` A with `127.0.0.1` and AAAA with `::1`, returns no answer for other `.test` types, and refuses non-`.test` names. Packet parsing and response sizes are bounded.

`vsn dns plan` reports the OS-specific privileged resolver-routing step. `vsn dns start/status/stop` manages the local responder as a VSN-owned child process. The baseline does **not** silently rewrite the machine DNS resolver; that remains behind the elevated network-admin boundary.
