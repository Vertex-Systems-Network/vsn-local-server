import { useEffect, useState } from 'react';
import { call } from '../api/agent';
import { Badge } from './ui';
import { useToast } from './Toast';

type Saved = { id: string; name: string; engine: 'postgres' | 'mysql' | 'mongo' | 'redis'; display: string };

const EXAMPLES: Record<Saved['engine'], string> = {
  postgres: 'host=db.example.com port=5432 user=app password=… dbname=app',
  mysql: 'mysql://app:…@db.example.com:3306/app',
  mongo: 'mongodb://app:…@db.example.com:27017',
  redis: 'redis://:…@cache.example.com:6379/0',
};

/** DVF-04: remote database connections saved in the VSN vault; the UI only ever sees a redacted label. */
export function SavedConnections({ onResult }: { onResult: (r: unknown) => void }) {
  const [list, setList] = useState<Saved[]>([]);
  const [name, setName] = useState('');
  const [engine, setEngine] = useState<Saved['engine']>('postgres');
  const [conn, setConn] = useState('');
  const toast = useToast();

  const load = () =>
    call<Saved[]>('db.saved.list')
      .then(setList)
      .catch(() => setList([]));
  useEffect(() => {
    void load();
  }, []);

  async function save() {
    try {
      await call('db.saved.save', { name, engine, connection: conn });
      setName('');
      setConn('');
      toast('Connection saved in the VSN vault');
      await load();
    } catch (e) {
      toast(String(e), 'danger');
    }
  }
  async function open(s: Saved) {
    try {
      const extra = s.engine === 'mongo' ? { database: null } : {};
      onResult(await call(`database.native.${s.engine}.inspect` as never, { saved: s.id, ...extra }));
    } catch (e) {
      toast(String(e), 'danger');
    }
  }
  async function remove(s: Saved) {
    try {
      await call('db.saved.delete', { id: s.id });
      await load();
    } catch (e) {
      toast(String(e), 'danger');
    }
  }

  return (
    <section className="panel" aria-labelledby="saved-dbs">
      <div className="panelHead">
        <div>
          <span className="kicker">Remote databases</span>
          <h3 id="saved-dbs">Saved connections</h3>
        </div>
        <Badge ok={list.length > 0} label={`${list.length} saved`} />
      </div>
      {list.length > 0 && (
        <ul className="historyList">
          {list.map((s) => (
            <li key={s.id}>
              <button className="linkButton" onClick={() => void open(s)}>
                <b>{s.name}</b>
              </button>
              <Badge ok label={s.engine} />
              <small>{s.display}</small>
              <button className="ghost" aria-label={`Delete ${s.name}`} onClick={() => void remove(s)}>
                Delete
              </button>
            </li>
          ))}
        </ul>
      )}
      <div className="formRow">
        <label className="stacked">
          Name
          <input value={name} placeholder="Production DB" onChange={(e) => setName(e.target.value)} />
        </label>
        <label className="stacked">
          Engine
          <select value={engine} onChange={(e) => setEngine(e.target.value as Saved['engine'])}>
            <option value="postgres">PostgreSQL</option>
            <option value="mysql">MySQL / MariaDB</option>
            <option value="mongo">MongoDB</option>
            <option value="redis">Redis / Valkey</option>
          </select>
        </label>
      </div>
      <label className="stacked">
        Connection (with password)
        <input
          type="password"
          autoComplete="off"
          value={conn}
          placeholder={EXAMPLES[engine]}
          onChange={(e) => setConn(e.target.value)}
        />
      </label>
      <p className="hint">
        The full connection is stored in the VSN vault and only used inside the agent; this screen sees just
        the host. For TLS-only servers use the External databases tab.
      </p>
      <button className="primary" disabled={!name.trim() || !conn} onClick={() => void save()}>
        Save connection
      </button>
    </section>
  );
}
