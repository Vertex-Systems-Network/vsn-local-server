# VSN Devify — product plan (v1)

Status: owner-requested product direction, 2026-10-17. This plan adds a **product track** (DVF-01 … DVF-14) beside the frozen hardening packages (PKG-03 … PKG-08). It does not change the frozen PKG DAG, tracker or denominators. DVF work runs in the `NON_ACCEPTANCE_PREIMPLEMENTATION` lane until each package gets its own acceptance contract.

## 1. Goal

One desktop app that replaces both **Docker-based dev setups** and **classic local servers** (XAMPP, Laragon, MAMP, Local WP, Herd, ServBay, FlyEnv, DDEV, Lando). Install VSN Devify, and nothing else is needed to build, run, test, share and deploy web projects.

## 2. What competitors do (research summary)

| Product | Strength | Gap Devify targets |
|---|---|---|
| Laragon | Very light native Windows stack | Windows only, no per-project isolation, no team or remote features |
| Local WP | Polished WordPress GUI, auto SSL, host sync | WordPress only |
| Laravel Herd (Pro) | Native PHP/Node, dumps, mail, logs, Xdebug, services, MCP server for AI | PHP/Laravel focus; services are a paid tier |
| ServBay | Many PHP/Node versions, MySQL/MariaDB/Postgres/Redis/Mongo, Mailpit, private CA + ACME, ngrok/frp tunnels | macOS first; no API client, VCS or analytics |
| FlyEnv | Native binaries (no VM), many languages, per-directory version switching, DNS, SSL, Mailpit | No team, remote, AI or profiling layer |
| DDEV / Lando | Reproducible Docker stacks that mirror production | CLI heavy; Docker Desktop is heavy on Windows |
| Docker Desktop | Full container control | Not a dev environment; resource hungry |
| Bruno / Postman | API client (Bruno: offline, git-native collections) | Separate tool, not tied to the running project |
| Mailpit | SMTP capture, HTML compatibility check, spam check, REST API | Separate tool |
| ngrok / Cloudflare Tunnel / frp | Public URLs for local apps | Separate tools; ngrok free tier is limited |

**Positioning:** the native speed of Laragon/FlyEnv, the isolation of DDEV, the polish of Local/Herd, plus the built-in API client, mail, tunnels, team access and AI that none of them have together.

Sources: servbay.com, support.servbay.com, herd.laravel.com, laravel-news.com/herd-pro, en.wikipedia.org/wiki/FlyEnv, github.com/axllent/mailpit, usebruno.com, laradock.io/docs/laradock-alternatives, stackharbor.com (WP local environments compared), pinggy.io and merginit.com (2026 tunnel comparisons), betterstack.com (mise vs asdf).

## 3. Core architecture decision: the Space

Every project lives in a **Space** (your point 4).

- **Isolated space (default):** the project folder plus a `.devify/` directory inside the user's chosen location:
  - `.devify/space.toml` pins the exact runtime, database, cache and tool versions (PHP 8.3, Node 22, pnpm 9, MySQL 8.4 …), the domain, ports, env vars and services.
  - Runtimes and databases are **not copied** into each project. They are installed once into the shared VSN store (`%LOCALAPPDATA%\VSN\store\<tool>\<version>`), content-addressed and verified, the way the existing `runtime.install` and trusted catalog already work. Each space gets its own **data** (its own DB data dir, its own npm/composer cache dir, its own logs), so two spaces never conflict even on different versions.
  - Processes start with a per-space `PATH` and env, so `php`, `node`, `composer` and `npm` resolve to the pinned versions. This is the mise/asdf model without containers, and it is how FlyEnv avoids a VM.
  - Ports are allocated per space; the reverse proxy maps `project.test` to them.
- **Global space:** uses the machine's global installations (current behaviour). Good for quick scripts.
- **Container space (optional):** the same `space.toml` rendered to Docker/Podman Compose for stacks that must mirror production (DDEV/Lando model). Uses the existing `vsn-container` crate.

When a project is created or imported, Devify asks: **Isolated (recommended) / Global / Container**.

A missing version is downloaded from the signed catalog, verified and installed automatically (point 5), using the existing `runtime.install-trusted` / `runtime.catalog-verify` pipeline.

