# `.test` OS resolver integration — 0.19

The VSN DNS responder remains loopback-only. Privileged OS integration additionally requires it on port 53.

- Windows: managed `.test` NRPT rule pointing to loopback.
- macOS: managed `/etc/resolver/test` file.
- Linux: managed systemd-resolved drop-in with `~test` route-only domain.

Apply/remove operations exist only inside the elevated network-admin command boundary. Ordinary Agent IPC does not silently modify system resolver configuration.
