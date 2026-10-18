import { useEffect, useState } from 'react';
import { call } from '../api/agent';
import type { SpaceSummary } from '../contracts';
import { Badge } from '../components/ui';
import { DataTable } from '../components/DataTable';
import { useToast } from '../components/Toast';
import { formatBytes } from './Analytics';

type Protocol = 'ftp' | 'ftps' | 'sftp';
type Profile = {
  protocol: Protocol;
  host: string;
  port?: number | null;
  user: string;
  remote_dir: string;
  key_path?: string | null;
};
type Settings = { profile: Profile; has_password: boolean };
type Entry = { name: string; is_dir: boolean; size: number | null };

const DEFAULT_PORT: Record<Protocol, number> = { ftp: 21, ftps: 21, sftp: 22 };
const EMPTY: Profile = { protocol: 'sftp', host: '', port: null, user: '', remote_dir: '/', key_path: null };

/** Deploy (DVF-07): FTP, FTPS and SFTP per space — browse, upload and download. */
export function Deploy() {
  const [spaces, setSpaces] = useState<SpaceSummary[]>([]);
  const [root, setRoot] = useState('');
  const [saved, setSaved] = useState<Settings | null>(null);
  const [profile, setProfile] = useState<Profile>(EMPTY);
  const [password, setPassword] = useState('');
  const [cwd, setCwd] = useState('');
  const [entries, setEntries] = useState<Entry[] | null>(null);
  const [local, setLocal] = useState('');
  const [localDir, setLocalDir] = useState('');
  const [busy, setBusy] = useState('');
  const [err, setErr] = useState('');
  const toast = useToast();
  const space = spaces.find((s) => s.root === root);

  useEffect(() => {
    call<SpaceSummary[]>('space.list')
      .then((l) => {
        const ok = l.filter((s) => s.spec);
        setSpaces(ok);
        if (ok[0]) setRoot(ok[0].root);
      })
      .catch((e) => setErr(String(e)));
  }, []);
  useEffect(() => {
    if (!root) return;
    setEntries(null);
    setCwd('');
    call<Settings | null>('space.transfer.get', { path: root })
      .then((s) => {
        setSaved(s);
        setProfile(s?.profile ?? EMPTY);
      })
      .catch(() => setSaved(null));
  }, [root]);
  const set = (patch: Partial<Profile>) => setProfile((p) => ({ ...p, ...patch }));

  async function save() {
    setBusy('save');
    try {
      const s = await call<Settings>('space.transfer.set', {
        path: root,
        profile: { ...profile, port: profile.port || null, key_path: profile.key_path || null },
        ...(password ? { password } : {}),
      });
      setSaved(s);
      setPassword('');
      toast('Server saved');
      await browse('');
    } catch (e) {
      toast(String(e), 'danger');
    } finally {
      setBusy('');
    }
  }
  async function browse(dir: string) {
    setBusy('list');
    setErr('');
    try {
      setEntries(await call<Entry[]>('space.transfer.list', { path: root, remote: dir }));
      setCwd(dir);
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy('');
    }
  }
  async function upload() {
    setBusy('upload');
    try {
      await call('space.transfer.upload', { path: root, local, remote: cwd });
      toast(`Uploaded ${local}`);
      await browse(cwd);
    } catch (e) {
      toast(String(e), 'danger');
    } finally {
      setBusy('');
    }
  }
  async function download(name: string) {
    setBusy('download');
    try {
      const dest = await call<string>('space.transfer.download', {
        path: root,
        remote: cwd ? `${cwd}/${name}` : name,
        local: localDir,
      });
      toast(`Saved to ${dest}`);
    } catch (e) {
      toast(String(e), 'danger');
    } finally {
      setBusy('');
    }
  }

  if (!spaces.length)
    return (
      <section className="panel" aria-labelledby="dep-title">
        <div className="panelHead">
          <h3 id="dep-title">Deploy</h3>
        </div>
        <div className="empty">{err || 'Create a space first; each space has its own deploy server.'}</div>
      </section>
    );

  return (
    <>
      <section className="panel" aria-labelledby="dep-title">
        <div className="panelHead">
          <div>
            <span className="kicker">FTP · FTPS · SFTP</span>
            <h3 id="dep-title">Deploy server</h3>
          </div>
          <Badge
            ok={!!saved}
            label={saved ? `${saved.profile.protocol.toUpperCase()} ${saved.profile.host}` : 'Not set'}
          />
        </div>
        <div className="formRow">
          <label className="stacked">
            Space
            <select value={root} onChange={(e) => setRoot(e.target.value)}>
              {spaces.map((s) => (
                <option key={s.root} value={s.root}>
                  {s.spec?.name}
                </option>
              ))}
            </select>
          </label>
          <label className="stacked">
            Protocol
            <select value={profile.protocol} onChange={(e) => set({ protocol: e.target.value as Protocol })}>
              <option value="sftp">SFTP (SSH)</option>
              <option value="ftps">FTPS (FTP + TLS)</option>
              <option value="ftp">FTP (unencrypted)</option>
            </select>
          </label>
          <label className="stacked">
            Host
            <input
              value={profile.host}
              placeholder="example.com"
              onChange={(e) => set({ host: e.target.value.trim() })}
            />
          </label>
          <label className="stacked">
            Port
            <input
              type="number"
              min={1}
              max={65535}
              placeholder={String(DEFAULT_PORT[profile.protocol])}
              value={profile.port ?? ''}
              onChange={(e) => set({ port: Number(e.target.value) || null })}
            />
          </label>
        </div>
        <div className="formRow">
          <label className="stacked">
            User
            <input value={profile.user} onChange={(e) => set({ user: e.target.value.trim() })} />
          </label>
          {profile.protocol === 'sftp' ? (
            <label className="stacked">
              SSH key file (optional)
              <input
                value={profile.key_path ?? ''}
                placeholder="C:\Users\you\.ssh\id_ed25519"
                onChange={(e) => set({ key_path: e.target.value })}
              />
            </label>
          ) : (
            <label className="stacked">
              Password {saved?.has_password ? '(saved — leave empty to keep)' : ''}
              <input
                type="password"
                autoComplete="new-password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
            </label>
          )}
          <label className="stacked">
            Remote folder
            <input
              value={profile.remote_dir}
              placeholder="/public_html"
              onChange={(e) => set({ remote_dir: e.target.value })}
            />
          </label>
        </div>
        {profile.protocol === 'ftp' && (
          <p className="hint">
            Plain FTP sends your password unencrypted. Prefer SFTP or FTPS when the host offers it.
          </p>
        )}
        {profile.protocol === 'sftp' && (
          <p className="hint">
            SFTP uses SSH keys (your default key or the file above); add the public key on the server first.
            FTP passwords are kept in the VSN vault, never in the project.
          </p>
        )}
        <div className="buttonRow">
          <button
            className="primary"
            disabled={!profile.host || !profile.user || !!busy}
            onClick={() => void save()}
          >
            {busy === 'save' ? 'Saving…' : 'Save and connect'}
          </button>
          {saved && (
            <button className="ghost" disabled={!!busy} onClick={() => void browse(cwd)}>
              {busy === 'list' ? 'Loading…' : 'Refresh'}
            </button>
          )}
        </div>
        {err && <div className="error">{err}</div>}
      </section>

      {saved && entries && (
        <section className="panel" aria-labelledby="dep-files">
          <div className="panelHead">
            <div>
              <span className="kicker">{space?.spec?.name}</span>
              <h3 id="dep-files">
                Remote: {`${saved.profile.remote_dir.replace(/\/$/, '')}/${cwd}`.replace(/\/+/g, '/') || '/'}
              </h3>
            </div>
            <button
              className="ghost"
              disabled={!cwd}
              onClick={() => void browse(cwd.split('/').slice(0, -1).join('/'))}
            >
              Up one folder
            </button>
          </div>
          <DataTable
            caption="Remote files"
            rows={entries}
            rowKey={(e) => e.name}
            empty="This folder is empty."
            columns={[
              {
                key: 'name',
                label: 'Name',
                render: (e) =>
                  e.is_dir ? (
                    <button
                      className="linkButton"
                      onClick={() => void browse(cwd ? `${cwd}/${e.name}` : e.name)}
                    >
                      <b>{e.name}/</b>
                    </button>
                  ) : (
                    e.name
                  ),
              },
              {
                key: 'size',
                label: 'Size',
                className: 'num',
                render: (e) => (e.is_dir ? '—' : formatBytes(e.size)),
              },
              {
                key: 'act',
                label: 'Actions',
                className: 'actions',
                render: (e) =>
                  e.is_dir ? null : (
                    <button
                      className="ghost"
                      disabled={!!busy}
                      aria-label={`Download ${e.name}`}
                      onClick={() => void download(e.name)}
                    >
                      Download
                    </button>
                  ),
              },
            ]}
          />
          <div className="formRow">
            <label className="stacked">
              Upload this file (path inside the space)
              <input
                value={local}
                placeholder="public/index.html"
                onChange={(e) => setLocal(e.target.value)}
              />
            </label>
            <label className="stacked">
              Downloads go to (folder inside the space)
              <input
                value={localDir}
                placeholder="(space folder)"
                onChange={(e) => setLocalDir(e.target.value)}
              />
            </label>
          </div>
          <button className="primary" disabled={!local || !!busy} onClick={() => void upload()}>
            {busy === 'upload' ? 'Uploading…' : 'Upload here'}
          </button>
        </section>
      )}
    </>
  );
}
