import { useState, type ChangeEvent } from 'react';
import { call } from '../api/agent';
import { Badge, JsonView } from '../components/ui';

export function Services() {
  const [name, setName] = useState('VSN-');
  const [result, setResult] = useState<any>(null);
  const [err, setErr] = useState('');
  async function run(action: 'status' | 'start' | 'stop' | 'restart') {
    setErr('');
    try {
      setResult(
        action === 'status'
          ? await call('service.status', { name })
          : await call('service.action', { name, action }),
      );
    } catch (e) {
      setErr(String(e));
    }
  }
  return (
    <section className="panel">
      <div className="panelHead">
        <div>
          <span className="kicker">OS service boundary</span>
          <h3>VSN-managed services</h3>
        </div>
        <Badge ok label="VSN-* mutations only" />
      </div>
      <div className="formRow">
        <input
          value={name}
          onChange={(e: ChangeEvent<HTMLInputElement>) => setName(e.target.value)}
          placeholder="VSN-Postgres"
        />
        <button className="primary" onClick={() => void run('status')}>
          Status
        </button>
      </div>
      <div className="buttonRow">
        <button className="ghost" onClick={() => void run('start')}>
          Start
        </button>
        <button className="ghost" onClick={() => void run('restart')}>
          Restart
        </button>
        <button className="danger" onClick={() => void run('stop')}>
          Stop
        </button>
      </div>
      {result && <JsonView value={result} />} {err && <div className="error">{err}</div>}
    </section>
  );
}
