import { useEffect, useState } from 'react';
import { call } from '../api/agent';
import { Badge } from '../components/ui';
import { DataTable } from '../components/DataTable';
import { ResultView } from '../components/ResultView';
import { LanShare } from '../components/LanShare';

type Port = {
  protocol: string;
  local_address: string;
  port: number;
  pid: number | null;
  state: string | null;
};

export function Networking() {
  const [domain, setDomain] = useState('workforce.test');
  const [port, setPort] = useState('8000');
  const [plan, setPlan] = useState<any>(null);
  const [err, setErr] = useState('');
  const [ports, setPorts] = useState<Port[]>([]);
  const [portErr, setPortErr] = useState('');
  async function run() {
    setErr('');
    try {
      setPlan(await call('domain.plan', { domain, port: Number(port), tls: true }));
    } catch (e) {
      setErr(String(e));
    }
  }
  async function loadPorts() {
    try {
      setPorts(await call<Port[]>('port.list'));
      setPortErr('');
    } catch (e) {
      setPortErr(String(e));
    }
  }
  useEffect(() => {
    void loadPorts();
  }, []);
  const listening = ports.filter((p) => !p.state || /listen/i.test(p.state));
  return (
    <>
      <LanShare />
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
        <ResultView title="Domain plan" value={plan} error={err} />
        <p className="hint">
          Hosts-file mutation, local CA installation and Caddy binding require the elevated network helper
          instead of normal IPC permissions.
        </p>
      </section>
      <section className="panel" aria-labelledby="ports-title">
        <div className="panelHead">
          <div>
            <span className="kicker">This machine</span>
            <h3 id="ports-title">Listening ports</h3>
          </div>
          <button className="ghost" onClick={() => void loadPorts()}>
            Refresh
          </button>
        </div>
        <DataTable
          caption="Listening ports"
          rows={listening}
          rowKey={(p) => `${p.protocol}-${p.local_address}-${p.port}`}
          empty={portErr || 'No listening ports reported.'}
          columns={[
            { key: 'port', label: 'Port', className: 'num', render: (p) => <code>{p.port}</code> },
            { key: 'proto', label: 'Protocol', render: (p) => p.protocol.toUpperCase() },
            {
              key: 'addr',
              label: 'Address',
              className: 'hideSm',
              render: (p) => <code>{p.local_address}</code>,
            },
            { key: 'pid', label: 'PID', className: 'num', render: (p) => p.pid ?? '—' },
          ]}
        />
      </section>
    </>
  );
}
