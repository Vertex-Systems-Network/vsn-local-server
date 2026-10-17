import type { ContainerBackend } from '../contracts';
import { Badge } from '../components/ui';
import { DataTable } from '../components/DataTable';

export function Containers({ rows }: { rows: ContainerBackend[] }) {
  return (
    <section className="panel">
      <div className="panelHead">
        <div>
          <span className="kicker">Container backend</span>
          <h3>Engines</h3>
        </div>
        <span>{rows.filter((r) => r.daemon_reachable).length} reachable</span>
      </div>
      <DataTable
        caption="Container engines"
        rows={rows}
        rowKey={(r) => r.id}
        empty="No container engine detected. Install Docker or Podman."
        columns={[
          { key: 'id', label: 'Engine', render: (r) => <b>{r.id}</b> },
          { key: 'version', label: 'Version', render: (r) => r.version ?? 'Executable not detected' },
          {
            key: 'state',
            label: 'Status',
            render: (r) => (
              <Badge
                ok={!!r.daemon_reachable}
                label={
                  r.daemon_reachable ? 'Daemon reachable' : r.installed ? 'Daemon unavailable' : 'Not installed'
                }
              />
            ),
          },
        ]}
      />
    </section>
  );
}
