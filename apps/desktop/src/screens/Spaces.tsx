import { useEffect, useState } from 'react';
import { call, openUrl, routeDomain } from '../api/agent';
import type { SpaceDetail, SpaceMode, SpaceSpec, SpaceSummary } from '../contracts';
import { Badge } from '../components/ui';
import { readPrefs, spaceUrl } from '../theme/prefs';
import { DataTable } from '../components/DataTable';
import { Dialog } from '../components/Dialog';
import { SpaceConfig } from '../components/SpaceConfig';
import { useToast } from '../components/Toast';

const MODES: [SpaceMode, string, string][] = [
  ['isolated', 'Isolated (recommended)', 'Own versions, caches, data and ports'],
  ['global', 'Global', 'Uses tools installed on this PC'],
  ['container', 'Container', 'Runs the stack in Docker or Podman'],
];
const TOOL_CHOICES: Record<string, string[]> = {
  php: ['', '8.5', '8.4', '8.3', '8.2', '8.1', '8.0', '7.4', '7.3', '7.2', '5.6'],
  node: ['', '24', '22', '20', '18', '16', '14'],
  python: ['', '3.13', '3.12', '3.11', '3.10'],
  go: ['', '1.24', '1.23'],
  ruby: ['', '3.4', '3.3', '3.2'],
  java: ['', '21', '17', '11'],
  dotnet: ['', '9', '8'],
  rust: ['', 'stable'],
  bun: ['', '1'],
  deno: ['', '2'],
};
const SERVICE_CHOICES: [string, string[]][] = [
  ['mysql', ['8.4', '8.0', '5.7']],
  ['mariadb', ['11.4', '10.11']],
  ['postgres', ['17', '16', '15']],
  ['redis', ['7']],
  ['valkey', ['8']],
  ['memcached', ['1.6']],
  ['varnish', ['7']],
  ['mongodb', ['8.0', '7.0']],
  ['meilisearch', ['1']],
  ['minio', ['latest']],
  ['ftp', ['1']],
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
  const [catalog, setCatalog] = useState('');
  const [trust, setTrust] = useState('');
  const [command, setCommand] = useState('');
  const [runId, setRunId] = useState('web');
  const [appPort, setAppPort] = useState(3000);
  const [routes, setRoutes] = useState<Record<string, number>>({});
  useEffect(() => {
    call<Record<string, number>>('domain.routes')
      .then(setRoutes)
      .catch(() => setRoutes({}));
  }, [root]);
  async function route(domain: string) {
    try {
      const code = await routeDomain(domain, appPort);
      if (code !== 0)
        throw new Error(`setup did not finish (code ${code}); approve the Windows prompt to continue`);
      setRoutes(await call<Record<string, number>>('domain.routes'));
      toast(`${domain} now opens your app on port ${appPort}`);
    } catch (e) {
      toast(String(e), 'danger');
    }
  }
  const [portEdit, setPortEdit] = useState<{ kind: string; value: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const toast = useToast();
  const load = () =>
    call<SpaceDetail>('space.get', { path: root })
      .then(setDetail)
      .catch((e) => setErr(String(e)));
  useEffect(() => {
    setDetail(null);
    void load();
  }, [root]);
  async function installMissing() {
    setBusy(true);
    try {
      const r = await call<{ installed: unknown[]; unavailable: { tool: string; version: string }[] }>(
        'space.install',
        {
          path: root,
          catalog,
          trust,
        },
      );
      toast(
        r.unavailable.length
          ? `Installed ${r.installed.length}; not in catalog: ${r.unavailable.map((u) => `${u.tool} ${u.version}`).join(', ')}`
          : `Installed ${r.installed.length} version(s)`,
        r.unavailable.length ? 'danger' : 'info',
      );
      await load();
    } catch (e) {
      toast(String(e), 'danger');
    } finally {
      setBusy(false);
    }
  }
  async function service(kind: string, action: 'start' | 'stop') {
    setBusy(true);
    try {
      await call(action === 'start' ? 'space.service.start' : 'space.service.stop', { path: root, kind });
      toast(`${kind} ${action === 'start' ? 'started' : 'stopped'}`);
    } catch (e) {
      toast(String(e), 'danger');
    } finally {
      setBusy(false);
    }
  }
  async function savePort() {
    if (!detail || !portEdit) return;
    const port = Number(portEdit.value);
    if (!Number.isInteger(port) || port < 1024 || port > 65535) {
      toast('Choose a port between 1024 and 65535', 'danger');
      return;
    }
    const spec = {
      ...detail.spec,
      services: detail.spec.services.map((sv) => (sv.kind === portEdit.kind ? { ...sv, port } : sv)),
    };
    try {
      setDetail(await call<SpaceDetail>('space.save', { path: detail.root, spec }));
      toast(`${portEdit.kind} now uses port ${port}. Restart it to apply.`);
      setPortEdit(null);
    } catch (e) {
      toast(String(e), 'danger');
    }
  }
  async function run() {
    const [program, ...args] = command.trim().split(/\s+/);
    if (!program) return;
    setBusy(true);
    try {
      const st = await call<{ id: string; pid: number }>('space.process.start', {
        path: root,
        id: runId,
        program,
        args,
      });
      toast(`Started ${st.id} (PID ${st.pid}); see Processes → Managed processes`);
    } catch (e) {
      toast(String(e), 'danger');
    } finally {
      setBusy(false);
    }
  }
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
          <dd>
            {spec.domain ? <code>{spec.domain}</code> : '—'}
            {spec.domain && routes[spec.domain] && <> → 127.0.0.1:{routes[spec.domain]}</>}{' '}
            {spec.domain?.endsWith('.test') && (
              <button className="ghost" onClick={() => void route(spec.domain!)}>
                Route to port {appPort}
              </button>
            )}
          </dd>
        </div>
        <div>
          <dt>Local address</dt>
          <dd>
            <code>{spaceUrl(readPrefs().urlPattern, spec.name, appPort, spec.domain)}</code>{' '}
            <label className="srOnly" htmlFor="space-app-port">
              App port
            </label>
            <input
              id="space-app-port"
              className="portInput"
              type="number"
              min={1}
              max={65535}
              value={appPort}
              onChange={(e) => setAppPort(Number(e.target.value))}
            />{' '}
            <button
              className="ghost"
              onClick={() =>
                void openUrl(spaceUrl(readPrefs().urlPattern, spec.name, appPort, spec.domain)).catch((e) =>
                  toast(String(e), 'danger'),
                )
              }
            >
              Open
            </button>
          </dd>
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
        <>
          <h4>Install missing versions</h4>
          <p className="hint">
            Not installed yet: {env.missing.map((m) => `${m.tool} ${m.version}`).join(', ')}. VSN picks the
            newest matching release from a signed runtime catalog and verifies it before installing.
          </p>
          <div className="formGrid">
            <label>
              Catalog file
              <input
                value={catalog}
                onChange={(e) => setCatalog(e.target.value)}
                placeholder="C:\catalogs\runtimes.json"
              />
            </label>
            <label>
              Trust file
              <input
                value={trust}
                onChange={(e) => setTrust(e.target.value)}
                placeholder="C:\catalogs\trust.json"
              />
            </label>
          </div>
          <div className="buttonRow">
            <button
              className="primary"
              disabled={!catalog || !trust || busy}
              onClick={() => void installMissing()}
            >
              Install {env.missing.length} missing
            </button>
          </div>
        </>
      )}
      {spec.services.length > 0 && (
        <>
          <h4>Services</h4>
          <DataTable
            caption={`${spec.name} services`}
            rows={spec.services}
            rowKey={(sv) => sv.kind}
            empty="No services"
            columns={[
              { key: 'kind', label: 'Service', render: (sv) => <b>{sv.kind}</b> },
              { key: 'version', label: 'Version', render: (sv) => sv.version },
              {
                key: 'port',
                label: 'Port',
                className: 'num',
                render: (sv) =>
                  portEdit?.kind === sv.kind ? (
                    <input
                      className="portInput"
                      type="number"
                      min={1024}
                      max={65535}
                      aria-label={`New port for ${sv.kind}`}
                      value={portEdit.value}
                      onChange={(e) => setPortEdit({ kind: sv.kind, value: e.target.value })}
                      onKeyDown={(e) => e.key === 'Enter' && void savePort()}
                      autoFocus
                    />
                  ) : (
                    <code>{sv.port ?? '—'}</code>
                  ),
              },
              {
                key: 'act',
                label: 'Actions',
                className: 'actions',
                render: (sv) => (
                  <>
                    <button
                      className="ghost"
                      disabled={busy}
                      onClick={() => void service(sv.kind, 'start')}
                      aria-label={`Start ${sv.kind}`}
                    >
                      Start
                    </button>{' '}
                    <button
                      className="ghost"
                      disabled={busy}
                      onClick={() => void service(sv.kind, 'stop')}
                      aria-label={`Stop ${sv.kind}`}
                    >
                      Stop
                    </button>{' '}
                    {portEdit?.kind === sv.kind ? (
                      <>
                        <button className="ghost" disabled={busy} onClick={() => void savePort()}>
                          Save port
                        </button>{' '}
                        <button className="ghost" onClick={() => setPortEdit(null)}>
                          Cancel
                        </button>
                      </>
                    ) : (
                      <button
                        className="ghost"
                        onClick={() => setPortEdit({ kind: sv.kind, value: String(sv.port ?? '') })}
                        aria-label={`Change ${sv.kind} port`}
                      >
                        Change port
                      </button>
                    )}
                  </>
                ),
              },
            ]}
          />
          <p className="hint">
            Each service listens on 127.0.0.1 only and keeps its data in this space's .devify/data folder.
          </p>
        </>
      )}
      <SpaceConfig root={detail.root} />
      <h4>Run in this space</h4>
      <p className="hint">
        Runs with this space's versions first on PATH, its caches and env vars, in the space folder.
      </p>
      <div className="formGrid">
        <label>
          Command
          <input
            value={command}
            onChange={(e) => setCommand(e.target.value)}
            placeholder="php -S 127.0.0.1:8000 -t public"
          />
        </label>
        <label>
          Name
          <input value={runId} onChange={(e) => setRunId(e.target.value.replace(/[^A-Za-z0-9_-]/g, ''))} />
        </label>
      </div>
      <div className="buttonRow">
        <button
          className="primary"
          disabled={!command.trim() || !runId || busy || env.missing.length > 0}
          onClick={() => void run()}
        >
          Start
        </button>
      </div>
    </section>
  );
}

