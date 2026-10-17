export function Badge({ ok, label }: { ok: boolean; label: string }) {
  return (
    <span className={ok ? 'badge ok' : 'badge muted'}>
      <i />
      {label}
    </span>
  );
}
export function Stat({ label, value, hint }: { label: string; value: string | number; hint?: string }) {
  return (
    <div className="stat">
      <span>{label}</span>
      <strong>{value}</strong>
      {hint && <small>{hint}</small>}
    </div>
  );
}
export function JsonView({ value }: { value: unknown }) {
  return <pre className="json">{JSON.stringify(value, null, 2)}</pre>;
}
