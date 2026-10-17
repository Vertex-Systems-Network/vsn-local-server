import { useState } from 'react';
import { call } from '../api/agent';
import { Badge, JsonView } from '../components/ui';

export function Networking() {
  const [domain, setDomain] = useState('workforce.test');
  const [port, setPort] = useState('8000');
  const [plan, setPlan] = useState<any>(null);
  async function run() {
    setPlan(await call('domain.plan', { domain, port: Number(port), tls: true }));
  }
  return (
    <section className="panel">
      <div className="panelHead">
        <div>
          <span className="kicker">Local routing</span>
          <h3>.test domain planner</h3>
        </div>
        <Badge ok label="TLS-ready" />
      </div>
      <div className="formGrid">
        <label>
          Domain
          <input value={domain} onChange={(e) => setDomain(e.target.value)} />
        </label>
        <label>
          Upstream port
          <input value={port} onChange={(e) => setPort(e.target.value)} />
        </label>
      </div>
      <button className="primary" onClick={() => void run()}>
        Validate plan
      </button>
      {plan && <JsonView value={plan} />}
      <p className="hint">
        Hosts-file mutation, local CA installation and Caddy binding require the elevated network helper
        instead of normal IPC permissions.
      </p>
    </section>
  );
}
