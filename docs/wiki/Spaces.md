# Spaces

A **space** is a project folder plus everything it needs: tool versions, services, environment variables and ports. Settings are saved in `space.toml` so they can be committed with the project.

## Modes
- **Isolated** — tools come from VSN's own runtime store; each space can pin different PHP/Node versions.
- **Global** — uses tools already installed on the computer.

## Tools you can pin
PHP 5.6–8.5, Node 14–24, Python, Go, Ruby, Java, .NET, Rust, Bun and Deno. Missing pinned versions are downloaded from the signed runtime catalog when available.

## Services
Add MySQL, MariaDB, Postgres, Redis, Valkey, Memcached, Varnish, MongoDB, Meilisearch, MinIO or Mailpit. See [[Services]].

## Useful actions
- **Change port** — move a service to another port (1024 or higher, not used by another space).
- **Config** — edit `.env` and framework config files; **Clear caches** removes framework caches.
- **Update framework** — runs composer/npm updates for the project.
- **Lock** — protect a space with a password (12+ characters). While locked, its files, terminals, git, transfers and backups are blocked until you unlock it for this session.
- **Route to port** — give the space a `name.test` domain (see [[Domains and URLs]]).
