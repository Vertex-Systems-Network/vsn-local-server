import { useEffect, useMemo, useState } from 'react';
import { call } from '../api/agent';
import type { SpaceSummary } from '../contracts';
import { Badge } from '../components/ui';
import { DataTable } from '../components/DataTable';
import { Dialog } from '../components/Dialog';
import { useToast } from '../components/Toast';

type FileChange = { code: string; path: string; staged: boolean };
type RepoStatus = {
  is_repo: boolean;
  branch: string | null;
  upstream: string | null;
  ahead: number;
  behind: number;
  files: FileChange[];
};
type Commit = { hash: string; author: string; when: string; subject: string };
type Branch = { name: string; current: boolean };

const CODE_LABEL: Record<string, string> = {
  M: 'Modified',
  A: 'Added',
  D: 'Deleted',
  R: 'Renamed',
  C: 'Copied',
  U: 'Conflict',
  '?': 'New',
};
const describe = (code: string) => CODE_LABEL[code.trim()[0] ?? ''] ?? code.trim();

function git<T>(path: string, action: string, extra: Record<string, unknown> = {}) {
  return call<T>('git', { path, action, ...extra });
}

/** Source control (DVF-08): git status, diff, stage, commit, branches, pull/push per project. */
export function SourceControl() {
  const [spaces, setSpaces] = useState<SpaceSummary[]>([]);
  const [path, setPath] = useState('');
  const [status, setStatus] = useState<RepoStatus | null>(null);
  const [log, setLog] = useState<Commit[]>([]);
  const [branches, setBranches] = useState<Branch[]>([]);
  const [message, setMessage] = useState('');
  const [diff, setDiff] = useState<{ file: string; text: string } | null>(null);
  const [newBranch, setNewBranch] = useState('');
  const [busy, setBusy] = useState('');
  const [err, setErr] = useState('');
  const [cloning, setCloning] = useState(false);
  const [cloneUrl, setCloneUrl] = useState('');
  const [cloneFolder, setCloneFolder] = useState('');
  const toast = useToast();

  useEffect(() => {
    call<SpaceSummary[]>('space.list')
      .then((list) => {
        setSpaces(list);
        if (!path && list[0]) setPath(list[0].root);
      })
      .catch(() => setSpaces([]));
  }, []);

  async function refresh(p = path) {
    if (!p) return;
    setErr('');
    try {
      const st = await git<RepoStatus>(p, 'status');
      setStatus(st);
      if (st.is_repo) {
        const [l, b] = await Promise.all([
          git<Commit[]>(p, 'log', { limit: 50 }),
          git<Branch[]>(p, 'branches'),
        ]);
        setLog(l);
        setBranches(b);
      } else {
        setLog([]);
        setBranches([]);
      }
    } catch (e) {
      setStatus(null);
      setErr(String(e));
    }
  }
  useEffect(() => {
    setDiff(null);
    void refresh();
  }, [path]);

  async function act(label: string, action: string, extra: Record<string, unknown> = {}, done?: string) {
    setBusy(label);
    try {
      const r = await git<{ output?: string }>(path, action, extra);
      toast(done ?? (r?.output?.trim().split('\n')[0] || `${label} done`));
      await refresh();
      return true;
    } catch (e) {
      toast(String(e), 'danger');
      return false;
    } finally {
      setBusy('');
    }
  }
  async function showDiff(f: FileChange) {
    try {
      const r = await git<{ diff: string }>(path, 'diff', { file: f.path, staged: f.staged });
      setDiff({
        file: f.path,
        text: r.diff || (f.code === '??' ? 'New file: stage it to see its contents.' : '(no changes)'),
      });
    } catch (e) {
      toast(String(e), 'danger');
    }
  }

  const staged = useMemo(() => status?.files.filter((f) => f.staged) ?? [], [status]);
  const unstaged = useMemo(() => status?.files.filter((f) => !f.staged || f.code[1] !== ' ') ?? [], [status]);

  return (
    <>
      <section className="panel" aria-labelledby="git-title">
        <div className="panelHead">
          <div>
            <span className="kicker">Git</span>
            <h3 id="git-title">Source control</h3>
          </div>
          <div className="buttonRow">
            <button className="ghost" onClick={() => setCloning(true)}>
              Clone
            </button>
            <button className="ghost" onClick={() => void refresh()}>
              Refresh
            </button>
          </div>
        </div>
        <div className="formRow">
          <label className="stacked">
            Space
            <select
              value={spaces.some((s) => s.root === path) ? path : ''}
              onChange={(e) => setPath(e.target.value)}
            >
              <option value="">Other folder…</option>
              {spaces.map((s) => (
                <option key={s.root} value={s.root}>
                  {s.spec?.name ?? s.root}
                </option>
              ))}
            </select>
          </label>
          <label className="stacked">
            Project folder
            <input value={path} onChange={(e) => setPath(e.target.value)} onBlur={() => void refresh()} />
          </label>
        </div>
        {err && <div className="error">{err}</div>}
        {status && !status.is_repo && (
          <div className="empty">
            This folder is not a git repository yet.{' '}
            <button className="primary" disabled={!!busy} onClick={() => void act('Init', 'init')}>
              Initialize repository
            </button>
          </div>
        )}
        {status?.is_repo && (
          <>
            <div className="heroBadges">
              <Badge ok label={`Branch ${status.branch ?? '—'}`} />
              <Badge
                ok={!!status.upstream}
                label={status.upstream ? `Tracks ${status.upstream}` : 'No upstream'}
              />
              {status.ahead > 0 && <Badge ok label={`${status.ahead} to push`} />}
              {status.behind > 0 && <Badge ok={false} label={`${status.behind} to pull`} />}
            </div>
            <div className="buttonRow">
              <button
                className="ghost"
                disabled={!!busy}
                onClick={() => void act('Fetch', 'fetch', {}, 'Fetched')}
              >
                {busy === 'Fetch' ? 'Fetching…' : 'Fetch'}
              </button>
              <button className="ghost" disabled={!!busy} onClick={() => void act('Pull', 'pull')}>
                {busy === 'Pull' ? 'Pulling…' : 'Pull'}
              </button>
              <button
                className="ghost"
                disabled={!!busy}
                onClick={() => void act('Push', 'push', {}, 'Pushed')}
              >
                {busy === 'Push' ? 'Pushing…' : 'Push'}
              </button>
            </div>
          </>
        )}
      </section>

      {status?.is_repo && (
        <>
          <section className="panel" aria-labelledby="git-changes">
            <div className="panelHead">
              <div>
                <span className="kicker">{status.files.length} changed</span>
                <h3 id="git-changes">Changes</h3>
              </div>
              <button
                className="ghost"
                disabled={!unstaged.length || !!busy}
                onClick={() =>
                  void act('Stage', 'stage', { files: unstaged.map((f) => f.path) }, 'All changes staged')
                }
              >
                Stage all
              </button>
            </div>
            <DataTable
              caption="Changed files"
              rows={status.files}
              rowKey={(f) => `${f.code}${f.path}`}
              empty="Nothing to commit, working tree clean."
              columns={[
                {
                  key: 'file',
                  label: 'File',
                  render: (f) => (
                    <button className="linkButton" onClick={() => void showDiff(f)}>
                      {f.path}
                    </button>
                  ),
                },
                {
                  key: 'state',
                  label: 'State',
                  render: (f) => (
                    <Badge ok={f.staged} label={`${describe(f.code)}${f.staged ? ' · staged' : ''}`} />
                  ),
                },
                {
                  key: 'act',
                  label: 'Actions',
                  className: 'actions',
                  render: (f) => (
                    <>
                      {f.staged ? (
                        <button
                          className="ghost"
                          aria-label={`Unstage ${f.path}`}
                          onClick={() => void act('Unstage', 'unstage', { files: [f.path] }, 'Unstaged')}
                        >
                          Unstage
                        </button>
                      ) : (
                        <button
                          className="ghost"
                          aria-label={`Stage ${f.path}`}
                          onClick={() => void act('Stage', 'stage', { files: [f.path] }, 'Staged')}
                        >
                          Stage
                        </button>
                      )}{' '}
                      {f.code !== '??' && !f.staged && (
                        <button
                          className="ghost"
                          aria-label={`Discard changes to ${f.path}`}
                          onClick={() => {
                            if (window.confirm(`Discard your changes to ${f.path}? This cannot be undone.`))
                              void act('Discard', 'discard', { files: [f.path] }, 'Changes discarded');
                          }}
                        >
                          Discard
                        </button>
                      )}
                    </>
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
                disabled={!staged.length || !message.trim() || !!busy}
                onClick={() =>
                  void act('Commit', 'commit', { message }, `Committed ${staged.length} file(s)`).then(
                    (ok) => ok && setMessage(''),
                  )
                }
              >
                Commit {staged.length ? `${staged.length} staged` : ''}
              </button>
            </div>
          </section>

          {diff && (
            <section className="panel" aria-labelledby="git-diff">
              <div className="panelHead">
                <div>
                  <span className="kicker">Diff</span>
                  <h3 id="git-diff">{diff.file}</h3>
                </div>
                <button className="ghost" onClick={() => setDiff(null)}>
                  Close
                </button>
              </div>
              <pre className="json diffView">
                {diff.text.split('\n').map((line, i) => (
                  <span
                    key={i}
                    className={
                      line.startsWith('+') && !line.startsWith('+++')
                        ? 'add'
                        : line.startsWith('-') && !line.startsWith('---')
                          ? 'del'
                          : line.startsWith('@@')
                            ? 'hunk'
                            : undefined
                    }
                  >
                    {line + '\n'}
                  </span>
                ))}
              </pre>
            </section>
          )}

          <div className="overviewGrid">
            <section className="panel" aria-labelledby="git-branches">
              <div className="panelHead">
                <div>
                  <span className="kicker">{branches.length} local</span>
                  <h3 id="git-branches">Branches</h3>
                </div>
              </div>
              <ul className="historyList">
                {branches.map((b) => (
                  <li key={b.name}>
                    <b>{b.name}</b>
                    {b.current ? (
                      <Badge ok label="Current" />
                    ) : (
                      <button
                        className="ghost"
                        disabled={!!busy}
                        onClick={() =>
                          void act('Switch', 'switch', { branch: b.name }, `Switched to ${b.name}`)
                        }
                      >
                        Switch
                      </button>
                    )}
                  </li>
                ))}
              </ul>
              <div className="formRow">
                <label className="stacked">
                  New branch
                  <input
                    value={newBranch}
                    onChange={(e) => setNewBranch(e.target.value.trim())}
                    placeholder="feature/login"
                  />
                </label>
              </div>
              <button
                className="ghost"
                disabled={!newBranch || !!busy}
                onClick={() =>
                  void act(
                    'Branch',
                    'switch',
                    { branch: newBranch, create: true },
                    `Created ${newBranch}`,
                  ).then((ok) => ok && setNewBranch(''))
                }
              >
                Create and switch
              </button>
            </section>
            <section className="panel" aria-labelledby="git-log">
              <div className="panelHead">
                <div>
                  <span className="kicker">Latest {log.length}</span>
                  <h3 id="git-log">History</h3>
                </div>
              </div>
              {log.length === 0 ? (
                <p className="hint">No commits yet.</p>
              ) : (
                <ul className="historyList">
                  {log.map((c) => (
                    <li key={c.hash}>
                      <code>{c.hash}</code> {c.subject}
                      <small>
                        {c.author} · {c.when}
                      </small>
                    </li>
                  ))}
                </ul>
              )}
            </section>
          </div>
        </>
      )}

      {cloning && (
        <Dialog title="Clone a repository" onClose={() => setCloning(false)}>
          <label className="stacked">
            Repository URL (https://, ssh:// or git@)
            <input
              value={cloneUrl}
              onChange={(e) => setCloneUrl(e.target.value.trim())}
              placeholder="https://github.com/you/app.git"
            />
          </label>
          <label className="stacked">
            Into this folder (inside a workspace root)
            <input value={path} onChange={(e) => setPath(e.target.value)} />
          </label>
          <label className="stacked">
            New folder name
            <input
              value={cloneFolder}
              onChange={(e) => setCloneFolder(e.target.value.trim())}
              placeholder="app"
            />
          </label>
          <p className="hint">
            Private repositories use the git credentials already set up on this PC (Git Credential Manager or
            SSH keys). VSN never asks for or stores them.
          </p>
          <div className="buttonRow">
            <button
              className="primary"
              disabled={!cloneUrl || !cloneFolder || !!busy}
              onClick={() =>
                void act(
                  'Clone',
                  'clone',
                  { url: cloneUrl, folder: cloneFolder },
                  `Cloned into ${cloneFolder}`,
                ).then((ok) => {
                  if (ok) {
                    setCloning(false);
                    setPath(`${path.replace(/[\\/]+$/, '')}/${cloneFolder}`);
                  }
                })
              }
            >
              {busy === 'Clone' ? 'Cloning…' : 'Clone'}
            </button>
            <button className="ghost" onClick={() => setCloning(false)}>
              Cancel
            </button>
          </div>
        </Dialog>
      )}
    </>
  );
}
