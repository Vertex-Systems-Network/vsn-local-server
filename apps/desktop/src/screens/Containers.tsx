import type { ContainerBackend } from '../contracts';
import { Badge } from '../components/ui';

export function Containers({ rows }: { rows: ContainerBackend[] }) {
  return (
    <div className="cards">
      {rows.map((r) => (
        <article className="panel engine" key={r.id}>
          <span className="kicker">Container backend</span>
          <h3>{r.id}</h3>
          <p>{r.version ?? 'Executable not detected'}</p>
          <Badge
            ok={!!r.daemon_reachable}
            label={
              r.daemon_reachable
                ? 'Daemon reachable'
                : r.installed
                  ? 'Installed / daemon unavailable'
                  : 'Not installed'
            }
          />
        </article>
      ))}
    </div>
  );
}
