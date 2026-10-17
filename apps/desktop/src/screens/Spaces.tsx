import { useEffect, useState } from 'react';
import { call } from '../api/agent';
import type { SpaceDetail, SpaceMode, SpaceSpec, SpaceSummary } from '../contracts';
import { Badge } from '../components/ui';
import { DataTable } from '../components/DataTable';
import { Dialog } from '../components/Dialog';
import { useToast } from '../components/Toast';

const MODES: [SpaceMode, string, string][] = [
  ['isolated', 'Isolated (recommended)', 'Own versions, caches, data and ports'],
  ['global', 'Global', 'Uses tools installed on this PC'],
  ['container', 'Container', 'Runs the stack in Docker or Podman'],
];
const TOOL_CHOICES: Record<string, string[]> = {
  php: ['', '8.4', '8.3', '8.2', '8.1', '7.4'],
  node: ['', '24', '22', '20', '18'],
  python: ['', '3.13', '3.12', '3.11'],
  go: ['', '1.24', '1.23'],
};
const SERVICE_CHOICES: [string, string[]][] = [
  ['mysql', ['8.4', '8.0', '5.7']],
  ['mariadb', ['11.4', '10.11']],
  ['postgres', ['17', '16', '15']],
  ['redis', ['7']],
  ['mongodb', ['8.0', '7.0']],
  ['mailpit', ['1']],
];

function slug(path: string) {
  const last = path.split(/[\\/]/).filter(Boolean).pop() ?? '';
  return last
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, '-')
    .replace(/^[-.]+/, '')
    .slice(0, 64);
}

