import { useEffect, useState } from 'react';
import { call } from '../api/agent';
import type { SpaceSummary } from '../contracts';
import { DataTable } from '../components/DataTable';
import { useToast } from '../components/Toast';

type Destination = { id: string; name: string; kind: 'local' | 'rclone'; target: string; keep: number };
type BackupRecord = {
  space: string;
  destination: string;
  archive: string;
  bytes: number;
  at_unix: number;
  databases: string[];
};

/** Cloud providers: OAuth ones sign in in the browser; the rest ask for keys or a password. */
const PROVIDERS: { id: string; label: string; fields: [string, string, boolean][] }[] = [
  { id: 'drive', label: 'Google Drive', fields: [] },
  { id: 'onedrive', label: 'Microsoft OneDrive', fields: [] },
  { id: 'dropbox', label: 'Dropbox', fields: [] },
  { id: 'box', label: 'Box', fields: [] },
  { id: 'pcloud', label: 'pCloud', fields: [] },
  {
    id: 'mega',
    label: 'MEGA',
    fields: [
      ['user', 'MEGA email', false],
      ['pass', 'MEGA password', true],
    ],
  },
  {
    id: 's3',
    label: 'Amazon S3 / Wasabi / R2 / MinIO',
    fields: [
      ['provider', 'S3 provider (AWS, Wasabi, Cloudflare, Minio, Other)', false],
      ['access_key_id', 'Access key ID', false],
      ['secret_access_key', 'Secret access key', true],
      ['region', 'Region (e.g. us-east-1)', false],
      ['endpoint', 'Endpoint (leave empty for AWS)', false],
    ],
  },
  {
    id: 'b2',
    label: 'Backblaze B2',
    fields: [
      ['account', 'Key ID', false],
      ['key', 'Application key', true],
    ],
  },
  {
    id: 'webdav',
    label: 'WebDAV / Nextcloud / ownCloud',
    fields: [
      ['url', 'WebDAV URL', false],
      ['vendor', 'Vendor (nextcloud, owncloud, other)', false],
      ['user', 'User', false],
      ['pass', 'Password', true],
    ],
  },
  {
    id: 'sftp',
    label: 'SFTP server',
    fields: [
      ['host', 'Host', false],
      ['user', 'User', false],
      ['pass', 'Password', true],
    ],
  },
  {
    id: 'ftp',
    label: 'FTP server',
    fields: [
      ['host', 'Host', false],
      ['user', 'User', false],
      ['pass', 'Password', true],
    ],
  },
];

function size(bytes: number) {
  if (bytes > 1 << 30) return `${(bytes / (1 << 30)).toFixed(1)} GB`;
  if (bytes > 1 << 20) return `${(bytes / (1 << 20)).toFixed(1)} MB`;
  return `${Math.max(1, Math.round(bytes / 1024))} KB`;
}