## 4. Feature map — your 22 points

Legend: **Have** = exists in the agent today; **Partial** = foundation exists; **New** = to build.

| # | Feature | Today | Plan | Package |
|---|---|---|---|---|
| 1–3 | Animation, agent connection, visibility fixes | Fixed in the 2026-10-17 build | Keep the 60-scan accessibility gate | done |
| 4 | Isolated vs global environments | **In progress (DVF-01 steps 1–3 done):** `vsn-space` crate; `space.list/get/save/forget/install/process.start/service.start/service.stop`; Spaces screen, New space wizard, install missing versions, run inside a space, per-space services (Redis, Postgres, MySQL, MariaDB, MongoDB, Memcached, Mailpit) on their own ports with data in `.devify/data` | Space model above: `space.toml`, per-space PATH/data/ports, Isolated / Global / Container choice | DVF-01 |
| 5 | Frameworks and CMS: install, update, uninstall | **Create + update done (DVF-02):** 16 generators; one-click update of WordPress core/plugins, Composer, npm/pnpm/yarn/Bun, Bundler, Poetry/uv/Pipenv/pip, Go modules and Cargo; uninstall = remove space (files kept) | Template catalog adds WordPress, Symfony, CodeIgniter, OpenCart, Drupal, Magento, Next.js, Nuxt, Astro, Express, NestJS, Flask, FastAPI, Rails, Spring Boot. Shopify via Shopify CLI theme/app dev. **Wix and Weebly are hosted-only**: Devify integrates their CLIs/APIs where they exist (Wix CLI for apps), it cannot run them locally | DVF-02 |
| 5 | Every runtime and version auto-installed | Partial: signed runtime catalog, install/uninstall/repair; Spaces offer PHP 5.6–8.5, Node 14–24, Python, Go, Ruby, Java, .NET, Rust, Bun and Deno pins (auto-install depends on catalog entries) | Catalog grows to PHP 5.6–8.5, Node 14–24, Python, Go, Ruby, Java, .NET, Rust, Bun, Deno; package managers npm/pnpm/yarn/composer/pip/poetry; version per space | DVF-01 |
| 6 | FTP / SFTP per space | **Done (DVF-07):** Deploy screen, FTP/FTPS via curl, SFTP with pinned host keys, password in vault; local FTP server per space (pyftpdlib, 127.0.0.1) | Built-in FTP/FTPS/SFTP server per space with its own user and folder jail; client panel to sync with remote hosts | DVF-07 |
| 7 | Version control (Git, SVN) | **Git done, SVN basics done (DVF-08):** Source control screen; SVN checkout, status, update, add, revert, commit, history and per-file diff (`svn` CLI) | Git (status, stage, commit, branches, diff, history, stash, remotes, PRs via GitHub/GitLab APIs) and SVN (checkout, update, commit, log, diff, revert) in a Source Control screen | DVF-08 |
| 8 | Remote access and team sharing | Partial: control plane already has IAM roles, tokens, SCIM, approvals and fleet; needs a hosted control plane to use from the app | Team spaces: invite members, role per space (view/edit/admin), share a running project, terminal, files and DB over the authenticated remote channel | DVF-09 |
| 9 | Local email testing | **Done (DVF-05):** Mail screen over each space's Mailpit service; Gmail/Outlook/SMTP relay with per-email release | Built-in SMTP catcher (Mailpit-class): inbox per space, HTML/text/raw/headers, HTML compatibility and spam check, attachment view, template preview with test data, REST API for tests | DVF-05 |
| 10 | Lock a space with a password | **Done:** Argon2 space lock covering processes, services, files, terminal, git and deploy; auto-lock after 30 idle minutes (8 hours at most) | Space lock: encrypts the space's secrets and blocks agent commands for that space until unlocked (password or Windows Hello); auto-lock on idle | DVF-01 |
| 11 | One DB UI for every engine | Have: SQLite, Postgres, MySQL, MongoDB, Redis, Valkey and Memcached browse/query/edit; per-space backup and restore; CSV/JSON export of any result table; CSV row import; table designer that writes CREATE TABLE SQL | Unified Database Studio for all engines, including ones without their own UI; schema designer, import/export, backups per space | DVF-04 |
| 12 | Remote databases | **Done (DVF-04):** one-click space database backup and restore; saved remote connections (Postgres, MySQL, MongoDB, Redis) in the vault, opened by the agent; TLS adapters unchanged | Saved remote connections with SSH tunnel, TLS, read-only mode and vault-stored credentials | DVF-04 |
| 13 | Analytics dashboard | **Done (DVF-10):** per-space CPU/memory/data with trends; requests, 4xx/5xx and slowest URLs per name.test domain from Caddy access logs | Per-space request log through the VSN proxy: slowest routes, error rate, response times, CPU/RAM per service; PHP/Node profiler hooks for slow code paths | DVF-10 |
| 14 | Config shortcuts (php.ini, Apache, Nginx, DB) | **Done:** edit .env, space.toml, php.ini, .htaccess, httpd/apache.conf, nginx.conf, Caddyfile, varnish.vcl, my.cnf/my.ini, postgresql.conf, redis.conf, package/composer.json; starter templates for Nginx, Apache, .htaccess, Varnish and php.ini | Config editor per space with validation and safe restart: php.ini, extensions, Apache/Nginx vhosts, MySQL my.cnf, Postgres conf, Redis | DVF-03 |
| 15 | ngrok / Nginx | **Step 1 done:** one-click route of a space domain (name.test) to its app port via Caddy (one UAC prompt, all routes kept, HTTPS via mkcert or Caddy internal CA); Cloudflare quick tunnel for public links | Web server choice per space (Caddy/Nginx/Apache); share to internet via Cloudflare Tunnel, ngrok or self-hosted frp | DVF-06 |
| 16 | Every cache system | Partial: Redis/Valkey/Memcached services; package caches clearable per space; Redis/Valkey and Memcached browse, edit and one-click flush in Database Studio; Valkey, Memcached and Varnish (`.devify/varnish.vcl` or a 127.0.0.1:8000 backend) as space services; Meilisearch and MinIO too | One-click Redis, Memcached, Valkey, Varnish; cache browser and flush per space | DVF-03 |
| 17 | AI with the user's own accounts | **Done (DVF-11):** AI assistant with the user's Anthropic key (vault), space-aware; MCP server (`vsn-agent mcp`) for Claude Desktop and other AI tools | Connect OpenAI, Anthropic or Gemini keys (stored in the vault); the assistant performs Devify actions through the same policy layer (create space, install PHP, fix config, read logs). Devify also exposes an MCP server so external AI tools can drive it, like Herd 1.22 | DVF-11 |
| 18 | Devify AI subscription | Blocked: needs a hosted service and billing (DVF-12) | Hosted Devify AI plan (no own key needed), billing and usage limits; server side outside this repo | DVF-12 |
| 19 | Open a project from anywhere (mobile, other network) | **Done (DVF-06):** LAN share with QR code and auto-expiry (`share.lan.start/stop`, `share.list`); public HTTPS link via Cloudflare quick tunnel (`share.tunnel.start/status/stop`, needs cloudflared); phone-friendly read-only status page on the LAN behind a secret link (`share.status-page.start`); ngrok as a second tunnel provider; self-hosted frp (`share.frp.start`, token in the vault) | LAN URL + QR code for phones on the same Wi-Fi; public share link through the tunnel (DVF-06) with optional password | DVF-06 |
| 20 | Name: VSN Devify | Done in the app UI | Installer identity rename is a governed change at public release (PKG-03 identity is frozen) | — |
| 21 | Beat Docker and local servers | Measured on Linux (release agent): idle agent 8–10 MB RAM; IPC answers in milliseconds. Space start time depends on the runtime catalog | This plan; performance budget: idle agent < 60 MB RAM, space start < 2 s for native stacks | all |
| 22 | Postman-like API client per project | **Done (DVF-05):** send, save to `.devify/api`, `{{VAR}}` from the space, history, Postman/OpenAPI import, Bearer/Basic auth helpers, response tests | API client inside each space: collections saved as plain files in `.devify/api/` (git-friendly, Bruno model), environments from the space, auth helpers, tests, history, import from Postman/OpenAPI | DVF-05 |
| 23 | Backups anywhere | **Done (DVF-15):** `space.backup.run` archives a space (no node_modules/vendor/.git/build output) with fresh DB dumps and delivers it to a local folder (newest-N retention) or an rclone remote (`backup.remote.create` with browser OAuth or obscured keys; `backup.destination.*`, `backup.history`); email delivery through SMTP (password in the vault), hourly/daily/weekly schedules run by the agent (`backup.schedule.*`), restore into a new folder (`space.backup.restore`); Backups screen | Back up a whole space (files + database dumps) to a local folder or drive, email, Google Drive, OneDrive, Dropbox, Amazon S3 and S3-compatible storage (Wasabi, Backblaze B2, Cloudflare R2, MinIO), MEGA, Box, pCloud, WebDAV/Nextcloud, FTP/SFTP and every other provider rclone supports; OAuth sign-in for Google/Microsoft/Dropbox/Box, keys and passwords encrypted in rclone's config or the vault; schedules, retention and restore | DVF-15 |

