import { useEffect, useState } from 'react';
import { call } from '../api/agent';
import { Badge } from './ui';
import { DataTable } from './DataTable';
import { Dialog } from './Dialog';
import { useToast } from './Toast';

type Managed = { id: string; pid: number; program: string; started_at_unix: number; running: boolean };

/** Processes VSN started itself; only these can be stopped from the app (Figma: "30 Processes — Confirm stop"). */
export function ManagedProcesses() {
  const [rows, setRows] = useState<Managed[]>([]);
  const [err, setErr] = useState('');
  const [confirm, setConfirm] = useState<Managed | null>(null);
  const toast = useToast();
  async function load() {
    try {
      setRows(await call<Managed[]>('process.managed.list'));
      setErr('');
    } catch (e) {
      setErr(String(e));
    }
  }
  useEffect(() => {
    void load();
  }, []);
  async function stop(p: Managed) {
    setConfirm(null);
    try {
      await call('process.managed.stop', { id: p.id });
      toast(`Stopped ${p.id}`);
      await load();
    } catch (e) {
      toast(String(e), 'danger');
    }
  }
  return (
    <section className="panel" aria-labelledby="managed-title">
      <div className="panelHead">
        <div>
          <span className="kicker">Started by VSN</span>
          <h3 id="managed-title">Managed processes</h3>
        </div>
        <span>{rows.filter((r) => r.running).length} running</span>
      </div>
      <DataTable
        caption="Managed processes"
        rows={rows}
        rowKey={(p) => p.id}
        empty="VSN has not started any background processes."
        columns={[
          { key: 'id', label: 'Name', render: (p) => <b>{p.id}</b> },
          { key: 'pid', label: 'PID', className: 'num', render: (p) => <code>{p.pid}</code> },
          { key: 'prog', label: 'Program', className: 'hideSm', render: (p) => <code>{p.program}</code> },
          {
            key: 'started',
            label: 'Started',
            className: 'hideSm',
            render: (p) => new Date(p.started_at_unix * 1000).toLocaleString(),
          },
          {
            key: 'state',
            label: 'Status',
            render: (p) => <Badge ok={p.running} label={p.running ? 'Running' : 'Stopped'} />,
          },
          {
            key: 'act',
            label: 'Actions',
            className: 'actions',
            render: (p) =>
              p.running ? (
                <button className="danger" onClick={() => setConfirm(p)} aria-label={`Stop ${p.id}`}>
                  Stop
                </button>
              ) : (
                '—'
              ),
          },
        ]}
      />
      {err && (
        <div className="error" role="alert">
          {err}
        </div>
      )}
      {confirm && (
        <Dialog title={`Stop ${confirm.id}?`} onClose={() => setConfirm(null)}>
          <p>
            PID {confirm.pid} will be stopped. Anything that depends on it loses its connection until it is
            started again.
          </p>
          <div className="buttonRow">
            <button className="ghost" onClick={() => setConfirm(null)}>
              Cancel
            </button>
            <button className="danger" onClick={() => void stop(confirm)}>
              Stop process
            </button>
          </div>
        </Dialog>
      )}
    </section>
  );
}
