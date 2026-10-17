import { useEffect, useState, type ChangeEvent } from 'react';
import { call } from '../api/agent';
import { Badge } from './ui';
import { DataTable } from './DataTable';

/** Workspace roots the agent may read and write (Projects and Settings → Workspace roots). */
export function WorkspaceRoots({ onInspect }: { onInspect?: (root: string) => void }) {
  const [root, setRoot] = useState('');
  const [roots, setRoots] = useState<string[]>([]);
  const [err, setErr] = useState('');
  async function load() {
    try {
      setRoots(await call<string[]>('workspace.roots'));
    } catch (e) {
      setErr(String(e));
    }
  }
  useEffect(() => {
    void load();
  }, []);
  async function rootAction(action: 'add' | 'remove', target = root) {
    setErr('');
    try {
      await call(action === 'add' ? 'workspace.add' : 'workspace.remove', { path: target });
      if (action === 'add') setRoot('');
      await load();
    } catch (e) {
      setErr(String(e));
    }
  }
  return (
      <section className="panel">
        <div className="panelHead">
          <div>
            <span className="kicker">Workspace sandbox</span>
            <h3>Project roots</h3>
          </div>
          <Badge ok={roots.length > 0} label={`${roots.length} roots`} />
        </div>
        <div className="formRow">
          <input
            value={root}
            onChange={(e: ChangeEvent<HTMLInputElement>) => setRoot(e.target.value)}
            placeholder="Absolute workspace root"
          />
          <button className="primary" onClick={() => void rootAction('add')}>
            Add
          </button>
        </div>
        <DataTable
          caption="Workspace roots"
          rows={roots}
          rowKey={(r) => r}
          empty="No workspace roots yet. Add a folder above to let VSN detect projects in it."
          columns={[
            { key: 'path', label: 'Root', render: (r) => <code>{r}</code> },
            {
              key: 'act',
              label: 'Actions',
              className: 'actions',
              render: (r) => (
                <>
                  {onInspect && (
                    <>
                      <button className="ghost" onClick={() => onInspect(r)} aria-label={`Inspect ${r}`}>
                        Inspect
                      </button>{' '}
                    </>
                  )}
                  <button className="ghost" onClick={() => void rootAction('remove', r)} aria-label={`Remove ${r}`}>
                    Remove
                  </button>
                </>
              ),
            },
          ]}
        />
        {err && <div className="error" role="alert">{err}</div>}
      </section>
  );
}
