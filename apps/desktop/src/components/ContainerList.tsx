import { useEffect, useState } from 'react';
import { call } from '../api/agent';
import type { ContainerBackend } from '../contracts';
import { Badge } from './ui';
import { DataTable } from './DataTable';
import { Dialog } from './Dialog';
import { useToast } from './Toast';

type Container = { id: string; name: string; image: string; status: string; ports: string };
type Action = 'start' | 'stop' | 'restart';

/** Containers on one reachable engine, with start/stop/restart (stop asks first). */
export function ContainerList({ backend }: { backend: ContainerBackend }) {
  const [rows, setRows] = useState<Container[]>([]);
  const [err, setErr] = useState('');
  const [confirm, setConfirm] = useState<Container | null>(null);
  const toast = useToast();
  async function load() {
    try {
      setRows(await call<Container[]>('container.list', { backend: backend.id, all: true }));
      setErr('');
    } catch (e) {
      setErr(String(e));
    }
  }
  useEffect(() => {
    void load();
  }, [backend.id]);
  async function act(c: Container, action: Action) {
    try {
      await call('container.action', { backend: backend.id, action, target: c.id });
      toast(`${c.name}: ${action} requested`);
      await load();
    } catch (e) {
      toast(String(e), 'danger');
    }
  }
  const running = (c: Container) => /^up\b|running/i.test(c.status);
  return (
    <section className="panel" aria-labelledby={`ct-${backend.id}`}>
      <div className="panelHead">
        <div>
          <span className="kicker">{backend.id}</span>
          <h3 id={`ct-${backend.id}`}>Containers</h3>
        </div>
        <button className="ghost" onClick={() => void load()}>
          Refresh
        </button>
      </div>
      <DataTable
        caption={`${backend.id} containers`}
        rows={rows}
        rowKey={(c) => c.id}
        empty="No containers on this engine."
        columns={[
          { key: 'name', label: 'Name', render: (c) => <b>{c.name}</b> },
          { key: 'image', label: 'Image', className: 'hideSm', render: (c) => <code>{c.image}</code> },
          {
            key: 'status',
            label: 'Status',
            render: (c) => <Badge ok={running(c)} label={c.status || 'unknown'} />,
          },
          { key: 'ports', label: 'Ports', className: 'hideSm', render: (c) => c.ports || '—' },
          {
            key: 'act',
            label: 'Actions',
            className: 'actions',
            render: (c) =>
              running(c) ? (
                <>
                  <button
                    className="ghost"
                    onClick={() => void act(c, 'restart')}
                    aria-label={`Restart ${c.name}`}
                  >
                    Restart
                  </button>{' '}
                  <button className="danger" onClick={() => setConfirm(c)} aria-label={`Stop ${c.name}`}>
                    Stop
                  </button>
                </>
              ) : (
                <button className="ghost" onClick={() => void act(c, 'start')} aria-label={`Start ${c.name}`}>
                  Start
                </button>
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
        <Dialog title={`Stop ${confirm.name}?`} onClose={() => setConfirm(null)}>
          <p>Services inside this container stop until it is started again.</p>
          <div className="buttonRow">
            <button className="ghost" onClick={() => setConfirm(null)}>
              Cancel
            </button>
            <button
              className="danger"
              onClick={() => {
                const c = confirm;
                setConfirm(null);
                void act(c, 'stop');
              }}
            >
              Stop container
            </button>
          </div>
        </Dialog>
      )}
    </section>
  );
}
