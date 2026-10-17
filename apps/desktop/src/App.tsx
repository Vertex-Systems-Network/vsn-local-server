import { useCallback, useEffect, useMemo, useState, type ChangeEvent } from 'react';
import { call } from './api/agent';
import type {
  ContainerBackend,
  MachineIdentity,
  ProcessInfo,
  RemoteConfig,
  RuntimeDetection,
} from './contracts';
import {
  connectionLabel,
  initialOverviewState,
  offlineOverviewState,
  onlineOverviewState,
  type AgentStatus,
} from './agentOverviewState';
import { Badge, JsonView } from './components/ui';
import { Overview } from './screens/Overview';
import { Projects } from './screens/Projects';
import { Services } from './screens/Services';
import { Runtimes } from './screens/Runtimes';
import { Processes } from './screens/Processes';
import { Containers } from './screens/Containers';
import { DatabaseStudio } from './screens/DatabaseStudio';
import { DatabaseAdvanced } from './screens/DatabaseAdvanced';
import { FilesPanel } from './screens/FilesPanel';
import { TerminalPanel } from './screens/TerminalPanel';
import { Networking } from './screens/Networking';
import { RemotePanel } from './screens/RemotePanel';
import { ContainerRegistry } from './screens/ContainerRegistry';
import { DatabaseJobs } from './screens/DatabaseJobs';
import { NativePostgresJobs } from './screens/NativePostgresJobs';

type Section =
  | 'overview'
  | 'projects'
  | 'services'
  | 'runtimes'
  | 'processes'
  | 'containers'
  | 'database'
  | 'files'
  | 'terminal'
  | 'network'
  | 'remote';
