import type { ContainerBackend, MachineIdentity, RemoteConfig } from '../contracts';
import { Badge, Stat } from '../components/ui';

export function Overview({
  machine,
  status,
  installed,
  processes,
  backends,
  remote,
}: {
  machine: MachineIdentity | null;
  status: any;
  installed: number;
  processes: number;
  backends: ContainerBackend[];
  remote: RemoteConfig | null;
}) {
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
        <Stat
          label="Container engines"
          value={backends.filter((b) => b.installed).length}
          hint="Docker / Podman"
        />
        <Stat
          label="Remote channel"
          value={remote?.enabled ? 'ON' : 'OFF'}
          hint={remote?.control_plane_url ?? 'Not configured'}
        />
      </div>
      <section className="panel">
        <div className="panelHead">
          <div>
            <span className="kicker">Security boundary</span>
            <h3>Local agent posture</h3>
          </div>
        </div>
        <div className="grid2">
          <div className="info">
            <span>Secure store</span>
            <b>{status?.security?.secure_store ?? '—'}</b>
          </div>
          <div className="info">
            <span>Agent health</span>
            <b>{status?.health?.detail ?? '—'}</b>
          </div>
        </div>
      </section>
    </>
  );
}
