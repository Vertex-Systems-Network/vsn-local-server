import { useState, type ChangeEvent } from 'react';
import { call } from '../api/agent';
import type { RuntimeDetection } from '../contracts';
import { Badge } from '../components/ui';
import { DataTable } from '../components/DataTable';
import { ResultView } from '../components/ResultView';

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
        <DataTable
          caption="Detected runtimes"
          rows={rows}
          rowKey={(r) => r.id}
          empty="No runtimes detected yet. Refresh, or install one from a verified catalog below."
          columns={[
            { key: 'id', label: 'Runtime', render: (r) => <b>{r.id}</b> },
            { key: 'version', label: 'Version', render: (r) => r.version ?? '—' },
            { key: 'exe', label: 'Executable', className: 'hideSm', render: (r) => (r.executable ? <code>{r.executable}</code> : '—') },
            { key: 'status', label: 'Status', render: (r) => <Badge ok={r.installed} label={r.installed ? 'Available' : 'Missing'} /> },
          ]}
        />
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
        <ResultView title="Runtime action" value={result} error={err} />
      </section>
    </>
  );
}
