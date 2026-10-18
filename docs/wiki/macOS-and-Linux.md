# macOS and Linux

Personal unsigned builds are available for macOS (Apple silicon `.dmg`) and Linux (`.AppImage`, `.deb`). They include the VSN Agent and the `vsn` command line.

- **macOS**: right-click → **Open** the first time (not notarized yet). Domain routing asks for your admin password.
- **Linux**: `chmod +x` the AppImage, or install the `.deb`. Domain routing uses pkexec (polkit).

Feature parity with Windows is in progress (DVF-13); please report anything that behaves differently.
