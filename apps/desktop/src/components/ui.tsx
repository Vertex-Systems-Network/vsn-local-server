import { useEffect, useRef } from 'react';
import { countUp, pulse } from '../motion';

export function Badge({ ok, label }: { ok: boolean; label: string }) {
  const ref = useRef<HTMLSpanElement>(null);
  const first = useRef(true);
  useEffect(() => {
    if (first.current) {
      first.current = false;
      return;
    }
    pulse(ref.current);
  }, [ok, label]);
  return (
    <span ref={ref} className={ok ? 'badge ok' : 'badge muted'}>
      <i />
      {label}
    </span>
  );
}
export function Stat({ label, value, hint }: { label: string; value: string | number; hint?: string }) {
  const ref = useRef<HTMLElement>(null);
  useEffect(() => {
    if (typeof value === 'number') countUp(ref.current, value);
  }, [value]);
  return (
    <div className="stat">
      <span>{label}</span>
      <strong ref={ref}>{value}</strong>
      {hint && <small>{hint}</small>}
    </div>
  );
}
export function JsonView({ value }: { value: unknown }) {
  return <pre className="json">{JSON.stringify(value, null, 2)}</pre>;
}
