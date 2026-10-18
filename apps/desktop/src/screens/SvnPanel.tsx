import { useEffect, useState } from 'react';
import { call } from '../api/agent';
import { Badge } from '../components/ui';
import { DataTable } from '../components/DataTable';
import { useToast } from '../components/Toast';

type SvnChange = { code: string; path: string };
type SvnStatus = {
  is_working_copy: boolean;
  url: string | null;
  revision: string | null;
  files: SvnChange[];
};

const LABEL: Record<string, string> = {
  M: 'Modified',
  A: 'Added',
  D: 'Deleted',
  R: 'Replaced',
  C: 'Conflict',
  '?': 'Not versioned',
  '!': 'Missing',
  '~': 'Obstructed',
  I: 'Ignored',
};

function svn<T>(path: string, action: string, extra: Record<string, unknown> = {}) {
  return call<T>('svn', { path, action, ...extra });
}

/** Subversion basics (status, update, add, revert, commit) for a folder that is an svn working copy. */
export function SvnPanel({ path }: { path: string }) {
  const [status, setStatus] = useState<SvnStatus | null>(null);
  const [message, setMessage] = useState('');
  const [busy, setBusy] = useState('');
  const toast = useToast();

  async function refresh() {
    try {
      setStatus(await svn<SvnStatus>(path, 'status'));
    } catch {
      setStatus(null);
    }
  }
  useEffect(() => {
    setStatus(null);
    if (path) void refresh();
  }, [path]);

  async function act(label: string, action: string, extra: Record<string, unknown> = {}) {
    setBusy(label);
    try {
      const r = await svn<{ output?: string }>(path, action, extra);
      const lines = r?.output?.trim().split('\n') ?? [];
      toast(lines[lines.length - 1] || `${label} done`);
      await refresh();
      return true;
    } catch (e) {
      toast(String(e), 'danger');
      return false;
    } finally {
      setBusy('');
    }
  }

  if (!status?.is_working_copy) return null;
  const pending = status.files.filter((f) => f.code !== '?' && f.code !== 'I');
  return (
    <section className="panel" aria-labelledby="svn-title">
      <div className="panelHead">
        <div>
          <span className="kicker">Subversion</span>
          <h3 id="svn-title">SVN working copy</h3>
        </div>
        <div className="buttonRow">
          <button className="ghost" disabled={!!busy} onClick={() => void act('Update', 'update')}>
            {busy === 'Update' ? 'Updating…' : 'Update'}
          </button>
          <button className="ghost" onClick={() => void refresh()}>
            Refresh
          </button>
        </div>
      </div>
      <div className="heroBadges">
        <Badge ok label={`Revision ${status.revision ?? '—'}`} />
        {status.url && <Badge ok label={status.url} />}
      </div>
      <DataTable
        caption="SVN changes"
        rows={status.files}
        rowKey={(f) => `${f.code}${f.path}`}
        empty="No local changes."
        columns={[
          { key: 'file', label: 'File', render: (f) => <code>{f.path}</code> },
          {
            key: 'state',
            label: 'State',
            render: (f) => <Badge ok={f.code !== 'C' && f.code !== '!'} label={LABEL[f.code] ?? f.code} />,
          },
          {
            key: 'act',
            label: 'Actions',
            className: 'actions',
            render: (f) =>
              f.code === '?' ? (
                <button
                  className="ghost"
                  aria-label={`Add ${f.path}`}
                  onClick={() => void act('Add', 'add', { files: [f.path] })}
                >
                  Add
                </button>
              ) : (
                <button
                  className="ghost"
                  aria-label={`Revert ${f.path}`}
                  onClick={() => {
                    if (window.confirm(`Throw away your changes to ${f.path}?`))
                      void act('Revert', 'revert', { files: [f.path] });
                  }}
                >
                  Revert
                </button>
              ),
          },
        ]}
      />
      <label className="stacked">
        Commit message
        <textarea rows={3} value={message} onChange={(e) => setMessage(e.target.value)} />
      </label>
      <div className="buttonRow">
        <button
          className="primary"
          disabled={!message.trim() || pending.length === 0 || !!busy}
          onClick={() =>
            void act('Commit', 'commit', { message }).then((ok) => {
              if (ok) setMessage('');
            })
          }
        >
          {busy === 'Commit'
            ? 'Committing…'
            : `Commit ${pending.length} change${pending.length === 1 ? '' : 's'}`}
        </button>
      </div>
      <p className="hint">
        Uses the svn command line on this PC and the credentials it already has; VSN never asks for or stores
        them.
      </p>
    </section>
  );
}
