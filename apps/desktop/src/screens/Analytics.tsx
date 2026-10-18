import { useEffect, useRef, useState } from 'react';
import { call } from '../api/agent';
import { Badge, Stat } from '../components/ui';
import { DataTable } from '../components/DataTable';

type ProcMetrics = {
  id: string;
  label: string;
  pid: number;
  running: boolean;
  started_at_unix: number;
  cpu_percent: number | null;
  memory_bytes: number | null;
};
type SpaceMetrics = { space: string; root: string; processes: ProcMetrics[]; data_bytes: number };

const POLL_MS = 4000;
const SAMPLES = 30;

export function formatBytes(n: number | null | undefined) {
  if (n == null) return '—';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let v = n;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v < 10 && i > 0 ? v.toFixed(1) : Math.round(v)} ${units[i]}`;
}
function uptime(startedUnix: number) {
  const s = Math.max(0, Math.floor(Date.now() / 1000 - startedUnix));
  if (s < 60) return `${s}s`;
  if (s < 3600) return `${Math.floor(s / 60)}m`;
  return `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m`;
}

/** Tiny inline trend line; values are scaled to the series' own max. */
function Spark({ values, label }: { values: number[]; label: string }) {
  if (values.length < 2) return <span className="hint">…</span>;
  const max = Math.max(...values, 1);
  const pts = values.map((v, i) => `${(i / (SAMPLES - 1)) * 100},${28 - (v / max) * 26}`).join(' ');
  return (
    <svg className="spark" viewBox="0 0 100 30" preserveAspectRatio="none" role="img" aria-label={label}>
      <polyline points={pts} fill="none" vectorEffect="non-scaling-stroke" />
    </svg>
  );
}

/** Analytics (DVF-10): live CPU and memory of everything VSN runs for each space. */
type DomainTraffic = {
  domain: string;
  requests: number;
  errors_4xx: number;
  errors_5xx: number;
  avg_ms: number;
  slowest: [string, number][];
};

/** Requests per routed name.test domain, from Caddy's access logs (DVF-10). */
function RequestLog() {
  const [rows, setRows] = useState<DomainTraffic[]>([]);
  useEffect(() => {
    const load = () =>
      call<DomainTraffic[]>('domain.traffic')
        .then(setRows)
        .catch(() => setRows([]));
    void load();
    const timer = window.setInterval(load, 10000);
    return () => window.clearInterval(timer);
  }, []);
  return (
    <section className="panel" aria-labelledby="traffic-title">
      <div className="panelHead">
        <div>
          <span className="kicker">Through name.test domains</span>
          <h3 id="traffic-title">Requests</h3>
        </div>
      </div>
      <DataTable
        caption="Requests per domain"
        rows={rows}
        rowKey={(r) => r.domain}
        empty="No routed domains yet. Use Route to port in Spaces; requests through the domain appear here."
        columns={[
          { key: 'domain', label: 'Domain', render: (r) => r.domain },
          { key: 'req', label: 'Requests', render: (r) => r.requests },
          { key: 'avg', label: 'Avg time', render: (r) => `${r.avg_ms} ms` },
          {
            key: 'err',
            label: 'Errors',
            render: (r) => (
              <Badge ok={r.errors_5xx === 0} label={`${r.errors_4xx} × 4xx · ${r.errors_5xx} × 5xx`} />
            ),
          },
          {
            key: 'slow',
            label: 'Slowest',
            render: (r) =>
              r.slowest.length
                ? r.slowest
                    .slice(0, 3)
                    .map(([u, ms]) => `${u} (${Math.round(ms)} ms)`)
                    .join(', ')
                : '—',
          },
        ]}
      />
    </section>
  );
}

export function Analytics() {
  const [data, setData] = useState<SpaceMetrics[]>([]);
  const [err, setErr] = useState('');
  const [paused, setPaused] = useState(false);
  const history = useRef<Record<string, { cpu: number[]; mem: number[] }>>({});
  const [, setTick] = useState(0);

  useEffect(() => {
    if (paused) return;
    let alive = true;
    const load = async () => {
      try {
        const list = await call<SpaceMetrics[]>('space.metrics');
        if (!alive) return;
        for (const s of list)
          for (const p of s.processes) {
            const h = (history.current[p.id] ??= { cpu: [], mem: [] });
            h.cpu = [...h.cpu, p.cpu_percent ?? 0].slice(-SAMPLES);
            h.mem = [...h.mem, p.memory_bytes ?? 0].slice(-SAMPLES);
          }
        setData(list);
        setErr('');
        setTick((t) => t + 1);
      } catch (e) {
        if (alive) setErr(String(e));
      }
    };
    void load();
    const t = window.setInterval(() => void load(), POLL_MS);
    return () => {
      alive = false;
      window.clearInterval(t);
    };
  }, [paused]);

  const all = data.flatMap((s) => s.processes);
  const running = all.filter((p) => p.running);
  const cpu = running.reduce((a, p) => a + (p.cpu_percent ?? 0), 0);
  const mem = running.reduce((a, p) => a + (p.memory_bytes ?? 0), 0);
  const disk = data.reduce((a, s) => a + s.data_bytes, 0);
  const cpuKnown = running.some((p) => p.cpu_percent != null);

  return (
    <>
      <section className="panel" aria-labelledby="an-title">
        <div className="panelHead">
          <div>
            <span className="kicker">Live · every {POLL_MS / 1000} s</span>
            <h3 id="an-title">Analytics</h3>
          </div>
          <button className="ghost" onClick={() => setPaused((p) => !p)}>
            {paused ? 'Resume' : 'Pause'}
          </button>
        </div>
        {err && <div className="error">{err}</div>}
        <div className="stats">
          <Stat label="Running processes" value={running.length} hint={`${all.length} managed`} />
          <Stat
            label="CPU"
            value={cpuKnown ? `${cpu.toFixed(1)}%` : '—'}
            hint={cpuKnown ? 'All spaces' : 'Not reported on Windows yet'}
          />
          <Stat label="Memory" value={formatBytes(mem)} hint="All spaces" />
          <Stat label="Space data" value={formatBytes(disk)} hint=".devify/data + logs" />
        </div>
      </section>
      {data.length === 0 && !err && (
        <section className="panel">
          <div className="empty">
            No spaces yet. Create one in Spaces and start its services to see them here.
          </div>
        </section>
      )}
      {data.map((s) => {
        const sMem = s.processes.reduce((a, p) => a + (p.running ? (p.memory_bytes ?? 0) : 0), 0);
        return (
          <section className="panel" key={s.root} aria-labelledby={`an-${s.space}`}>
            <div className="panelHead">
              <div>
                <span className="kicker">
                  {formatBytes(sMem)} memory · {formatBytes(s.data_bytes)} data
                </span>
                <h3 id={`an-${s.space}`}>{s.space}</h3>
              </div>
              <Badge
                ok={s.processes.some((p) => p.running)}
                label={`${s.processes.filter((p) => p.running).length} running`}
              />
            </div>
            <DataTable
              caption={`${s.space} processes`}
              rows={s.processes}
              rowKey={(p) => p.id}
              empty="Nothing running. Start a service or a command in Spaces."
              columns={[
                { key: 'name', label: 'Process', render: (p) => <b>{p.label}</b> },
                {
                  key: 'state',
                  label: 'State',
                  render: (p) => (
                    <Badge ok={p.running} label={p.running ? `Up ${uptime(p.started_at_unix)}` : 'Stopped'} />
                  ),
                },
                {
                  key: 'cpu',
                  label: 'CPU',
                  className: 'num',
                  render: (p) => (p.cpu_percent == null ? '—' : `${p.cpu_percent.toFixed(1)}%`),
                },
                {
                  key: 'cpuTrend',
                  label: 'CPU trend',
                  className: 'hideSm',
                  render: (p) => (
                    <Spark values={history.current[p.id]?.cpu ?? []} label={`${p.label} CPU trend`} />
                  ),
                },
                { key: 'mem', label: 'Memory', className: 'num', render: (p) => formatBytes(p.memory_bytes) },
                {
                  key: 'memTrend',
                  label: 'Memory trend',
                  className: 'hideSm',
                  render: (p) => (
                    <Spark values={history.current[p.id]?.mem ?? []} label={`${p.label} memory trend`} />
                  ),
                },
              ]}
            />
          </section>
        );
      })}
      <RequestLog />
    </>
  );
}
