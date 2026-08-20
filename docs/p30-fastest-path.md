# P30 Fastest Completion Path — 0.38.1

Current valid evidence: **0/21**.

| Pack | Mode | New controls | Cumulative | P30 exact | Overall exact |
|---|---|---:|---:|---:|---:|
| `linux-core` | ci | 6 | 6/21 | 75.71% | 99.22% |
| `windows-core` | ci | 4 | 10/21 | 82.19% | 99.43% |
| `macos-core` | ci | 4 | 14/21 | 88.67% | 99.63% |
| `security-nightly` | ci | 2 | 16/21 | 91.90% | 99.74% |
| `operations` | live-environment | 4 | 20/21 | 98.38% | 99.95% |
| `independent-review` | reviewer | 1 | 21/21 | 100.00% | 100.00% |

## Controls

**linux-core** — `rust-linux`, `desktop-build`, `dashboard-build`, `deb-install-uninstall`, `updater-linux`, `rustsec-audit`
**windows-core** — `rust-windows`, `msi-install-uninstall`, `updater-windows`, `windows-authenticode`
**macos-core** — `rust-macos`, `pkg-install-uninstall`, `updater-macos`, `macos-notarization`
**security-nightly** — `fuzz-remote-protocol`, `fuzz-stream-open`
**operations** — `control-load-slo`, `ha-failover`, `dr-restore`, `vault-key-rotation`
**independent-review** — `penetration-test`
