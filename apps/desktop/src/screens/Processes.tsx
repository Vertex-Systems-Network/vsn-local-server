import type { ProcessInfo } from '../contracts';

export function Processes({ rows }: { rows: ProcessInfo[] }) {
  return (
    <section className="panel">
      <div className="panelHead">
        <div>
          <span className="kicker">Machine</span>
          <h3>Process snapshot</h3>
        </div>
        <span>{rows.length} shown</span>
      </div>
      <div className="table">
        <div className="tr th">
          <span>PID</span>
          <span>Name</span>
          <span>Command</span>
        </div>
        {rows.map((p) => (
          <div className="tr" key={p.pid}>
            <code>{p.pid}</code>
            <b>{p.name}</b>
            <span className="truncate">{p.command ?? '—'}</span>
          </div>
        ))}
      </div>
    </section>
  );
}
