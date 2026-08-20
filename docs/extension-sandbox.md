# Executable extension sandbox — 0.19

Signed extensions may declare bounded executable entries when `process.execute` is present. Before execution VSN re-verifies the installed manifest against the recorded signer and canonicalizes the executable path inside the extension root.

The 0.19 executable backend is Linux Bubblewrap. The extension root is mounted read-only, `/tmp` is temporary, network is denied unless declared, workspace access is read-only/read-write according to the manifest, environment is cleared, arguments are passed directly without a shell, and runtime/stdout/stderr are bounded.

Windows and macOS executable backends are intentionally unavailable in this batch rather than falling back to unsandboxed execution.
