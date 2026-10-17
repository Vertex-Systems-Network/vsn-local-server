import { useMemo, useState } from 'react';
import type { ProcessInfo } from '../contracts';
import { DataTable } from '../components/DataTable';
import { filterProcesses, type ProcessSort as SortKey } from '../screenLogic';

export function Processes({ rows }: { rows: ProcessInfo[] }) {
  const [query, setQuery] = useState('');
  const [sort, setSort] = useState<SortKey>('name');
  const shown = useMemo(() => filterProcesses(rows, query, sort), [rows, query, sort]);
  return (
    <section className="panel">
      <div className="panelHead">
        <div>
          <span className="kicker">Machine</span>
          <h3>Process snapshot</h3>
        </div>
        <span>
          {shown.length} of {rows.length} shown
        </span>
      </div>
      <div className="formRow">
        <input
          type="search"
          aria-label="Filter processes"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Filter by name, PID or command"
        />
        <select aria-label="Sort processes" value={sort} onChange={(e) => setSort(e.target.value as SortKey)}>
          <option value="name">Sort by name</option>
          <option value="pid">Sort by PID</option>
        </select>
      </div>
      <DataTable
        caption="Running processes"
        rows={shown}
        rowKey={(p) => String(p.pid)}
        empty={rows.length ? 'No process matches this filter.' : 'No processes in the current snapshot.'}
        columns={[
          { key: 'pid', label: 'PID', className: 'num', render: (p) => <code>{p.pid}</code> },
          { key: 'name', label: 'Name', render: (p) => <b>{p.name}</b> },
          {
            key: 'cmd',
            label: 'Command',
            className: 'hideSm',
            render: (p) => <span className="truncate cmd">{p.command ?? '—'}</span>,
          },
        ]}
      />
    </section>
  );
}