/** Backups anywhere (DVF-15): connect accounts, choose destinations, back spaces up. */
export function Backups() {
  const toast = useToast();
  const [spaces, setSpaces] = useState<SpaceSummary[]>([]);
  const [remotes, setRemotes] = useState<string[]>([]);
  const [destinations, setDestinations] = useState<Destination[]>([]);
  const [history, setHistory] = useState<BackupRecord[]>([]);
  const [busy, setBusy] = useState('');
  const [provider, setProvider] = useState('drive');
  const [remoteName, setRemoteName] = useState('gdrive');
  const [fields, setFields] = useState<Record<string, string>>({});
  const [dest, setDest] = useState<Destination>({ id: '', name: '', kind: 'local', target: '', keep: 7 });
  const [space, setSpace] = useState('');
  const [destinationId, setDestinationId] = useState('');
  const [withDatabases, setWithDatabases] = useState(true);

  async function load() {
    const [s, r, d, h] = await Promise.all([
      call<SpaceSummary[]>('space.list').catch(() => []),
      call<string[]>('backup.remote.list').catch(() => []),
      call<Destination[]>('backup.destination.list').catch(() => []),
      call<BackupRecord[]>('backup.history').catch(() => []),
    ]);
    setSpaces(s);
    setRemotes(r);
    setDestinations(d);
    setHistory([...h].reverse());
    if (!space && s[0]) setSpace(s[0].root);
    if (!destinationId && d[0]) setDestinationId(d[0].id);
  }
  useEffect(() => {
    void load();
  }, []);

  async function run<T>(label: string, work: () => Promise<T>, done: (r: T) => string) {
    setBusy(label);
    try {
      const r = await work();
      toast(done(r));
      await load();
    } catch (e) {
      toast(String(e), 'danger');
    } finally {
      setBusy('');
    }
  }

  const chosen = PROVIDERS.find((p) => p.id === provider) ?? PROVIDERS[0];
  return (
    <>
      <section className="panel" aria-labelledby="backup-run">
        <div className="panelHead">
          <div>
            <span className="kicker">Backups anywhere</span>
            <h3 id="backup-run">Back up a space</h3>
          </div>
        </div>
        <div className="formGrid">
          <label>
            Space
            <select value={space} onChange={(e) => setSpace(e.target.value)}>
              {spaces.map((s) => (
                <option key={s.root} value={s.root}>
                  {s.spec?.name ?? s.root}
                </option>
              ))}
            </select>
          </label>
          <label>
            Destination
            <select value={destinationId} onChange={(e) => setDestinationId(e.target.value)}>
              {destinations.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name}
                </option>
              ))}
            </select>
          </label>
          <label className="checkRow">
            <input
              type="checkbox"
              checked={withDatabases}
              onChange={(e) => setWithDatabases(e.target.checked)}
            />
            Include fresh database dumps (services must be running)
          </label>
        </div>
        <div className="buttonRow">
          <button
            className="primary"
            disabled={!space || !destinationId || !!busy}
            onClick={() =>
              void run(
                'run',
                () =>
                  call<BackupRecord>('space.backup.run', {
                    path: space,
                    destination: destinationId,
                    databases: withDatabases,
                  }),
                (r) => `Backed up ${r.space} (${size(r.bytes)}) to ${r.destination}`,
              )
            }
          >
            {busy === 'run' ? 'Backing up…' : 'Back up now'}
          </button>
        </div>
        <p className="hint">
          The archive holds the space folder without node_modules, vendor, .git and build output, plus
          database dumps. It can hold secrets such as .env, so choose destinations you trust.
        </p>
      </section>

      <section className="panel" aria-labelledby="backup-dest">
        <div className="panelHead">
          <div>
            <span className="kicker">Where backups go</span>
            <h3 id="backup-dest">Destinations</h3>
          </div>
        </div>
        <DataTable
          caption="Backup destinations"
          rows={destinations}
          rowKey={(d) => d.id}
          empty="No destinations yet. Add a local folder or a connected cloud account below."
          columns={[
            { key: 'name', label: 'Name', render: (d) => d.name },
            { key: 'target', label: 'Location', render: (d) => <code>{d.target}</code> },
            { key: 'keep', label: 'Keeps', render: (d) => (d.kind === 'local' && d.keep ? d.keep : 'All') },
            {
              key: 'act',
              label: 'Actions',
              className: 'actions',
              render: (d) => (
                <button
                  className="danger"
                  aria-label={`Remove destination ${d.name}`}
                  onClick={() =>
                    void run(
                      'del',
                      () => call('backup.destination.delete', { id: d.id }),
                      () => 'Destination removed',
                    )
                  }
                >
                  Remove
                </button>
              ),
            },
          ]}
        />
        <div className="formGrid">
          <label>
            Type
            <select
              value={dest.kind}
              onChange={(e) => setDest({ ...dest, kind: e.target.value as Destination['kind'] })}
            >
              <option value="local">Folder on this PC, a USB disk or a synced folder</option>
              <option value="rclone">Connected cloud account</option>
            </select>
          </label>
          <label>
            Name
            <input value={dest.name} onChange={(e) => setDest({ ...dest, name: e.target.value })} />
          </label>
          <label>
            {dest.kind === 'local' ? 'Folder (full path)' : 'Account and folder (account:folder)'}
            <input
              value={dest.target}
              placeholder={dest.kind === 'local' ? 'D:\\Backups' : `${remotes[0] ?? 'gdrive'}:Backups/VSN`}
              onChange={(e) => setDest({ ...dest, target: e.target.value })}
            />
          </label>
          {dest.kind === 'local' && (
            <label>
              Keep newest
              <input
                inputMode="numeric"
                value={String(dest.keep)}
                onChange={(e) => setDest({ ...dest, keep: Number(e.target.value.replace(/\D/g, '') || 0) })}
              />
            </label>
          )}
        </div>
        <div className="buttonRow">
          <button
            className="ghost"
            disabled={!dest.name || !dest.target || !!busy}
            onClick={() =>
              void run(
                'dest',
                () =>
                  call('backup.destination.save', {
                    ...dest,
                    id:
                      dest.name
                        .toLowerCase()
                        .replace(/[^a-z0-9_-]+/g, '-')
                        .replace(/^-+/, '') || 'backup',
                  }),
                () => 'Destination saved',
              )
            }
          >
            Add destination
          </button>
        </div>
      </section>

      <section className="panel" aria-labelledby="backup-accounts">
        <div className="panelHead">
          <div>
            <span className="kicker">Google Drive, OneDrive, Dropbox, S3, MEGA …</span>
            <h3 id="backup-accounts">Cloud accounts</h3>
          </div>
        </div>
        <p className="hint">
          Connected: {remotes.length ? remotes.join(', ') : 'none yet'}. Google, Microsoft, Dropbox, Box and
          pCloud open your browser to sign in; keys and passwords are stored encrypted by rclone in VSN's
          private folder. Needs rclone (winget install Rclone.Rclone).
        </p>
        <div className="formGrid">
          <label>
            Service
            <select
              value={provider}
              onChange={(e) => {
                setProvider(e.target.value);
                setFields({});
              }}
            >
              {PROVIDERS.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.label}
                </option>
              ))}
            </select>
          </label>
          <label>
            Account name
            <input
              value={remoteName}
              onChange={(e) => setRemoteName(e.target.value.replace(/[^A-Za-z0-9_-]/g, ''))}
            />
          </label>
          {chosen.fields.map(([key, label, secret]) => (
            <label key={key}>
              {label}
              <input
                type={secret ? 'password' : 'text'}
                autoComplete="off"
                value={fields[key] ?? ''}
                onChange={(e) => setFields({ ...fields, [key]: e.target.value })}
              />
            </label>
          ))}
        </div>
        <div className="buttonRow">
          <button
            className="primary"
            disabled={!remoteName || !!busy}
            onClick={() =>
              void run(
                'connect',
                () =>
                  call('backup.remote.create', {
                    name: remoteName,
                    provider,
                    params: Object.fromEntries(Object.entries(fields).filter(([, v]) => v)),
                  }),
                () => `${chosen.label} connected as ${remoteName}`,
              ).then(() => setFields({}))
            }
          >
            {busy === 'connect' ? 'Waiting for sign-in…' : `Connect ${chosen.label}`}
          </button>
          {remotes.map((r) => (
            <button
              key={r}
              className="ghost"
              aria-label={`Disconnect ${r}`}
              onClick={() =>
                void run(
                  'disc',
                  () => call('backup.remote.delete', { name: r }),
                  () => `${r} disconnected`,
                )
              }
            >
              Disconnect {r}
            </button>
          ))}
        </div>
      </section>

      <section className="panel" aria-labelledby="backup-history">
        <div className="panelHead">
          <div>
            <span className="kicker">Done</span>
            <h3 id="backup-history">Recent backups</h3>
          </div>
        </div>
        <DataTable
          caption="Recent backups"
          rows={history.slice(0, 50)}
          rowKey={(h) => `${h.at_unix}-${h.archive}`}
          empty="No backups yet."
          columns={[
            { key: 'when', label: 'When', render: (h) => new Date(h.at_unix * 1000).toLocaleString() },
            { key: 'space', label: 'Space', render: (h) => h.space },
            { key: 'dest', label: 'Destination', render: (h) => h.destination },
            { key: 'size', label: 'Size', render: (h) => size(h.bytes) },
            {
              key: 'db',
              label: 'Databases',
              render: (h) => (h.databases.length ? h.databases.join(', ') : '—'),
            },
          ]}
        />
      </section>
    </>
  );
}
