import type { ContainerBackend, MachineIdentity, RemoteConfig } from '../contracts';
import { Badge, Stat } from '../components/ui';
import { Icon, type IconName } from '../components/Icon';

type Attention = { tone: 'danger' | 'warning' | 'info'; title: string; detail: string };

function attentionItems(status: any, backends: ContainerBackend[], remote: RemoteConfig | null): Attention[] {
  const items: Attention[] = [];
  if (!status?.security?.device_identity_ready)
    items.push({ tone: 'danger', title: 'Device identity not ready', detail: 'The agent has not provisioned a device key.' });
  if (!status?.security?.ipc_secret_ready)
    items.push({ tone: 'danger', title: 'Authenticated IPC not ready', detail: 'Desktop ↔ agent calls are not authenticated yet.' });
  const down = backends.filter((b) => b.installed && b.daemon_reachable === false);
  for (const b of down)
    items.push({ tone: 'warning', title: `${b.id} daemon unreachable`, detail: 'Installed but not responding. Start the engine.' });
  if (!backends.some((b) => b.installed))
    items.push({ tone: 'info', title: 'No container engine', detail: 'Install Docker or Podman to manage containers.' });
  if (remote?.enabled && !remote.control_plane_url)
    items.push({ tone: 'warning', title: 'Remote enabled without control plane', detail: 'Set a control-plane URL in Remote.' });
  return items;
}

export function Overview({
  machine,
  status,
  installed,
  processes,
  backends,
  remote,
  onNavigate,
}: {
  machine: MachineIdentity | null;
  status: any;
  installed: number;
  processes: number;
  backends: ContainerBackend[];
  remote: RemoteConfig | null;
  onNavigate?: (section: 'projects' | 'terminal' | 'database') => void;
}) {
  const attention = attentionItems(status, backends, remote);
  const quick: [('projects' | 'terminal' | 'database'), string, IconName][] = [
    ['projects', 'Projects', 'folder'],
    ['terminal', 'Open terminal', 'terminal'],
    ['database', 'Database Studio', 'database'],
  ];
  return (
    <>
      <section className="hero">
        <div>
          <span className="kicker">Machine workspace</span>
          <h2>{machine?.display_name ?? 'VSN machine'}</h2>
          <p>
            Runtimes, projects, databases, services, files and remote control stay behind the authenticated
            VSN Agent execution boundary.
          </p>
          <div className="heroBadges">
            <Badge ok={!!status?.security?.device_identity_ready} label="Device identity" />
            <Badge ok={!!status?.security?.ipc_secret_ready} label="Authenticated IPC" />
            <Badge ok={!!remote?.enabled} label={remote?.enabled ? 'Remote enabled' : 'Remote disabled'} />
          </div>
        </div>
        <div className="deviceId">
          <span>DEVICE ID</span>
          <code>{machine?.device_id ?? '—'}</code>
        </div>
      </section>
      <div className="stats">
        <Stat label="Detected runtimes" value={installed} hint="PATH + VSN registry" />
        <Stat label="Visible processes" value={processes} hint="Current snapshot" />
        <Stat label="Container engines" value={backends.filter((b) => b.installed).length} hint="Docker / Podman" />
        <Stat
          label="Remote channel"
          value={remote?.enabled ? 'ON' : 'OFF'}
          hint={remote?.control_plane_url ?? 'Not configured'}
        />
      </div>
      <div className="overviewGrid">
        <section className="panel" aria-labelledby="ov-attention">
          <div className="panelHead">
            <div>
              <span className="kicker">Health</span>
              <h3 id="ov-attention">Needs attention</h3>
            </div>
            <Badge ok={attention.length === 0} label={attention.length ? `${attention.length} open` : 'All clear'} />
          </div>
          {attention.length === 0 ? (
            <p className="hint">Nothing needs your attention right now.</p>
          ) : (
            <ul className="attention">
              {attention.map((a) => (
                <li key={a.title} className={`attention-${a.tone}`}>
                  <i aria-hidden="true" />
                  <div>
                    <b>{a.title}</b>
                    <span>{a.detail}</span>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </section>
        <section className="panel" aria-labelledby="ov-quick">
          <div className="panelHead">
            <div>
              <span className="kicker">Shortcuts</span>
              <h3 id="ov-quick">Quick actions</h3>
            </div>
          </div>
          <div className="quickActions">
            {quick.map(([id, label, icon]) => (
              <button key={id} type="button" className="ghost quickAction" onClick={() => onNavigate?.(id)}>
                <Icon name={icon} />
                {label}
              </button>
            ))}
          </div>
        </section>
      </div>
      <section className="panel" aria-labelledby="ov-machine">
        <div className="panelHead">
          <div>
            <span className="kicker">Security boundary</span>
            <h3 id="ov-machine">This machine</h3>
          </div>
        </div>
        <div className="grid2">
          <div className="info">
            <span>Operating system</span>
            <b>{machine?.os ?? '—'}</b>
          </div>
          <div className="info">
            <span>Secure store</span>
            <b>{status?.security?.secure_store ?? '—'}</b>
          </div>
          <div className="info">
            <span>Agent health</span>
            <b>{status?.health?.detail ?? '—'}</b>
          </div>
          <div className="info">
            <span>Remote poll interval</span>
            <b>{remote ? `${remote.poll_interval_ms} ms` : '—'}</b>
          </div>
        </div>
      </section>
    </>
  );
}