## 5. Packages (product track)

| Package | Scope | Depends on |
|---|---|---|
| DVF-01 Spaces | `space.toml`, Isolated/Global/Container, per-space PATH/data/ports, auto-install of missing versions, space lock | runtime catalog |
| DVF-02 Framework catalog | Templates and lifecycle for every framework/CMS in §4 | DVF-01 |
| DVF-03 Services and config | Web servers, DB servers, caches, queues, search (Meilisearch, Typesense, OpenSearch), object storage (MinIO); config editors | DVF-01 |
| DVF-04 Database Studio 2 | Unified UI, remote connections, backups | existing DB crates |
| DVF-05 Mail and API client | SMTP catcher and API client per space | DVF-01 |
| DVF-06 Share and tunnels | LAN/QR, Cloudflare Tunnel, ngrok, frp | network crate |
| DVF-07 FTP/SFTP | Server per space, sync client | DVF-01 |
| DVF-08 Source control | Git and SVN | — |
| DVF-09 Teams and remote | Team spaces, roles, shared access | remote crate |
| DVF-10 Analytics | Proxy request log, profiler, dashboard | DVF-03, DVF-06 |
| DVF-11 AI with own keys + MCP | Assistant and MCP server | ai crate, vault |
| DVF-12 Devify AI subscription | Hosted AI plan and billing | DVF-11 |
| DVF-13 macOS and Linux parity | Same features on all three OSes. Started: personal unsigned macOS dmg and Linux AppImage/deb builds (`personal-unix-build.yml`) with the bundled agent and CLI; domain routing elevates with the macOS admin prompt or pkexec | PKG-05 |
| DVF-14 Public release | Signing, rename installer identity, website | PKG-03 03.22–03.25 |
| DVF-15 Backups anywhere | Space backup archives, destinations (local, email, rclone remotes), schedules, retention, restore | DVF-01, DVF-04 |

