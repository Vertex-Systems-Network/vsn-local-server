import { useEffect, useRef, type ReactNode } from 'react';
import { staggerRows } from '../motion';

export type Column<T> = { key: string; label: string; render: (row: T) => ReactNode; className?: string };

/** Accessible data table with a GSAP row stagger whenever the row keys change. */
export function DataTable<T>({
  caption,
  columns,
  rows,
  rowKey,
  empty,
}: {
  caption: string;
  columns: Column<T>[];
  rows: T[];
  rowKey: (row: T) => string;
  empty: ReactNode;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const signature = rows.map(rowKey).join('|');
  useEffect(() => {
    staggerRows(ref.current);
  }, [signature]);
  if (!rows.length) return <div className="empty">{empty}</div>;
  return (
    <div className="tableWrap" ref={ref}>
      <table className="dataTable">
        <caption className="srOnly">{caption}</caption>
        <thead>
          <tr>
            {columns.map((c) => (
              <th key={c.key} scope="col" className={c.className}>
                {c.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={rowKey(r)}>
              {columns.map((c) => (
                <td key={c.key} className={c.className}>
                  {c.render(r)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