type LockMode = 'unlock' | 'set' | 'change' | 'clear';
const LOCK_TITLES: Record<LockMode, string> = {
  unlock: 'Unlock space',
  set: 'Lock space with a password',
  change: 'Change space password',
  clear: 'Remove space lock',
};

/** Space lock: one dialog for unlock, set, change and remove. */
function LockDialog({
  space,
  mode,
  onClose,
  onDone,
}: {
  space: SpaceSummary;
  mode: LockMode;
  onClose: () => void;
  onDone: (message: string) => void;
}) {
  const [current, setCurrent] = useState('');
  const [next, setNext] = useState('');
  const [confirm, setConfirm] = useState('');
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');
  const needsNew = mode === 'set' || mode === 'change';
  const needsCurrent = mode !== 'set';
  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (needsNew && next !== confirm) return setErr('The two passwords do not match.');
    setBusy(true);
    setErr('');
    try {
      const path = space.root;
      if (mode === 'unlock') await call('space.unlock', { path, password: current });
      if (mode === 'clear') await call('space.lock.clear', { path, password: current });
      if (needsNew)
        await call('space.lock.set', { path, password: next, ...(mode === 'change' ? { current } : {}) });
      onDone(
        mode === 'unlock'
          ? 'Space unlocked for 8 hours'
          : mode === 'clear'
            ? 'Space lock removed'
            : 'Space locked',
      );
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <Dialog title={`${LOCK_TITLES[mode]}: ${space.spec?.name ?? ''}`} onClose={onClose}>
      <form onSubmit={(e) => void submit(e)}>
        {needsCurrent && (
          <label className="stacked">
            {mode === 'unlock' ? 'Password' : 'Current password'}
            <input
              type="password"
              autoComplete="current-password"
              value={current}
              onChange={(e) => setCurrent(e.target.value)}
              autoFocus
            />
          </label>
        )}
        {needsNew && (
          <>
            <label className="stacked">
              New password (12+ characters)
              <input
                type="password"
                autoComplete="new-password"
                minLength={12}
                value={next}
                onChange={(e) => setNext(e.target.value)}
                autoFocus={!needsCurrent}
              />
            </label>
            <label className="stacked">
              Repeat new password
              <input
                type="password"
                autoComplete="new-password"
                value={confirm}
                onChange={(e) => setConfirm(e.target.value)}
              />
            </label>
            <p className="hint">
              While locked, nobody can start, stop, change or remove this space without the password. The
              password is stored only as an Argon2 hash in VSN's data folder, never in the project.
            </p>
          </>
        )}
        {err && <div className="error">{err}</div>}
        <div className="buttonRow">
          <button type="submit" className="primary" disabled={busy}>
            {busy ? 'Working…' : LOCK_TITLES[mode]}
          </button>
          <button type="button" className="ghost" onClick={onClose}>
            Cancel
          </button>
        </div>
      </form>
    </Dialog>
  );
}

