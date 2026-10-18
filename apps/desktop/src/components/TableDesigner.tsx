import { useState } from 'react';
import { createTableSql, type DesignColumn, type SqlDialect } from '../screenLogic';
import { useToast } from './Toast';

const blank = (): DesignColumn => ({ name: '', type: 'TEXT', primary: false, nullable: true, default: '' });
const TYPES: Record<SqlDialect, string[]> = {
  postgres: [
    'BIGSERIAL',
    'BIGINT',
    'INTEGER',
    'TEXT',
    'VARCHAR(255)',
    'BOOLEAN',
    'NUMERIC(12,2)',
    'TIMESTAMPTZ',
    'DATE',
    'JSONB',
    'UUID',
  ],
  mysql: [
    'BIGINT AUTO_INCREMENT',
    'BIGINT',
    'INT',
    'TEXT',
    'VARCHAR(255)',
    'TINYINT(1)',
    'DECIMAL(12,2)',
    'DATETIME',
    'DATE',
    'JSON',
  ],
  sqlite: ['INTEGER', 'TEXT', 'REAL', 'NUMERIC', 'BLOB'],
};

/** Visual table designer: columns in, CREATE TABLE SQL out for the chosen engine. */
export function TableDesigner() {
  const toast = useToast();
  const [dialect, setDialect] = useState<SqlDialect>('postgres');
  const [table, setTable] = useState('new_table');
  const [cols, setCols] = useState<DesignColumn[]>([
    { name: 'id', type: 'BIGSERIAL', primary: true, nullable: false, default: '' },
    blank(),
  ]);
  const sql = createTableSql(table, cols, dialect);
  const edit = (i: number, patch: Partial<DesignColumn>) =>
    setCols((all) => all.map((c, n) => (n === i ? { ...c, ...patch } : c)));

  return (
    <section className="panel" aria-labelledby="db-designer">
      <div className="panelHead">
        <div>
          <span className="kicker">Schema</span>
          <h3 id="db-designer">Table designer</h3>
        </div>
      </div>
      <div className="formGrid">
        <label>
          Engine
          <select value={dialect} onChange={(e) => setDialect(e.target.value as SqlDialect)}>
            <option value="postgres">PostgreSQL</option>
            <option value="mysql">MySQL / MariaDB</option>
            <option value="sqlite">SQLite</option>
          </select>
        </label>
        <label>
          Table name
          <input value={table} onChange={(e) => setTable(e.target.value)} />
        </label>
      </div>
      <datalist id="designer-types">
        {TYPES[dialect].map((t) => (
          <option key={t} value={t} />
        ))}
      </datalist>
      <div className="tableWrap">
        <table className="dataTable">
          <caption className="srOnly">Columns of {table}</caption>
          <thead>
            <tr>
              <th scope="col">Column</th>
              <th scope="col">Type</th>
              <th scope="col">Primary key</th>
              <th scope="col">Allows empty</th>
              <th scope="col">Default</th>
              <th scope="col">
                <span className="srOnly">Remove</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {cols.map((c, i) => (
              <tr key={i}>
                <td>
                  <input
                    aria-label={`Column ${i + 1} name`}
                    value={c.name}
                    onChange={(e) => edit(i, { name: e.target.value })}
                  />
                </td>
                <td>
                  <input
                    aria-label={`Column ${i + 1} type`}
                    list="designer-types"
                    value={c.type}
                    onChange={(e) => edit(i, { type: e.target.value })}
                  />
                </td>
                <td>
                  <input
                    type="checkbox"
                    aria-label={`Column ${i + 1} is part of the primary key`}
                    checked={c.primary}
                    onChange={(e) => edit(i, { primary: e.target.checked })}
                  />
                </td>
                <td>
                  <input
                    type="checkbox"
                    aria-label={`Column ${i + 1} allows empty values`}
                    checked={c.nullable}
                    onChange={(e) => edit(i, { nullable: e.target.checked })}
                  />
                </td>
                <td>
                  <input
                    aria-label={`Column ${i + 1} default`}
                    value={c.default}
                    onChange={(e) => edit(i, { default: e.target.value })}
                  />
                </td>
                <td>
                  <button
                    className="ghost"
                    aria-label={`Remove column ${i + 1}`}
                    onClick={() => setCols((all) => all.filter((_, n) => n !== i))}
                  >
                    Remove
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="buttonRow">
        <button className="ghost" onClick={() => setCols((all) => [...all, blank()])}>
          Add column
        </button>
        <button
          className="primary"
          onClick={() => void navigator.clipboard?.writeText(sql).then(() => toast('SQL copied'))}
        >
          Copy SQL
        </button>
      </div>
      <label className="stacked">
        Generated SQL
        <textarea readOnly value={sql} rows={Math.min(14, sql.split('\n').length + 1)} />
      </label>
      <p className="hint">
        Run the statement in your database tool or the space's database shell. Defaults are written as typed,
        so quote text values (for example 'active').
      </p>
    </section>
  );
}
