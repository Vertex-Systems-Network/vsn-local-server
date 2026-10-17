import { useCallback, useEffect, useMemo, useRef, useState, type ChangeEvent } from 'react';
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
import { Icon, type IconName } from './components/Icon';
import { Logo } from './components/Logo';
import { enterContent, enterDrawer, settleSidebar } from './motion';
import { CommandPalette, type Command } from './components/CommandPalette';
import { useToast } from './components/Toast';
import { Tabs } from './components/Tabs';
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
const groups: [string, [Section, string, IconName][]][] = [
  ['Home', [['overview', 'Overview', 'home']]],
  [
    'Build',
    [
      ['projects', 'Projects', 'folder'],
      ['runtimes', 'Runtimes', 'layers'],
      ['services', 'Services', 'server'],
    ],
  ],
  ['Data', [['database', 'Database Studio', 'database']]],
  [
    'Operate',
    [
      ['processes', 'Processes', 'activity'],
      ['containers', 'Containers', 'box'],
      ['terminal', 'Terminal', 'terminal'],
      ['files', 'Files', 'file'],
    ],
  ],
  [
    'Connect',
    [
      ['network', 'Networking', 'globe'],
      ['remote', 'Remote', 'remote'],
    ],
  ],
];
const nav = groups.flatMap(([group, items]) =>
  items.map(([id, label, icon]) => ({ id, label, icon, group })),
);
const DB_TABS = [
  ['studio', 'Browser & query'],
  ['advanced', 'External databases'],
  ['models', 'Model analyzer'],
  ['jobs', 'Jobs'],
] as const;
type DbTab = (typeof DB_TABS)[number][0];
const COLLAPSE_KEY = 'vsn.sidebar.collapsed';
function readCollapsed(): boolean {
  try {
    const saved = localStorage.getItem(COLLAPSE_KEY);
    if (saved !== null) return saved === '1';
  } catch {
    // Storage may be unavailable; fall back to the viewport default.
  }
  // Design: laptops and small desktops (< 1280px) start with the collapsed rail.
  return window.innerWidth < 1280;
}
export default function App() {
  const [section, setSection] = useState<Section>('overview');
  const [collapsed, setCollapsed] = useState(readCollapsed);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [dbTab, setDbTab] = useState<DbTab>('studio');
  const [mobile, setMobile] = useState(() => window.matchMedia('(max-width: 768px)').matches);
  useEffect(() => {
    const mq = window.matchMedia('(max-width: 768px)');
    const onChange = () => setMobile(mq.matches);
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  }, []);
  const rail = collapsed && !mobile;
  const contentRef = useRef<HTMLDivElement>(null);
  const sidebarRef = useRef<HTMLElement>(null);
  useEffect(() => enterContent(contentRef.current), [section]);
  useEffect(() => {
    if (drawerOpen) enterDrawer(sidebarRef.current);
  }, [drawerOpen]);
  const toggleCollapsed = () => {
    setCollapsed((c) => {
      try {
        localStorage.setItem(COLLAPSE_KEY, c ? '0' : '1');
      } catch {
        // Preference persistence is best-effort.
      }
      return !c;
    });
    requestAnimationFrame(() => settleSidebar(sidebarRef.current));
  };
  const go = (id: Section) => {
    setSection(id);
    setDrawerOpen(false);
  };
  const current = nav.find((n) => n.id === section);
  const [overview, setOverview] = useState(initialOverviewState);
  const { connection, machine, status, runtimes, processes, backends, remote, coreError, optionalErrors } =
    overview;
  const [busy, setBusy] = useState(false);
  const toast = useToast();
  const [paletteOpen, setPaletteOpen] = useState(false);
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
      toast('Agent status refreshed');
    } catch (e) {
      setOverview(offlineOverviewState(e));
      toast('Agent unavailable', 'danger');
    } finally {
      setBusy(false);
    }
  }, [toast]);
  useEffect(() => {
    void refresh();
  }, [refresh]);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        setPaletteOpen((o) => !o);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);
  const commands: Command[] = [
    ...nav.map((n) => ({
      id: `go-${n.id}`,
      label: n.label,
      hint: n.group,
      icon: n.icon,
      run: () => go(n.id),
    })),
    {
      id: 'refresh',
      label: 'Refresh agent status',
      hint: 'Agent',
      icon: 'refresh',
      run: () => void refresh(),
    },
    { id: 'sidebar', label: 'Toggle sidebar', hint: 'View', icon: 'chevron', run: toggleCollapsed },
  ];
  const installed = useMemo(() => runtimes.filter((r) => r.installed).length, [runtimes]);
  return (
    <>
      <div className="glow" aria-hidden="true">
        <i />
        <i />
        <i />
      </div>
      <div className={`shell${rail ? ' collapsed' : ''}${drawerOpen ? ' drawerOpen' : ''}`}>
        <aside className="sidebar" ref={sidebarRef} aria-label="Main navigation">
          <div className="brand">
            <Logo />
            <div className="name">
              <b>VSN</b>
              <small>Dev Platform</small>
            </div>
            <button
              className="iconButton collapseToggle"
              onClick={toggleCollapsed}
              aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
              aria-expanded={!collapsed}
            >
              <span style={{ display: 'grid', transform: collapsed ? 'none' : 'rotate(180deg)' }}>
                <Icon name="chevron" size={16} />
              </span>
            </button>
            <button
              className="iconButton mobileOnly"
              onClick={() => setDrawerOpen(false)}
              aria-label="Close menu"
            >
              <Icon name="x" size={16} />
            </button>
          </div>
          <nav aria-label="Sections">
            {groups.map(([group, items]) => (
              <div className="navGroup" key={group}>
                <span>{group}</span>
                {items.map(([id, label, icon]) => (
                  <button
                    key={id}
                    className={`navItem${section === id ? ' active' : ''}`}
                    onClick={() => go(id)}
                    data-tooltip={label}
                    aria-label={rail ? label : undefined}
                    aria-current={section === id ? 'page' : undefined}
                  >
                    <Icon name={icon} />
                    <span className="label">{label}</span>
                  </button>
                ))}
              </div>
            ))}
          </nav>
          <div className="agentFoot">
            <span
              className={
                connection === 'online' ? 'dot' : connection === 'offline' ? 'dot offline' : 'dot connecting'
              }
            />
            <div className="agentText">
              <Badge
                ok={connection === 'online' && !!status?.health?.healthy}
                label={connectionLabel(connection)}
              />
              <small>Local-first · v0.38.1</small>
            </div>
          </div>
        </aside>
        <button className="drawerBackdrop" aria-label="Close menu" onClick={() => setDrawerOpen(false)} />
        <main>
          <header className="topbar">
            <button
              className="iconButton mobileOnly"
              onClick={() => setDrawerOpen(true)}
              aria-label="Open menu"
            >
              <Icon name="menu" size={18} />
            </button>
            <span className="mobileOnly">
              <Logo size={28} />
            </span>
            <div className="titles">
              <p className="crumb">{current?.group}</p>
              <h1>{current?.label}</h1>
            </div>
            <div className="headerActions">
              <button
                className="paletteTrigger"
                onClick={() => setPaletteOpen(true)}
                aria-label="Open command palette"
              >
                <Icon name="search" size={16} />
                <span>Search or run a command…</span>
                <kbd>Ctrl K</kbd>
              </button>
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
                    {connection === 'offline'
                      ? 'Agent unavailable'
                      : (machine?.display_name ?? 'Connecting…')}
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
          <div className="content" ref={contentRef}>
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
                <Tabs
                  tabs={DB_TABS}
                  value={dbTab}
                  onChange={setDbTab}
                  label="Database Studio"
                  panelId="db-panel"
                />
                <div role="tabpanel" id="db-panel" aria-labelledby={`db-panel-tab-${dbTab}`}>
                  {dbTab === 'studio' && <DatabaseStudio />}
                  {dbTab === 'advanced' && <DatabaseAdvanced />}
                  {dbTab === 'models' && <AdvancedModelAnalyzer />}
                  {dbTab === 'jobs' && (
                    <>
                      <DatabaseJobs />
                      <NativePostgresJobs />
                    </>
                  )}
                </div>
              </>
            )}
            {section === 'files' && <FilesPanel />}
            {section === 'terminal' && <TerminalPanel />}
            {section === 'network' && <Networking />}
            {section === 'remote' && <RemotePanel config={remote} />}
          </div>
        </main>
      </div>
      {paletteOpen && <CommandPalette commands={commands} onClose={() => setPaletteOpen(false)} />}
    </>
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
