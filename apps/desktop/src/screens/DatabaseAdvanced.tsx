import { useState } from 'react';
import { call } from '../api/agent';
import { Badge, JsonView } from '../components/ui';

export function DatabaseAdvanced() {
  const [mongoUrl, setMongoUrl] = useState('mongodb://localhost:27017');
  const [mongoDb, setMongoDb] = useState('test');
  const [mongoCollection, setMongoCollection] = useState('users');
  const [mongoFilter, setMongoFilter] = useState('{}');
  const [mongoMutation, setMongoMutation] = useState('{"values":{"name":"Ada"},"filter":{}}');
  const [pgTls, setPgTls] = useState('host=db.example.com user=postgres dbname=postgres');
  const [pgCa, setPgCa] = useState('');
  const [mysqlTls, setMysqlTls] = useState('mysql://user@db.example.com/app');
  const [mysqlCa, setMysqlCa] = useState('');
  const [sql, setSql] = useState('SELECT 1');
  const [result, setResult] = useState<any>(null);
  const [err, setErr] = useState('');
  async function mongo(action: 'inspect' | 'browse' | 'indexes' | 'stats' | 'insert' | 'update' | 'delete') {
    setErr('');
    try {
      const base = { url: mongoUrl, database: mongoDb, collection: mongoCollection };
      if (action === 'inspect')
        setResult(await call('database.native.mongo.inspect', { url: mongoUrl, database: mongoDb || null }));
      else if (action === 'browse')
        setResult(
          await call('database.native.mongo.browse', {
            ...base,
            limit: 100,
            offset: 0,
            filter: JSON.parse(mongoFilter || '{}'),
          }),
        );
      else if (action === 'indexes' || action === 'stats')
        setResult(await call(`database.native.mongo.${action}`, base));
      else
        setResult(
          await call(`database.native.mongo.${action}`, { ...base, request: JSON.parse(mongoMutation) }),
        );
    } catch (e) {
      setErr(String(e));
    }
  }
  async function pg(action: 'inspect' | 'browse' | 'query') {
    setErr('');
    try {
      const connection = { connection_string: pgTls, root_ca_pem_path: pgCa };
      if (action === 'inspect') setResult(await call('database.tls.postgres.inspect', { connection }));
      else if (action === 'browse')
        setResult(
          await call('database.tls.postgres.browse', {
            connection,
            schema: 'public',
            table: mongoCollection,
            limit: 100,
            offset: 0,
          }),
        );
      else setResult(await call('database.tls.postgres.query', { connection, sql }));
    } catch (e) {
      setErr(String(e));
    }
  }
  async function mysql(action: 'inspect' | 'browse' | 'query') {
    setErr('');
    try {
      const connection = { url: mysqlTls, root_ca_path: mysqlCa };
      if (action === 'inspect') setResult(await call('database.tls.mysql.inspect', { connection }));
      else if (action === 'browse')
        setResult(
          await call('database.tls.mysql.browse', {
            connection,
            database: mongoDb,
            table: mongoCollection,
            limit: 100,
            offset: 0,
          }),
        );
      else setResult(await call('database.tls.mysql.query', { connection, sql }));
    } catch (e) {
      setErr(String(e));
    }
  }
  return (
    <>
      <section className="panel">
        <div className="panelHead">
          <div>
            <span className="kicker">Native document database</span>
            <h3>MongoDB Studio</h3>
          </div>
          <Badge ok label="Local or SRV/TLS policy" />
        </div>
        <div className="formGrid">
          <label>
            MongoDB URL
            <input value={mongoUrl} onChange={(e) => setMongoUrl(e.target.value)} />
          </label>
          <label>
            Database
            <input value={mongoDb} onChange={(e) => setMongoDb(e.target.value)} />
          </label>
          <label>
            Collection
            <input value={mongoCollection} onChange={(e) => setMongoCollection(e.target.value)} />
          </label>
        </div>
        <label className="stacked">
          Browse filter JSON
          <textarea value={mongoFilter} onChange={(e) => setMongoFilter(e.target.value)} rows={3} />
        </label>
        <div className="buttonRow">
          <button className="primary" onClick={() => void mongo('inspect')}>
            Inspect Mongo
          </button>
          <button className="ghost" onClick={() => void mongo('browse')}>
            Browse
          </button>
          <button className="ghost" onClick={() => void mongo('indexes')}>
            Indexes
          </button>
          <button className="ghost" onClick={() => void mongo('stats')}>
            Stats
          </button>
        </div>
        <label className="stacked">
          Structured mutation JSON
          <textarea value={mongoMutation} onChange={(e) => setMongoMutation(e.target.value)} rows={5} />
        </label>
        <div className="buttonRow">
          <button className="primary" onClick={() => void mongo('insert')}>
            Insert
          </button>
          <button className="ghost" onClick={() => void mongo('update')}>
            Update
          </button>
          <button className="danger" onClick={() => void mongo('delete')}>
            Delete
          </button>
        </div>
        <p className="hint">
          Non-loopback plaintext mongodb:// is rejected. Remote Mongo mutations are not part of the signed
          remote-command allowlist.
        </p>
      </section>
      <section className="panel">
        <div className="panelHead">
          <div>
            <span className="kicker">Verified database TLS</span>
            <h3>PostgreSQL / MySQL CA profiles</h3>
          </div>
          <Badge ok label="CA validation retained" />
        </div>
        <div className="formGrid">
          <label>
            PostgreSQL connection
            <input value={pgTls} onChange={(e) => setPgTls(e.target.value)} />
          </label>
          <label>
            PostgreSQL root CA
            <input value={pgCa} onChange={(e) => setPgCa(e.target.value)} placeholder="Absolute PEM path" />
          </label>
          <label>
            MySQL URL
            <input value={mysqlTls} onChange={(e) => setMysqlTls(e.target.value)} />
          </label>
          <label>
            MySQL root CA
            <input
              value={mysqlCa}
              onChange={(e) => setMysqlCa(e.target.value)}
              placeholder="Absolute PEM/DER path"
            />
          </label>
        </div>
        <label className="stacked">
          Read-only SQL
          <textarea value={sql} onChange={(e) => setSql(e.target.value)} rows={4} />
        </label>
        <div className="buttonRow">
          <button className="primary" onClick={() => void pg('inspect')}>
            PG TLS Inspect
          </button>
          <button className="ghost" onClick={() => void pg('query')}>
            PG TLS Query
          </button>
          <button className="primary" onClick={() => void mysql('inspect')}>
            MySQL TLS Inspect
          </button>
          <button className="ghost" onClick={() => void mysql('query')}>
            MySQL TLS Query
          </button>
        </div>
        <p className="hint">
          CA files must be absolute local files. Certificate validation and hostname verification are not
          disabled by these profiles.
        </p>
        {result && <JsonView value={result} />} {err && <div className="error">{err}</div>}
      </section>
    </>
  );
}