## 6. Delivery order

1. **Now:** DVF-01 Spaces, because isolation is the foundation every other feature sits on.
2. DVF-02 + DVF-03: the frameworks and services people install first.
3. DVF-05 + DVF-06: mail, API client and sharing; this is where Devify passes Laragon/Herd.
4. DVF-04, DVF-08, DVF-07.
5. DVF-10, DVF-11, DVF-09.
6. DVF-12, DVF-13, DVF-14.

Each package follows the same loop: design in Figma (light and dark, all breakpoints), agent commands with tests, desktop screens with the accessibility gate, a personal Windows build to try it.

## 7. UI screens needed (Figma)

New screens beyond the 38 already designed: Spaces list and space detail; New space wizard (template → Isolated/Global/Container → versions → domain); Space lock; Framework catalog; Services and versions; Config editor; Mail inbox and message view; API client; Share/tunnel dialog with QR; FTP/SFTP; Source control; Team members and invites; Analytics dashboard; AI assistant panel and provider keys; Devify AI subscription.

## 8. Honest limits

- Wix and Weebly cannot run locally; only their developer CLIs/APIs can be integrated.
- macOS/Linux parity depends on PKG-05.
- Public distribution still needs code signing (03.22) and the installer identity rename.
- Hosted pieces (Devify AI subscription, team relay) need a server outside this repository.
