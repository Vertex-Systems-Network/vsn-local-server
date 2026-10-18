# Database Studio

- **Space databases** — one click opens the database of any space service; **Back up** writes a dump to `<space>/.devify/backups/` (pg_dump, mysqldump/mariadb-dump, mongodump, redis-cli --rdb).
- **Saved connections** — store remote connections; passwords stay in the encrypted vault and never reach the screen.
- **Native clients** — browse, query, insert, update and delete for Postgres, MySQL/MariaDB, MongoDB, Redis and SQLite. Values are bound, never pasted into SQL.
- **Space databases → Restore** — puts the latest backup back into the running database (asks first).
- **Cache servers** — pick **Redis / Valkey** (URL) or **Memcached** (an address on this machine, such as `127.0.0.1:11211`). Inspect shows stats and sample keys; read, set, delete keys and **Flush database** (asks first).
- **Export** — any result that is a table gets **Save CSV**, **Save JSON** and **Copy CSV**.
- **Import CSV rows** — choose a file or paste CSV; the first row names the columns, empty cells become NULL, and each row is inserted into the PostgreSQL, MySQL or SQLite table set above. If a row fails, it tells you how many went in.
- **Table designer** — add columns (type, primary key, allows empty, default) and copy the CREATE TABLE statement for PostgreSQL, MySQL/MariaDB or SQLite.

For complete space backups to the cloud see [[Backups]].
