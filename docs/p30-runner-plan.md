# P30 Runner Plan — 0.38.1

Host: **linux** · Evidence: **0/21**

| Pack | Mode | Platform | Remaining | Missing required tools | Ready here |
|---|---|---|---:|---|---|
| `linux-core` | ci | linux | 6 | cargo, rustc | no |
| `windows-core` | ci | windows | 4 | cargo, rustc, pwsh, dotnet | no |
| `macos-core` | ci | macos | 4 | cargo, rustc, codesign, pkgbuild, productbuild | no |
| `security-nightly` | ci | linux | 2 | cargo, rustc | no |
| `operations` | live-environment | cross-platform | 4 | none | no |
| `independent-review` | reviewer | cross-platform | 1 | none | no |

## Remaining controls

**linux-core** — `rust-linux`, `desktop-build`, `dashboard-build`, `deb-install-uninstall`, `updater-linux`, `rustsec-audit`
**windows-core** — `rust-windows`, `msi-install-uninstall`, `updater-windows`, `windows-authenticode`
**macos-core** — `rust-macos`, `pkg-install-uninstall`, `updater-macos`, `macos-notarization`
**security-nightly** — `fuzz-remote-protocol`, `fuzz-stream-open`
**operations** — `control-load-slo`, `ha-failover`, `dr-restore`, `vault-key-rotation`
**independent-review** — `penetration-test`
