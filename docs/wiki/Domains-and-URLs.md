# Domains and URLs

## URL pattern
**Settings → Project URL pattern** controls how project addresses are shown, Laragon-style, using `{name}`, `{port}` and `{domain}` — the default is `http://{name}.localhost:{port}`.

## name.test domains
**Spaces → Route to port** gives a space a domain such as `shop.test` and sends it to the app's port:
- VSN adds the hosts-file entry and a Caddy route (one shared Caddyfile for all routes) and reloads Caddy.
- This needs administrator rights once: Windows shows a UAC prompt, macOS its admin password dialog, Linux a polkit (pkexec) prompt.
- Only `name.test` domains are accepted.

Requires Caddy (`winget install CaddyServer.Caddy`).
