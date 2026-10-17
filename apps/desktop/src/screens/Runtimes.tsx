import { useState, type ChangeEvent } from 'react';
import { call } from '../api/agent';
import type { RuntimeDetection } from '../contracts';
import { Badge, JsonView } from '../components/ui';

export function Runtimes({ rows }: { rows: RuntimeDetection[] }) {
  const [catalog, setCatalog] = useState('');
  const [runtime, setRuntime] = useState('php');
  const [version, setVersion] = useState('');
  const [result, setResult] = useState<any>(null);
  const [err, setErr] = useState('');
  async function run(action: 'install' | 'uninstall') {
    setErr('');
    try {
      setResult(
        action === 'install'
          ? await call('runtime.install', { catalog, runtime, version })
          : await call('runtime.uninstall', { runtime, version }),
      );
    } catch (e) {
      setErr(String(e));
    }
  }
  async function audit() {
    setErr('');
    try {
      setResult(await call('runtime.audit', {}));
    } catch (e) {
      setErr(String(e));
    }
  }
  return (
    <>
      <section className="panel">
        <div className="panelHead">
          <div>
            <span className="kicker">Provider-driven</span>
            <h3>Runtime inventory</h3>
          </div>
          <span>
            {rows.filter((r) => r.installed).length}/{rows.length} detected
          </span>
        </div>
        <div className="cards">
          {rows.map((r) => (
            <article className="runtime" key={r.id}>
              <div className="runtimeIcon">{r.id.slice(0, 2).toUpperCase()}</div>
              <div>
                <h4>{r.id}</h4>
                <p>{r.version ?? 'Not installed on PATH'}</p>
              </div>
              <Badge ok={r.installed} label={r.installed ? 'Available' : 'Missing'} />
            </article>
          ))}
        </div>
      </section>
      <section className="panel">
        <div className="panelHead">
          <div>
            <span className="kicker">VSN managed runtime</span>
            <h3>Install / uninstall</h3>
          </div>
        </div>
        <div className="formGrid">
          <label>
            Catalog path
            <input
              value={catalog}
              onChange={(e: ChangeEvent<HTMLInputElement>) => setCatalog(e.target.value)}
              placeholder="C:\catalogs\runtime.json"
            />
          </label>
          <label>
            Runtime
            <input
              value={runtime}
              onChange={(e: ChangeEvent<HTMLInputElement>) => setRuntime(e.target.value)}
            />
          </label>
          <label>
            Version
            <input
              value={version}
              onChange={(e: ChangeEvent<HTMLInputElement>) => setVersion(e.target.value)}
              placeholder="8.4.0"
            />
          </label>
        </div>
        <div className="buttonRow">
          <button className="primary" onClick={() => void run('install')}>
            Install verified artifact
          </button>
          <button className="ghost" onClick={() => void run('uninstall')}>
            Uninstall
          </button>
          <button className="ghost" onClick={() => void audit()}>
            Audit registry
          </button>
        </div>
        {err && <div className="error">{err}</div>}
        {result && <JsonView value={result} />}
      </section>
    </>
  );
}