export function Spaces() {
  const [rows, setRows] = useState<SpaceSummary[]>([]);
  const [err, setErr] = useState('');
  const [open, setOpen] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [lock, setLock] = useState<{ space: SpaceSummary; mode: LockMode } | null>(null);
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
            {
              key: 'name',
              label: 'Name',
              render: (r) => (
                <>
                  <b>{r.spec?.name ?? '—'}</b>{' '}
                  {r.locked && <Badge ok={!!r.unlocked} label={r.unlocked ? 'Unlocked' : 'Locked'} />}
                </>
              ),
            },
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
                  </button>{' '}
                  {r.locked && !r.unlocked && (
                    <button className="ghost" onClick={() => setLock({ space: r, mode: 'unlock' })}>
                      Unlock
                    </button>
                  )}
                  {r.locked && r.unlocked && (
                    <>
                      <button
                        className="ghost"
                        onClick={() =>
                          void call('space.relock', { path: r.root })
                            .then(load)
                            .catch((e) => toast(String(e), 'danger'))
                        }
                      >
                        Lock now
                      </button>{' '}
                      <button className="ghost" onClick={() => setLock({ space: r, mode: 'change' })}>
                        Change password
                      </button>{' '}
                      <button className="ghost" onClick={() => setLock({ space: r, mode: 'clear' })}>
                        Remove lock
                      </button>
                    </>
                  )}
                  {!r.locked && r.spec && (
                    <button className="ghost" onClick={() => setLock({ space: r, mode: 'set' })}>
                      Lock
                    </button>
                  )}
                </>
              ),
            },
          ]}
        />
      </section>
      {selected && <SpaceDetailPanel root={selected} />}
      {lock && (
        <LockDialog
          space={lock.space}
          mode={lock.mode}
          onClose={() => setLock(null)}
          onDone={(message) => {
            setLock(null);
            toast(message);
            void load();
          }}
        />
      )}
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
