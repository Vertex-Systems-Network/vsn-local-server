import { useEffect, useRef } from 'react';
import { popIn } from '../motion';
import { JsonView } from './ui';
import { gridToCsv, resultFacts, scalarText, toGrid } from '../screenLogic';
import { useToast } from './Toast';

function save(name: string, text: string, type: string) {
  const url = URL.createObjectURL(new Blob([text], { type }));
  const a = document.createElement('a');
  a.href = url;
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/** Export buttons for a result that is a table of rows. */
function GridExport({ value }: { value: unknown }) {
  const toast = useToast();
  const grid = toGrid(value);
  if (!grid) return null;
  const csv = () => gridToCsv(grid);
  const stamp = new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-');
  return (
    <div className="buttonRow">
      <span className="hint">
        {grid.rows.length} row(s) × {grid.columns.length} column(s)
      </span>
      <button className="ghost" onClick={() => save(`result-${stamp}.csv`, csv(), 'text/csv')}>
        Save CSV
      </button>
      <button
        className="ghost"
        onClick={() => save(`result-${stamp}.json`, JSON.stringify(value, null, 2), 'application/json')}
      >
        Save JSON
      </button>
      <button
        className="ghost"
        onClick={() => void navigator.clipboard?.writeText(csv()).then(() => toast('CSV copied'))}
      >
        Copy CSV
      </button>
    </div>
  );
}

/** Human summary of an agent result; the raw JSON stays one click away under "Inspect". */
export function ResultView({ title = 'Result', value, error }: { title?: string; value: unknown; error?: string }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (value !== null && value !== undefined) popIn(ref.current);
  }, [value]);
  if (error) return <div className="error" role="alert">{error}</div>;
  if (value === null || value === undefined) return null;
  const facts = resultFacts(value);
  const single = scalarText(value);
  return (
    <div className="result" ref={ref}>
      <h4>{title}</h4>
      {single !== null && <p className="resultValue">{single}</p>}
      {facts.length > 0 && (
        <dl className="facts">
          {facts.slice(0, 12).map(([k, v]) => (
            <div key={k}>
              <dt>{k}</dt>
              <dd>{v}</dd>
            </div>
          ))}
        </dl>
      )}
      {Array.isArray(value) && <p className="hint">{value.length} item(s)</p>}
      <GridExport value={value} />
      <details className="inspect">
        <summary>Inspect raw response</summary>
        <JsonView value={value} />
      </details>
    </div>
  );
}
