import { useEffect, useState } from 'react';
import { call } from '../api/agent';
import type { AgentCommand, DbClient, SpaceSummary } from '../contracts';
import { csvToInserts } from '../screenLogic';
import { Badge } from '../components/ui';
import { ResultView } from '../components/ResultView';
import { SavedConnections } from '../components/SavedConnections';

type SpaceDb = { space: string; kind: string; port: number; root?: string };
const DB_KINDS = ['postgres', 'mysql', 'mariadb', 'redis', 'valkey', 'mongodb'];
/** Every database service across spaces, with the port VSN allocated for it. */
export function spaceDatabases(list: SpaceSummary[]): SpaceDb[] {
  return list.flatMap((s) =>
    (s.spec?.services ?? [])
      .filter((sv) => DB_KINDS.includes(sv.kind) && sv.port)
      .map((sv) => ({ space: s.spec!.name, kind: sv.kind, port: sv.port!, root: s.root })),
  );
}

export function DatabaseStudio() {
  const [clients, setClients] = useState<DbClient[]>([]);
  const [engine, setEngine] = useState('postgresql');
  const [host, setHost] = useState('127.0.0.1');
  const [port, setPort] = useState('5432');
  const [user, setUser] = useState('');
  const [database, setDatabase] = useState('');
  const [credential, setCredential] = useState('');
  const [sql, setSql] = useState('SELECT 1');
  const [result, setResult] = useState<any>(null);
  const [sqlite, setSqlite] = useState('');
  const [sqliteEntity, setSqliteEntity] = useState('users');
  const [sqliteMutation, setSqliteMutation] = useState('{"values":{"name":"Ada"},"filter":{}}');
  const [err, setErr] = useState('');
  const [nativePg, setNativePg] = useState('host=localhost user=postgres dbname=postgres');
  const [nativeSchema, setNativeSchema] = useState('public');
  const [nativeTable, setNativeTable] = useState('');
  const [mysqlUrl, setMysqlUrl] = useState('mysql://root@localhost/mysql');
  const [mysqlDatabase, setMysqlDatabase] = useState('mysql');
  const [mysqlTable, setMysqlTable] = useState('user');
  const [redisUrl, setRedisUrl] = useState('redis://localhost:6379/0');
  const [redisKey, setRedisKey] = useState('');
  const [redisValue, setRedisValue] = useState('');
  const [redisTtl, setRedisTtl] = useState('');
  const [nativeMutation, setNativeMutation] = useState('{"values":{"name":"Ada"},"filter":{}}');
  const [spaceDbs, setSpaceDbs] = useState<SpaceDb[]>([]);
  const [importCsv, setImportCsv] = useState('');
  const [importTarget, setImportTarget] = useState<'postgres' | 'mysql' | 'sqlite'>('postgres');
  const [importing, setImporting] = useState('');
  useEffect(() => {
    call<SpaceSummary[]>('space.list')
      .then((list) => setSpaceDbs(spaceDatabases(list)))
      .catch(() => setSpaceDbs([]));
  }, []);
  /** One click: fill the right native connection for a space's database and inspect it. */
  async function openSpaceDb(db: SpaceDb) {
    setErr('');
    try {
      if (db.kind === 'postgres') {
        const c = `host=127.0.0.1 port=${db.port} user=postgres dbname=postgres`;
        setNativePg(c);
        setResult(await call('database.native.postgres.inspect', { connection_string: c }));
      } else if (db.kind === 'mysql' || db.kind === 'mariadb') {
        const u = `mysql://root@127.0.0.1:${db.port}/mysql`;
        setMysqlUrl(u);
        setResult(await call('database.native.mysql.inspect', { url: u }));
      } else if (db.kind === 'redis' || db.kind === 'valkey') {
        const u = `redis://127.0.0.1:${db.port}/0`;
        setRedisUrl(u);
        setResult(await call('database.native.redis.inspect', { url: u }));
      } else if (db.kind === 'mongodb') {
        setResult(
          await call('database.native.mongo.inspect', {
            url: `mongodb://127.0.0.1:${db.port}`,
            database: null,
          }),
        );
      }
    } catch (e) {
      setErr(`${String(e)}. Start the ${db.kind} service of ${db.space} in Spaces first.`);
    }
  }
  useEffect(() => {
    call<DbClient[]>('database.cli.detect')
      .then(setClients)
      .catch((e) => setErr(String(e)));
  }, []);
  function connection() {
    return {
      engine,
      host: host || null,
      port: port ? Number(port) : null,
      user: user || null,
      database: database || null,
      credential_file: credential || null,
      service: null,
    };
  }
  async function inspect() {
    setErr('');
    try {
      setResult(await call('database.cli.inspect', { connection: connection() }));
    } catch (e) {
      setErr(String(e));
    }
  }
  async function query() {
    setErr('');
    try {
      setResult(await call('database.cli.query', { connection: connection(), sql }));
    } catch (e) {
      setErr(String(e));
    }
  }
  async function sqliteAction(
    action: 'inspect' | 'browse' | 'indexes' | 'relations' | 'stats' | 'insert' | 'update' | 'delete',
  ) {
    setErr('');
    try {
      if (action === 'inspect') {
        setResult(await call('database.sqlite.inspect', { path: sqlite }));
        return;
      }
      if (action === 'browse') {
        setResult(
          await call('database.sqlite.browse', { path: sqlite, entity: sqliteEntity, limit: 100, offset: 0 }),
        );
        return;
      }
      if (action === 'indexes' || action === 'relations' || action === 'stats') {
        setResult(await call(`database.sqlite.${action}`, { path: sqlite, entity: sqliteEntity }));
        return;
      }
      const request = JSON.parse(sqliteMutation);
      setResult(await call(`database.sqlite.${action}`, { path: sqlite, entity: sqliteEntity, request }));
    } catch (e) {
      setErr(String(e));
    }
  }
  async function nativePgAction(
    action:
      | 'inspect'
      | 'browse'
      | 'query'
      | 'indexes'
      | 'relations'
      | 'stats'
      | 'insert'
      | 'update'
      | 'delete',
  ) {
    setErr('');
    try {
      if (action === 'inspect')
        setResult(await call('database.native.postgres.inspect', { connection_string: nativePg }));
      else if (action === 'browse')
        setResult(
          await call('database.native.postgres.browse', {
            connection_string: nativePg,
            schema: nativeSchema,
            table: nativeTable,
            limit: 100,
            offset: 0,
          }),
        );
      else if (action === 'query')
        setResult(await call('database.native.postgres.query', { connection_string: nativePg, sql }));
      else if (action === 'indexes' || action === 'relations' || action === 'stats')
        setResult(
          await call(`database.native.postgres.${action}`, {
            connection_string: nativePg,
            schema: nativeSchema,
            table: nativeTable,
          }),
        );
      else
        setResult(
          await call(`database.native.postgres.${action}`, {
            connection_string: nativePg,
            schema: nativeSchema,
            table: nativeTable,
            request: JSON.parse(nativeMutation),
          }),
        );
    } catch (e) {
      setErr(String(e));
    }
  }
  async function nativeMysqlAction(
    action:
      | 'inspect'
      | 'browse'
      | 'query'
      | 'indexes'
      | 'relations'
      | 'stats'
      | 'insert'
      | 'update'
      | 'delete',
  ) {
    setErr('');
    try {
      if (action === 'inspect') setResult(await call('database.native.mysql.inspect', { url: mysqlUrl }));
      else if (action === 'browse')
        setResult(
          await call('database.native.mysql.browse', {
            url: mysqlUrl,
            database: mysqlDatabase,
            table: mysqlTable,
            limit: 100,
            offset: 0,
          }),
        );
      else if (action === 'query')
        setResult(await call('database.native.mysql.query', { url: mysqlUrl, sql }));
      else if (action === 'indexes' || action === 'relations' || action === 'stats')
        setResult(
          await call(`database.native.mysql.${action}`, {
            url: mysqlUrl,
            database: mysqlDatabase,
            table: mysqlTable,
          }),
        );
      else
        setResult(
          await call(`database.native.mysql.${action}`, {
            url: mysqlUrl,
            database: mysqlDatabase,
            table: mysqlTable,
            request: JSON.parse(nativeMutation),
          }),
        );
    } catch (e) {
      setErr(String(e));
    }
  }
  /** Inserts every CSV row through the same bound-parameter insert the mutation buttons use. */
  async function importRows() {
    setErr('');
    const rows = csvToInserts(importCsv);
    if (!rows.length) return setErr('The CSV needs a header row and at least one data row.');
    const target: { command: AgentCommand; params: Record<string, unknown> } =
      importTarget === 'postgres'
        ? {
            command: 'database.native.postgres.insert',
            params: { connection_string: nativePg, schema: nativeSchema, table: nativeTable },
          }
        : importTarget === 'mysql'
          ? {
              command: 'database.native.mysql.insert',
              params: { url: mysqlUrl, database: mysqlDatabase, table: mysqlTable },
            }
          : { command: 'database.sqlite.insert', params: { path: sqlite, entity: sqliteEntity } };
    let done = 0;
    try {
      for (const values of rows) {
        setImporting(`Importing ${done + 1} of ${rows.length}…`);
        await call(target.command, { ...target.params, request: { values, filter: {} } });
        done++;
      }
      setResult({ imported: done, table: importTarget });
    } catch (e) {
      setErr(`Imported ${done} of ${rows.length} row(s), then stopped: ${String(e)}`);
    } finally {
      setImporting('');
    }
  }
  function loadCsvFile(file: File | undefined) {
    if (file) void file.text().then(setImportCsv);
  }
  async function nativeRedisAction(action: 'inspect' | 'get' | 'set' | 'delete' | 'flush') {
    setErr('');
    try {
      if (action === 'flush') {
        if (!window.confirm('Delete every key in this Redis database?')) return;
        setResult(await call('database.native.redis.flush', { url: redisUrl }));
      } else if (action === 'inspect')
        setResult(await call('database.native.redis.inspect', { url: redisUrl }));
      else if (action === 'get')
        setResult(await call('database.native.redis.get', { url: redisUrl, key: redisKey }));
      else if (action === 'set')
        setResult(
          await call('database.native.redis.set', {
            url: redisUrl,
            key: redisKey,
            value: redisValue,
            ttl_seconds: redisTtl ? Number(redisTtl) : null,
          }),
        );
      else setResult(await call('database.native.redis.delete', { url: redisUrl, key: redisKey }));
    } catch (e) {
      setErr(String(e));
    }
  }
  return (
    <>
      <SavedConnections onResult={setResult} />
      <section className="panel" aria-labelledby="space-dbs">
        <div className="panelHead">
          <div>
            <span className="kicker">One click</span>
            <h3 id="space-dbs">Space databases</h3>
          </div>
        </div>
        {spaceDbs.length === 0 ? (
          <p className="hint">
            No space has a database yet. Add mysql, mariadb, postgres, redis or mongodb to a space in Spaces.
          </p>
        ) : (
          <div className="buttonRow">
            {spaceDbs.map((db) => (
              <span key={`${db.space}-${db.kind}`} className="buttonRow">
                <button className="ghost" onClick={() => void openSpaceDb(db)}>
                  {db.space} · {db.kind} :{db.port}
                </button>
                <button
                  className="ghost"
                  aria-label={`Back up ${db.kind} of ${db.space}`}
                  onClick={() =>
                    void call<{ status_code: number | null; output_tail: string }>('space.db.backup', {
                      path: db.root,
                      kind: db.kind,
                    })
                      .then((r) => {
                        if (r.status_code === 0) setResult({ backup: 'saved in .devify/backups', ...r });
                        else setErr(r.output_tail || 'Backup failed');
                      })
                      .catch((e) => setErr(String(e)))
                  }
                >
                  Back up
                </button>
                {db.kind !== 'redis' && (
                  <button
                    className="ghost"
                    aria-label={`Restore ${db.kind} of ${db.space} from its latest backup`}
                    onClick={() =>
                      void call<string[]>('space.db.backups', { path: db.root })
                        .then((files) => {
                          const latest = files.find((f) => f.startsWith(`${db.kind}-`));
                          if (!latest) return setErr('No backup of this database yet. Press Back up first.');
                          if (!window.confirm(`Replace the ${db.kind} data of ${db.space} with ${latest}?`))
                            return;
                          return call<{ status_code: number | null; output_tail: string }>(
                            'space.db.restore',
                            {
                              path: db.root,
                              kind: db.kind,
                              file: latest,
                            },
                          ).then((r) =>
                            r.status_code === 0
                              ? setResult({ restored: latest })
                              : setErr(r.output_tail || 'Restore failed'),
                          );
                        })
                        .catch((e) => setErr(String(e)))
                    }
                  >
                    Restore
                  </button>
                )}
              </span>
            ))}
          </div>
        )}
      </section>
      <section className="panel">
        <div className="panelHead">
          <div>
            <span className="kicker">Native client adapters</span>
            <h3>Database clients</h3>
          </div>
          <span>
            {clients.filter((c) => c.available).length}/{clients.length} available
          </span>
        </div>
        <div className="cards">
          {clients.map((c) => (
            <article className="runtime" key={c.engine}>
              <div className="runtimeIcon">DB</div>
              <div>
                <h4>{c.engine}</h4>
                <p>{c.version ?? c.executable ?? 'Not detected'}</p>
              </div>
              <Badge ok={c.available} label={c.available ? 'Available' : 'Missing'} />
            </article>
          ))}
        </div>
      </section>
      <section className="panel">
        <div className="panelHead">
          <div>
            <span className="kicker">Universal DB Studio</span>
            <h3>Read-only external DB inspection</h3>
          </div>
          <Badge ok label="No password argv" />
        </div>
        <div className="formGrid cols3">
          <label>
            Engine
            <select value={engine} onChange={(e) => setEngine(e.target.value)}>
              <option value="postgresql">PostgreSQL</option>
              <option value="mysql">MySQL</option>
              <option value="mariadb">MariaDB</option>
              <option value="mongo">MongoDB</option>
              <option value="redis">Redis</option>
            </select>
          </label>
          <label>
            Host
            <input value={host} onChange={(e) => setHost(e.target.value)} />
          </label>
          <label>
            Port
            <input value={port} onChange={(e) => setPort(e.target.value)} />
          </label>
          <label>
            User
            <input value={user} onChange={(e) => setUser(e.target.value)} />
          </label>
          <label>
            Database
            <input value={database} onChange={(e) => setDatabase(e.target.value)} />
          </label>
          <label>
            Credential config file
            <input
              value={credential}
              onChange={(e) => setCredential(e.target.value)}
              placeholder="Optional .pgpass / defaults-extra-file"
            />
          </label>
        </div>
        <div className="buttonRow">
          <button className="primary" onClick={() => void inspect()}>
            Inspect
          </button>
        </div>
        <label className="stacked">
          Read-only SQL
          <textarea value={sql} onChange={(e) => setSql(e.target.value)} rows={5} />
        </label>
        <button
          className="ghost"
          disabled={engine === 'mongo' || engine === 'redis'}
          onClick={() => void query()}
        >
          Run read-only query
        </button>
        <p className="hint">
          PostgreSQL/MySQL/MariaDB query mode accepts read-only single-statement SELECT/SHOW/DESCRIBE only.
          MongoDB and Redis baseline exposes introspection only. Credentials are never accepted as password
          command-line arguments.
        </p>
      </section>
      <section className="panel">
        <div className="panelHead">
          <div>
            <span className="kicker">Native Rust drivers</span>
            <h3>PostgreSQL + MySQL + Redis</h3>
          </div>
          <Badge ok label="Direct protocol clients" />
        </div>
        <div className="formGrid">
          <label>
            PostgreSQL connection
            <input
              value={nativePg}
              onChange={(e) => setNativePg(e.target.value)}
              placeholder="host=localhost user=postgres dbname=postgres"
            />
          </label>
          <label>
            Schema
            <input value={nativeSchema} onChange={(e) => setNativeSchema(e.target.value)} />
          </label>
          <label>
            Table
            <input value={nativeTable} onChange={(e) => setNativeTable(e.target.value)} />
          </label>
        </div>
        <div className="buttonRow">
          <button className="primary" onClick={() => void nativePgAction('inspect')}>
            Inspect PostgreSQL
          </button>
          <button className="ghost" disabled={!nativeTable} onClick={() => void nativePgAction('browse')}>
            Browse table
          </button>
          <button className="ghost" onClick={() => void nativePgAction('query')}>
            Run read-only SQL
          </button>
          <button className="ghost" disabled={!nativeTable} onClick={() => void nativePgAction('indexes')}>
            Indexes
          </button>
          <button className="ghost" disabled={!nativeTable} onClick={() => void nativePgAction('relations')}>
            Relations
          </button>
          <button className="ghost" disabled={!nativeTable} onClick={() => void nativePgAction('stats')}>
            Stats
          </button>
        </div>
        <div className="formGrid">
          <label>
            MySQL URL
            <input value={mysqlUrl} onChange={(e) => setMysqlUrl(e.target.value)} />
          </label>
          <label>
            MySQL database
            <input value={mysqlDatabase} onChange={(e) => setMysqlDatabase(e.target.value)} />
          </label>
          <label>
            MySQL table
            <input value={mysqlTable} onChange={(e) => setMysqlTable(e.target.value)} />
          </label>
        </div>
        <div className="buttonRow">
          <button className="primary" onClick={() => void nativeMysqlAction('inspect')}>
            Inspect MySQL
          </button>
          <button className="ghost" disabled={!mysqlTable} onClick={() => void nativeMysqlAction('browse')}>
            Browse MySQL
          </button>
          <button className="ghost" onClick={() => void nativeMysqlAction('query')}>
            Run MySQL read-only SQL
          </button>
          <button className="ghost" disabled={!mysqlTable} onClick={() => void nativeMysqlAction('indexes')}>
            Indexes
          </button>
          <button
            className="ghost"
            disabled={!mysqlTable}
            onClick={() => void nativeMysqlAction('relations')}
          >
            Relations
          </button>
          <button className="ghost" disabled={!mysqlTable} onClick={() => void nativeMysqlAction('stats')}>
            Stats
          </button>
        </div>
        <label className="stacked">
          Structured mutation JSON
          <textarea value={nativeMutation} onChange={(e) => setNativeMutation(e.target.value)} rows={5} />
        </label>
        <div className="buttonRow">
          <button className="primary" disabled={!nativeTable} onClick={() => void nativePgAction('insert')}>
            PG Insert
          </button>
          <button className="ghost" disabled={!nativeTable} onClick={() => void nativePgAction('update')}>
            PG Update
          </button>
          <button className="danger" disabled={!nativeTable} onClick={() => void nativePgAction('delete')}>
            PG Delete
          </button>
          <button className="primary" disabled={!mysqlTable} onClick={() => void nativeMysqlAction('insert')}>
            MySQL Insert
          </button>
          <button className="ghost" disabled={!mysqlTable} onClick={() => void nativeMysqlAction('update')}>
            MySQL Update
          </button>
          <button className="danger" disabled={!mysqlTable} onClick={() => void nativeMysqlAction('delete')}>
            MySQL Delete
          </button>
        </div>
        <div className="formGrid">
          <label>
            Redis URL
            <input value={redisUrl} onChange={(e) => setRedisUrl(e.target.value)} />
          </label>
          <label>
            Redis key
            <input value={redisKey} onChange={(e) => setRedisKey(e.target.value)} />
          </label>
          <label>
            Redis value
            <input value={redisValue} onChange={(e) => setRedisValue(e.target.value)} />
          </label>
          <label>
            TTL seconds
            <input value={redisTtl} onChange={(e) => setRedisTtl(e.target.value)} />
          </label>
        </div>
        <div className="buttonRow">
          <button className="primary" onClick={() => void nativeRedisAction('inspect')}>
            Inspect Redis
          </button>
          <button className="ghost" disabled={!redisKey} onClick={() => void nativeRedisAction('get')}>
            Read key
          </button>
          <button className="ghost" disabled={!redisKey} onClick={() => void nativeRedisAction('set')}>
            Set string
          </button>
          <button className="danger" disabled={!redisKey} onClick={() => void nativeRedisAction('delete')}>
            Delete key
          </button>
          <button className="danger" onClick={() => void nativeRedisAction('flush')}>
            Flush database
          </button>
        </div>
        <p className="hint">
          Structured PostgreSQL/MySQL mutations bind values instead of concatenating them; update/delete
          require non-empty equality filters. Native database network policy remains conservative, and remote
          DB mutation is not exposed through the signed remote command allowlist.
        </p>
      </section>
      <section className="panel">
        <div className="panelHead">
          <div>
            <span className="kicker">Embedded DB Studio</span>
            <h3>SQLite browse + structured CRUD</h3>
          </div>
          <Badge ok label="Parameterized mutations" />
        </div>
        <div className="formGrid">
          <label>
            SQLite file
            <input
              value={sqlite}
              onChange={(e) => setSqlite(e.target.value)}
              placeholder="Absolute .sqlite/.db file"
            />
          </label>
          <label>
            Table / view
            <input
              value={sqliteEntity}
              onChange={(e) => setSqliteEntity(e.target.value)}
              placeholder="users"
            />
          </label>
        </div>
        <div className="buttonRow">
          <button className="primary" onClick={() => void sqliteAction('inspect')}>
            Inspect DB
          </button>
          <button className="ghost" onClick={() => void sqliteAction('browse')}>
            Browse 100
          </button>
          <button className="ghost" onClick={() => void sqliteAction('indexes')}>
            Indexes
          </button>
          <button className="ghost" onClick={() => void sqliteAction('relations')}>
            Relations
          </button>
          <button className="ghost" onClick={() => void sqliteAction('stats')}>
            Stats
          </button>
        </div>
        <label className="stacked">
          Mutation JSON
          <textarea value={sqliteMutation} onChange={(e) => setSqliteMutation(e.target.value)} rows={6} />
        </label>
        <div className="buttonRow">
          <button className="primary" onClick={() => void sqliteAction('insert')}>
            Insert
          </button>
          <button className="ghost" onClick={() => void sqliteAction('update')}>
            Update
          </button>
          <button className="danger" onClick={() => void sqliteAction('delete')}>
            Delete
          </button>
        </div>
        <p className="hint">
          Update/delete require a non-empty equality filter. Mutation values are bound as parameters; Desktop
          does not construct raw SQL values.
        </p>
      </section>
      <section className="panel" aria-labelledby="db-import">
        <div className="panelHead">
          <div>
            <span className="kicker">Import</span>
            <h3 id="db-import">Import CSV rows</h3>
          </div>
        </div>
        <p className="hint">
          The first row names the columns. Each row is inserted with bound parameters into the table set above
          for the chosen engine. Any result table can be saved as CSV or JSON from its result panel.
        </p>
        <div className="formGrid">
          <label>
            Target
            <select
              value={importTarget}
              onChange={(e) => setImportTarget(e.target.value as 'postgres' | 'mysql' | 'sqlite')}
            >
              <option value="postgres">PostgreSQL ({nativeTable || 'no table set'})</option>
              <option value="mysql">MySQL ({mysqlTable || 'no table set'})</option>
              <option value="sqlite">SQLite ({sqliteEntity || 'no table set'})</option>
            </select>
          </label>
          <label>
            CSV file
            <input type="file" accept=".csv,text/csv" onChange={(e) => loadCsvFile(e.target.files?.[0])} />
          </label>
        </div>
        <label className="stacked">
          CSV
          <textarea value={importCsv} onChange={(e) => setImportCsv(e.target.value)} rows={5} />
        </label>
        <div className="buttonRow">
          <button className="primary" disabled={!importCsv.trim() || !!importing} onClick={() => void importRows()}>
            {importing || 'Import rows'}
          </button>
        </div>
        <ResultView value={result} error={err} />
      </section>
    </>
  );
}
