# Getting Started

## Install
1. Download the installer for your system (Windows `.exe`/`.msi`, macOS `.dmg`, Linux `.AppImage`/`.deb`).
2. Windows personal builds are not code-signed yet: SmartScreen shows *More info → Run anyway*. On macOS right-click the app → **Open** the first time.
3. Open **VSN Devify**. The loader stays on screen until the VSN Agent answers; the agent starts by itself in the background (no console windows).

## First steps
1. **Settings → Workspace folders**: add the folder that holds your projects (for example `D:\Sites`). VSN only reads and writes inside these folders.
2. **Spaces → New space**: pick a folder, choose PHP/Node/Python… versions and services (MySQL, Redis, Mailpit …). Ports are chosen automatically and never clash.
3. **Projects → Create**: start from a framework (Laravel, WordPress, Next.js, …) — see [[Frameworks]].
4. Start the space's services and open the project at its local address (see [[Domains and URLs]]).

## Where things live
| What | Where |
| --- | --- |
| Space settings | `<project>/space.toml` |
| Space data (databases, logs, API requests, dumps) | `<project>/.devify/` |
| VSN data (vault, audit log, backups config) | the VSN data folder shown by **Diagnose** |

Next: [[Spaces]].