/** New space wizard: folder → environment type → versions → services (DVF-01). */
function NewSpace({ onClose, onCreated }: { onClose: () => void; onCreated: (d: SpaceDetail) => void }) {
  const [path, setPath] = useState('');
  const [name, setName] = useState('');
  const [mode, setMode] = useState<SpaceMode>('isolated');
  const [tools, setTools] = useState<Record<string, string>>({ php: '8.3', node: '22' });
  const [services, setServices] = useState<Record<string, string>>({ mysql: '8.4' });
  const [err, setErr] = useState('');
  const [busy, setBusy] = useState(false);
  const spaceName = name || slug(path);
  const domainName = spaceName.replace(/[^a-z0-9-]/g, '-').replace(/^-+|-+$/g, '');
  async function create() {
    setErr('');
    setBusy(true);
    const spec: SpaceSpec = {
      version: 1,
      name: spaceName,
      mode,
      domain: domainName ? `${domainName}.test` : undefined,
      tools: Object.fromEntries(Object.entries(tools).filter(([, v]) => v)),
      services: Object.entries(services)
        .filter(([, v]) => v)
        .map(([kind, version]) => ({ kind, version })),
      env: {},
    };
    try {
      onCreated(await call<SpaceDetail>('space.save', { path, spec }));
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <Dialog title="New space" onClose={onClose}>
      <label className="stacked">
        Project folder (inside a workspace root)
        <input value={path} onChange={(e) => setPath(e.target.value)} placeholder="C:\work\shop" />
      </label>
      <label className="stacked">
        Name
        <input value={name} onChange={(e) => setName(e.target.value)} placeholder={slug(path) || 'shop'} />
      </label>
      <h3 id="space-mode">Environment</h3>
      <div className="choiceGrid" role="radiogroup" aria-labelledby="space-mode">
        {MODES.map(([id, title, hint]) => (
          <button
            key={id}
            type="button"
            role="radio"
            aria-checked={mode === id}
            className="choice"
            onClick={() => setMode(id)}
          >
            {title}
            <small>{hint}</small>
          </button>
        ))}
      </div>
      <h3>Versions</h3>
      <div className="formGrid">
        {Object.entries(TOOL_CHOICES).map(([tool, versions]) => (
          <label key={tool}>
            {tool}
            <select
              value={tools[tool] ?? ''}
              onChange={(e) => setTools({ ...tools, [tool]: e.target.value })}
            >
              {versions.map((v) => (
                <option key={v} value={v}>
                  {v || 'Not used'}
                </option>
              ))}
            </select>
          </label>
        ))}
      </div>
      <h3>Services</h3>
      <div className="formGrid">
        {SERVICE_CHOICES.map(([kind, versions]) => (
          <label key={kind}>
            {kind}
            <select
              value={services[kind] ?? ''}
              onChange={(e) => setServices({ ...services, [kind]: e.target.value })}
            >
              <option value="">Not used</option>
              {versions.map((v) => (
                <option key={v} value={v}>
                  {v}
                </option>
              ))}
            </select>
          </label>
        ))}
      </div>
      <p className="hint">
        {domainName ? (
          <>
            Domain: <code>{domainName}.test</code> ·{' '}
          </>
        ) : null}
        Ports are assigned automatically so spaces never clash. Project files are not changed; VSN only adds a{' '}
        <code>.devify/</code> folder.
      </p>
      {err && (
        <div className="error" role="alert">
          {err}
        </div>
      )}
      <div className="buttonRow">
        <button className="ghost" onClick={onClose}>
          Cancel
        </button>
        <button className="primary" onClick={() => void create()} disabled={!path || !spaceName || busy}>
          {busy ? 'Creating…' : 'Create space'}
        </button>
      </div>
    </Dialog>
  );
}

function SpaceDetailPanel({ root }: { root: string }) {
  const [detail, setDetail] = useState<SpaceDetail | null>(null);
  const [err, setErr] = useState('');
  useEffect(() => {
    setDetail(null);
    call<SpaceDetail>('space.get', { path: root })
      .then(setDetail)
      .catch((e) => setErr(String(e)));
  }, [root]);
  if (err)
    return (
      <div className="error" role="alert">
        {err}
      </div>
    );
  if (!detail) return <p className="hint">Loading…</p>;
  const { spec, env } = detail;
  return (
    <section className="panel" aria-labelledby="space-detail">
      <div className="panelHead">
        <div>
          <span className="kicker">{spec.mode} space</span>
          <h3 id="space-detail">{spec.name}</h3>
        </div>
        <Badge
          ok={env.missing.length === 0}
          label={env.missing.length ? `${env.missing.length} to install` : 'Ready'}
        />
      </div>
      <dl className="facts kvList">
        <div>
          <dt>Folder</dt>
          <dd>
            <code>{detail.root}</code>
          </dd>
        </div>
        <div>
          <dt>Domain</dt>
          <dd>{spec.domain ? <code>{spec.domain}</code> : '—'}</dd>
        </div>
        <div>
          <dt>Versions</dt>
          <dd>
            {Object.entries(spec.tools)
              .map(([t, v]) => `${t} ${v}`)
              .join(' · ') || '—'}
          </dd>
        </div>
        <div>
          <dt>Services</dt>
          <dd>{spec.services.map((s) => `${s.kind} ${s.version} :${s.port ?? '?'}`).join(' · ') || '—'}</dd>
        </div>
      </dl>
      {env.missing.length > 0 && (
        <p className="hint">
          Not installed yet: {env.missing.map((m) => `${m.tool} ${m.version}`).join(', ')}. Install them from
          Runtimes; automatic download arrives with the runtime catalog update.
        </p>
      )}
    </section>
  );
}

export function Spaces() {
  const [rows, setRows] = useState<SpaceSummary[]>([]);
  const [err, setErr] = useState('');
  const [open, setOpen] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const toast = useToast();
  async function load() {
    try {
      setRows(await call<SpaceSummary[]>('space.list'));
      setErr('');
    } catch (e) {
      setErr(String(e));
    }
  }
  useEffect(() => {
    void load();
  }, []);
  async function forget(root: string) {
    try {
      await call('space.forget', { path: root });
      toast('Space removed from VSN (files kept)');
      if (selected === root) setSelected(null);
      await load();
    } catch (e) {
      toast(String(e), 'danger');
    }
  }
  return (
    <>
      <section className="panel" aria-labelledby="spaces-title">
        <div className="panelHead">
          <div>
            <span className="kicker">Isolated environments</span>
            <h3 id="spaces-title">Spaces</h3>
          </div>
          <button className="primary" onClick={() => setOpen(true)}>
            New space
          </button>
        </div>
        <DataTable
          caption="Spaces"
          rows={rows}
          rowKey={(r) => r.root}
          empty={err || 'No spaces yet. Create one to give a project its own versions, database and domain.'}
          columns={[
            { key: 'name', label: 'Name', render: (r) => <b>{r.spec?.name ?? '—'}</b> },
            {
              key: 'mode',
              label: 'Type',
              render: (r) => (r.spec ? <Badge ok={r.spec.mode === 'isolated'} label={r.spec.mode} /> : '—'),
            },
            {
              key: 'domain',
              label: 'Domain',
              className: 'hideSm',
              render: (r) => (r.spec?.domain ? <code>{r.spec.domain}</code> : '—'),
            },
            {
              key: 'tools',
              label: 'Versions',
              className: 'hideSm',
              render: (r) =>
                r.error ? (
                  <span className="hint">{r.error}</span>
                ) : (
                  Object.entries(r.spec?.tools ?? {})
                    .map(([t, v]) => `${t} ${v}`)
                    .join(' · ') || '—'
                ),
            },
            {
              key: 'act',
              label: 'Actions',
              className: 'actions',
              render: (r) => (
                <>
                  <button
                    className="ghost"
                    onClick={() => setSelected(r.root)}
                    aria-label={`Open ${r.spec?.name ?? r.root}`}
                  >
                    Open
                  </button>{' '}
                  <button
                    className="ghost"
                    onClick={() => void forget(r.root)}
                    aria-label={`Remove ${r.spec?.name ?? r.root}`}
                  >
                    Remove
                  </button>
                </>
              ),
            },
          ]}
        />
      </section>
      {selected && <SpaceDetailPanel root={selected} />}
      {open && (
        <NewSpace
          onClose={() => setOpen(false)}
          onCreated={(d) => {
            setOpen(false);
            toast(`Space ${d.spec.name} created`);
            setSelected(d.root);
            void load();
          }}
        />
      )}
    </>
  );
}
