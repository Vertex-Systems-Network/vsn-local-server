import { useEffect, useRef } from 'react';
import { popIn } from '../motion';
import { JsonView } from './ui';

function scalar(v: unknown): string | null {
  if (v === null || v === undefined) return '—';
  if (typeof v === 'string' || typeof v === 'number') return String(v);
  if (typeof v === 'boolean') return v ? 'Yes' : 'No';
  return null;
}
function label(k: string) {
  const s = k.replace(/_/g, ' ');
  return s.charAt(0).toUpperCase() + s.slice(1);
}

/** Human summary of an agent result; the raw JSON stays one click away under "Inspect". */
export function ResultView({ title = 'Result', value, error }: { title?: string; value: unknown; error?: string }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (value !== null && value !== undefined) popIn(ref.current);
  }, [value]);
  if (error) return <div className="error" role="alert">{error}</div>;
  if (value === null || value === undefined) return null;
  const facts =
    typeof value === 'object' && !Array.isArray(value)
      ? Object.entries(value as Record<string, unknown>).flatMap(([k, v]) => {
          const s = scalar(v);
          return s === null ? [] : [[k, s] as const];
        })
      : [];
  const single = scalar(value);
  return (
    <div className="result" ref={ref}>
      <h4>{title}</h4>
      {single !== null && <p className="resultValue">{single}</p>}
      {facts.length > 0 && (
        <dl className="facts">
          {facts.slice(0, 12).map(([k, v]) => (
            <div key={k}>
              <dt>{label(k)}</dt>
              <dd>{v}</dd>
            </div>
          ))}
        </dl>
      )}
      {Array.isArray(value) && <p className="hint">{value.length} item(s)</p>}
      <details className="inspect">
        <summary>Inspect raw response</summary>
        <JsonView value={value} />
      </details>
    </div>
  );
}
