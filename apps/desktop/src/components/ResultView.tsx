import { useEffect, useRef } from 'react';
import { popIn } from '../motion';
import { JsonView } from './ui';
import { resultFacts, scalarText } from '../screenLogic';

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
      <details className="inspect">
        <summary>Inspect raw response</summary>
        <JsonView value={value} />
      </details>
    </div>
  );
}
