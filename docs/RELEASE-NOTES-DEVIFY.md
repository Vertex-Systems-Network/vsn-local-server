# VSN Devify — release notes

## Devify product release (PR #247, October 2026)

Personal unsigned builds for Windows (NSIS/MSI), macOS (dmg) and Linux (AppImage/deb). Windows installer code signing (PKG-03 03.22–03.25) is **deferred by the owner**; SmartScreen/Gatekeeper warnings are expected.

### New
- **Spaces**: per-project tools, services, ports and data; isolated/global modes; space lock; URL pattern; change port; domain routing (`name.test`) on Windows, macOS and Linux.
- **Frameworks**: WordPress, Laravel, Drupal, Symfony, CodeIgniter, CakePHP, Yii, Slim, OpenCart, Magento, Next.js, Vite, Astro, Nuxt, Express, NestJS, Rails, Shopify theme; framework updates.
- **Services**: MySQL, MariaDB, Postgres, Redis, Valkey, Memcached, Varnish, MongoDB, Meilisearch, MinIO, Mailpit.
- **Database Studio**: native clients, saved connections in the vault, Redis flush, one-click space database backups.
- **Mail** catcher with Gmail/Outlook/SMTP relay; **API client** with history, imports, auth helpers and tests.
- **Source control**: Git and SVN (checkout, status, update, add, revert, commit, history, diff).
- **Sharing**: LAN + QR, LAN status page, Cloudflare, ngrok and self-hosted frp links.
- **Backups anywhere**: folder/USB, email, Google Drive, OneDrive, Dropbox, Box, pCloud, MEGA, S3-compatible storage, B2, WebDAV, SFTP, FTP; encryption, schedules, restore.
- **AI assistant** with your own key, **Analytics**, **Deploy** (FTP/FTPS/SFTP), **About**, **Help** with one-click **Diagnose**, user-guide wiki (`docs/wiki/`).

### Security
Upload staging, audit-chain continuity, remote process start, unsigned remote runtime installs, updater control folder, locks, state temp files, digest-bound rollback, restore-on-failure, parent-swap re-check and device-identity recovery (#183, #184, #187, #189–#199, #208).

### Dependencies
Dependabot updates applied except where frozen authorities pin bytes (desktop lockfile, `vsn-security`, toolchain and action pins).
