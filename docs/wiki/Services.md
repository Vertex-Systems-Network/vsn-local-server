# Services

Every service listens on **127.0.0.1 only** and keeps its data in `<space>/.devify/data/<service>`, so spaces never share state.

| Service | Program | Default base port | Notes |
| --- | --- | --- | --- |
| MySQL / MariaDB | mysqld / mariadbd | 33060 | first start initialises the data folder |
| Postgres | postgres | 54320 | user `postgres`, trust auth on localhost |
| Redis / Valkey | redis-server / valkey-server | 63790 | protected mode on |
| Memcached | memcached | 11220 | |
| Varnish | varnishd | 60810 | uses `.devify/varnish.vcl` if present, else backend 127.0.0.1:8000 |
| MongoDB | mongod | 27020 | |
| Meilisearch | meilisearch | 17700 | analytics disabled |
| MinIO | minio | 19000 (+1 console) | |
| FTP server | python -m pyftpdlib | 21210 | serves the space folder on 127.0.0.1, login `devify` / `devify`; needs `pip install pyftpdlib` |
| Mailpit | mailpit | 10250 (+1 web UI) | see [[Mail]] |

If a service will not start: run **Diagnose** ([[Troubleshooting]]) — it shows whether the program is installed and whether its port answers.
