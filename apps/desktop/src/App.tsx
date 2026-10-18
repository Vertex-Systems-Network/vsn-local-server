import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
  type CSSProperties,
} from 'react';
import { call, startAgent } from './api/agent';
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
import { driftGlow, enterContent, enterDrawer, introApp, settleSidebar, swapTitle } from './motion';
import { CommandPalette, type Command } from './components/CommandPalette';
import { useToast } from './components/Toast';
import { Tabs } from './components/Tabs';
import { Overview } from './screens/Overview';
import { Spaces } from './screens/Spaces';
import { Mail } from './screens/Mail';
import { About } from './screens/About';
import { Backups } from './screens/Backups';
import { Help } from './screens/Help';
import { Assistant } from './screens/Assistant';
import { Deploy } from './screens/Deploy';
import { Analytics } from './screens/Analytics';
import { SourceControl } from './screens/SourceControl';
import { Splash } from './components/Splash';
import { DonatePrompt } from './components/DonatePrompt';
import { ApiClient } from './screens/ApiClient';
import { AgentOffline } from './components/AgentOffline';
import { Settings } from './screens/Settings';
import { Updates } from './screens/Updates';
import { SIDEBAR_EVENT } from './theme/prefs';
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
  | 'spaces'
  | 'mail'
  | 'api'
  | 'git'
  | 'analytics'
  | 'deploy'
  | 'backups'
  | 'help'
  | 'assistant'
  | 'projects'
  | 'services'
  | 'runtimes'
  | 'processes'
  | 'containers'
  | 'database'
  | 'files'
  | 'terminal'
  | 'network'
  | 'remote'
  | 'settings'
  | 'updates'
  | 'about';
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
      ['spaces', 'Spaces', 'spaces'],
      ['assistant', 'AI assistant', 'spark'],
      ['projects', 'Projects', 'folder'],
      ['git', 'Source control', 'git'],
      ['runtimes', 'Runtimes', 'layers'],
      ['services', 'Services', 'server'],
    ],
  ],
  ['Data', [['database', 'Database Studio', 'database']]],
  [
    'Test',
    [
      ['mail', 'Mail', 'mail'],
      ['api', 'API client', 'api'],
    ],
  ],
  [
    'Operate',
    [
      ['analytics', 'Analytics', 'chart'],
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
      ['deploy', 'Deploy', 'upload'],
      ['backups', 'Backups', 'download'],
      ['remote', 'Remote', 'remote'],
    ],
  ],
  [
    'System',
    [
      ['settings', 'Settings', 'settings'],
      ['updates', 'Updates', 'download'],
      ['help', 'Help', 'file'],
      ['about', 'About', 'info'],
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
const WIDTH_KEY = 'vsn.sidebar.width';
const MIN_W = 180;
const MAX_W = 380;
function readWidth(): number {
  try {
    const w = Number(localStorage.getItem(WIDTH_KEY));
    if (w >= MIN_W && w <= MAX_W) return w;
  } catch {
    // Storage may be unavailable.
  }
  return 232;
}
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
  const [sideW, setSideW] = useState(readWidth);
  const [booted, setBooted] = useState(false);
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
  const glowRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const onSidebar = (e: Event) => {
      const next = (e as CustomEvent<boolean>).detail;
      setCollapsed(next);
      try {
        localStorage.setItem(COLLAPSE_KEY, next ? '1' : '0');
      } catch {
        // Preference persistence is best-effort.
      }
      requestAnimationFrame(() => settleSidebar(sidebarRef.current));
    };
    window.addEventListener(SIDEBAR_EVENT, onSidebar);
    return () => window.removeEventListener(SIDEBAR_EVENT, onSidebar);
  }, []);
  useEffect(() => {
    driftGlow(glowRef.current);
    return () => driftGlow(null);
  }, []);
  const shellRef = useRef<HTMLDivElement>(null);
  const titleRef = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    if (booted) introApp(shellRef.current);
  }, [booted]);
  useEffect(() => {
    enterContent(contentRef.current);
    swapTitle(titleRef.current);
  }, [section]);
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
  function resizeTo(w: number) {
    const next = Math.round(Math.min(MAX_W, Math.max(MIN_W, w)));
    setSideW(next);
    try {
      localStorage.setItem(WIDTH_KEY, String(next));
    } catch {
      // Preference persistence is best-effort.
    }
  }
  const go = (id: Section) => {
    setSection(id);
    setDrawerOpen(false);
  };
  const current = nav.find((n) => n.id === section);
  const [overview, setOverview] = useState(initialOverviewState);
  const { connection, machine, status, runtimes, processes, backends, remote, coreError, optionalErrors } =
    overview;
  const [busy, setBusy] = useState(false);
  const [lastSeen, setLastSeen] = useState<Date | null>(null);
  const toast = useToast();
  const [paletteOpen, setPaletteOpen] = useState(false);
  const refresh = useCallback(
    async (autoStart = false) => {
      setBusy(true);
      try {
        const load = () => Promise.all([call<MachineIdentity>('machine'), call<AgentStatus>('status')]);
        let core;
        try {
          core = await load();
        } catch (first) {
          // The agent may simply not be running yet: start the bundled one, then try once more.
          if (!autoStart || !(await startAgent().catch(() => false))) throw first;
          core = await load();
        }
        const [m, s] = core;
        const [r, p, c, rem] = await Promise.allSettled([
          call<RuntimeDetection[]>('runtime.list'),
          call<ProcessInfo[]>('process.list'),
          call<ContainerBackend[]>('container.backends'),
          call<RemoteConfig>('remote.status'),
        ]);
        setOverview(onlineOverviewState(m, s, { runtimes: r, processes: p, backends: c, remote: rem }));
        setLastSeen(new Date());
        toast('Agent status refreshed');
      } catch (e) {
        setOverview(offlineOverviewState(e));
        toast('Agent unavailable', 'danger');
      } finally {
        setBusy(false);
      }
    },
    [toast],
  );
  useEffect(() => {
    void refresh(true);
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
    { id: 'sidebar', label: 'Toggle sidebar', hint: 'View', icon: 'sidebar', run: toggleCollapsed },
  ];
  const installed = useMemo(() => runtimes.filter((r) => r.installed).length, [runtimes]);
  return (
    <>
      {!booted && <Splash ready={connection !== 'connecting'} onDone={() => setBooted(true)} />}
      {booted && <DonatePrompt />}
      <div className="glow" aria-hidden="true" ref={glowRef}>
        <i />
        <i />
        <i />
      </div>
      <div
        ref={shellRef}
        className={`shell${rail ? ' collapsed' : ''}${drawerOpen ? ' drawerOpen' : ''}`}
        style={{ '--sidebar-w': `${sideW}px` } as CSSProperties}
      >
        <aside className="sidebar" ref={sidebarRef} aria-label="Main navigation">
          <div className="brand">
            <Logo />
            <div className="name">
              <b>VSN</b>
              <small>Devify</small>
            </div>
            <button
              className="iconButton collapseToggle"
              onClick={toggleCollapsed}
              aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
              aria-expanded={!collapsed}
            >
              <Icon name="sidebar" size={18} />
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
        {!rail && !mobile && (
          <div
            className="sideResizer"
            role="separator"
            aria-orientation="vertical"
            aria-label="Resize sidebar"
            aria-valuemin={MIN_W}
            aria-valuemax={MAX_W}
            aria-valuenow={sideW}
            tabIndex={0}
            style={{ left: sideW - 3 }}
            onKeyDown={(e) => {
              const step = e.key === 'ArrowLeft' ? -16 : e.key === 'ArrowRight' ? 16 : 0;
              if (!step) return;
              e.preventDefault();
              resizeTo(sideW + step);
            }}
            onPointerDown={(e) => {
              e.preventDefault();
              const el = e.currentTarget;
              el.setPointerCapture(e.pointerId);
              document.body.classList.add('resizing');
              const move = (ev: PointerEvent) => {
                // Dragging far to the left folds the sidebar into the icon rail, like Elementor's panel.
                if (ev.clientX < MIN_W - 60) return;
                resizeTo(ev.clientX);
              };
              const up = (ev: PointerEvent) => {
                el.removeEventListener('pointermove', move);
                el.removeEventListener('pointerup', up);
                document.body.classList.remove('resizing');
                if (ev.clientX < MIN_W - 60) toggleCollapsed();
              };
              el.addEventListener('pointermove', move);
              el.addEventListener('pointerup', up);
            }}
          />
        )}
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
              <h1 ref={titleRef}>{current?.label}</h1>
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
          {coreError && section !== 'overview' && (
            <div className="error offlineBar" role="alert">
              <span>The VSN agent is offline, so this screen cannot load data.</span>
              <button className="primary" disabled={busy} onClick={() => void refresh(true)}>
                {busy ? 'Starting agent…' : 'Start agent'}
              </button>
            </div>
          )}
          {optionalErrors.length > 0 && (
            <div className="warning">Partial refresh: {optionalErrors.join(' · ')}</div>
          )}
          <div className="content" ref={contentRef}>
            <h2 className="srOnly">{current?.label} content</h2>
            {section === 'overview' && connection === 'offline' && (
              <AgentOffline
                error={coreError ?? ''}
                lastSeen={lastSeen}
                busy={busy}
                onRetry={() => void refresh(true)}
              />
            )}
            {section === 'overview' && connection !== 'offline' && (
              <Overview
                machine={machine}
                status={status}
                installed={installed}
                processes={processes.length}
                backends={backends}
                remote={remote}
                onNavigate={go}
              />
            )}
            {section === 'overview' && (
              <details className="panel coverage">
                <summary>Capability coverage</summary>
                <div className="panelHead">
                  <div>
                    <span className="kicker">Desktop source coverage</span>
                    <h2>{DESKTOP_CAPABILITIES.length} operator capabilities</h2>
                  </div>
                  <Badge ok label="Explicit coverage" />
                </div>
                <p className="hint">
                  Projects, services, runtimes, process/container management, relational and advanced
                  databases, jobs, files, terminal, networking and remote administration are explicitly
                  surfaced. Native installer/signing acceptance is tracked under P30.
                </p>
              </details>
            )}
            {section === 'settings' && <Settings machine={machine} status={status} />}
            {section === 'updates' && <Updates />}
            {section === 'spaces' && <Spaces />}
            {section === 'mail' && <Mail />}
            {section === 'about' && <About />}
            {section === 'backups' && <Backups />}
            {section === 'help' && <Help />}
            {section === 'git' && <SourceControl />}
            {section === 'analytics' && <Analytics />}
            {section === 'deploy' && <Deploy />}
            {section === 'assistant' && <Assistant />}
            {section === 'api' && <ApiClient />}
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