const DESKTOP_CAPABILITIES = [
  'projects',
  'services',
  'runtimes',
  'processes',
  'containers',
  'database',
  'advanced_db_models',
  'database_jobs',
  'files',
  'terminal',
  'network',
  'remote',
  'core_health',
] as const;
const nav: [Section, string, string][] = [
  ['overview', 'Overview', '⌂'],
  ['projects', 'Projects', '◇'],
  ['services', 'Services', '◉'],
  ['runtimes', 'Runtimes', '⌘'],
  ['processes', 'Processes', '◫'],
  ['containers', 'Containers', '⬡'],
  ['database', 'Database Studio', '▦'],
  ['files', 'Files', '▤'],
  ['terminal', 'Terminal', '>_'],
  ['network', 'Networking', '◎'],
  ['remote', 'Remote', '↗'],
];
export default function App() {
  const [section, setSection] = useState<Section>('overview');
  const [overview, setOverview] = useState(initialOverviewState);
  const { connection, machine, status, runtimes, processes, backends, remote, coreError, optionalErrors } =
    overview;
  const [busy, setBusy] = useState(false);
  const refresh = useCallback(async () => {
    setBusy(true);
    try {
      const [m, s] = await Promise.all([call<MachineIdentity>('machine'), call<AgentStatus>('status')]);
      const [r, p, c, rem] = await Promise.allSettled([
        call<RuntimeDetection[]>('runtime.list'),
        call<ProcessInfo[]>('process.list'),
        call<ContainerBackend[]>('container.backends'),
        call<RemoteConfig>('remote.status'),
      ]);
      setOverview(onlineOverviewState(m, s, { runtimes: r, processes: p, backends: c, remote: rem }));
    } catch (e) {
      setOverview(offlineOverviewState(e));
    } finally {
      setBusy(false);
    }
  }, []);
  useEffect(() => {
    void refresh();
  }, [refresh]);
  const installed = useMemo(() => runtimes.filter((r) => r.installed).length, [runtimes]);
  return (
    <div className="shell">
      <aside>
        <div className="brand">
          <div className="mark">V</div>
          <div>
            <b>VSN</b>
            <span>Dev Platform</span>
          </div>
        </div>
        <nav>
          {nav.map(([id, label, icon]) => (
            <button key={id} className={section === id ? 'active' : ''} onClick={() => setSection(id)}>
              <span>{icon}</span>
              {label}
            </button>
          ))}
        </nav>
        <div className="asideFoot">
          <Badge
            ok={connection === 'online' && !!status?.health?.healthy}
            label={connectionLabel(connection)}
          />
          <small>Local-first · v0.38.1</small>
        </div>
      </aside>
      <main>
        <header>
          <div>
            <p className="eyebrow">LOCAL + REMOTE DEVELOPMENT CONTROL PLANE</p>
            <h1>{nav.find((n) => n[0] === section)?.[1]}</h1>
          </div>
          <div className="headerActions">
            <div className="machineChip">
              <span
                className={
                  connection === 'online'
                    ? 'dot'
                    : connection === 'offline'
                      ? 'dot offline'
                      : 'dot connecting'
                }
              />
              <div>
                <b>
                  {connection === 'offline' ? 'Agent unavailable' : (machine?.display_name ?? 'Connecting…')}
                </b>
                <small>
                  {connection === 'offline' ? 'Authenticated bridge offline' : (machine?.os ?? '—')}
                </small>
              </div>
            </div>
            <button className="primary" disabled={busy} onClick={() => void refresh()}>
              {busy ? 'Refreshing…' : 'Refresh'}
            </button>
          </div>
        </header>
        {coreError && <div className="error">{coreError}</div>}
        {optionalErrors.length > 0 && (
          <div className="warning">Partial refresh: {optionalErrors.join(' · ')}</div>
        )}
        <div className="content">
          <section className="panel">
            <div className="panelHead">
              <div>
                <span className="kicker">Desktop source coverage</span>
                <h2>{DESKTOP_CAPABILITIES.length} operator capabilities</h2>
              </div>
              <Badge ok label="Explicit coverage" />
            </div>
            <p className="hint">
              Projects, services, runtimes, process/container management, relational and advanced databases,
              jobs, files, terminal, networking and remote administration are explicitly surfaced. Native
              installer/signing acceptance is tracked under P30.
            </p>
          </section>
          {section === 'overview' && (
            <Overview
              machine={machine}
              status={status}
              installed={installed}
              processes={processes.length}
              backends={backends}
              remote={remote}
            />
          )}
          {section === 'projects' && <Projects />}
          {section === 'services' && <Services />}
          {section === 'runtimes' && <Runtimes rows={runtimes} />}{' '}
          {section === 'processes' && <Processes rows={processes} />}{' '}
          {section === 'containers' && (
            <>
              <Containers rows={backends} />
              <ContainerRegistry />
            </>
          )}{' '}
          {section === 'database' && (
            <>
              <DatabaseStudio />
              <DatabaseAdvanced />
              <AdvancedModelAnalyzer />
              <DatabaseJobs />
              <NativePostgresJobs />
            </>
          )}
          {section === 'files' && <FilesPanel />}
          {section === 'terminal' && <TerminalPanel />}
          {section === 'network' && <Networking />}
          {section === 'remote' && <RemotePanel config={remote} />}
        </div>
      </main>
    </div>
  );
}

function AdvancedModelAnalyzer() {
  const [request, setRequest] = useState(
    '{"model":"document","documents":[{"id":1,"name":"Ada"},{"id":2,"name":null}]}',
  );
  const [result, setResult] = useState<any>(null);
  const [err, setErr] = useState('');
  async function analyze() {
    setErr('');
    try {
      setResult(await call('database.model.analyze', JSON.parse(request)));
    } catch (e) {
      setErr(String(e));
    }
  }
  return (
    <section className="panel">
      <div className="panelHead">
        <div>
          <span className="kicker">Advanced data models</span>
          <h3>Document / KV / Vector / Graph / Search / Time-series / Column analyzer</h3>
        </div>
        <Badge ok label="Bounded local analysis" />
      </div>
      <label className="stacked">
        Model sample JSON
        <textarea
          value={request}
          onChange={(e: ChangeEvent<HTMLTextAreaElement>) => setRequest(e.target.value)}
          rows={8}
        />
      </label>
      <button className="primary" onClick={() => void analyze()}>
        Analyze model sample
      </button>
      <p className="hint">
        Document, key-value, vector, graph, search, time-series and wide-column samples are normalized through
        one bounded schema-driven analyzer with type/distribution/integrity warnings.
      </p>
      {result && <JsonView value={result} />} {err && <div className="error">{err}</div>}
    </section>
  );
}
