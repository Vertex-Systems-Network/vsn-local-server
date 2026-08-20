# VSN security / load test gate — 0.17

The repository contains two bounded `cargo-fuzz` targets outside the main workspace: remote protocol JSON decoding and stream-open validation. A scheduled/manual GitHub workflow runs each target with a fixed 90-second budget and also runs RustSec audit.

`scripts/load-control-plane.py` is an opt-in live probe for a running Control Plane. It caps concurrency/requests, reports mean/p50/p95/p99 latency and fails when configured error-rate or p95 thresholds are exceeded. It does not create accounts, commands or cloud resources.

These source gates are not a penetration test and are not claimed as green until CI/live targets actually run. Production release still requires multi-node failure injection, authentication abuse tests, long-duration relay soak, updater crash/reboot tests and independent security review.
