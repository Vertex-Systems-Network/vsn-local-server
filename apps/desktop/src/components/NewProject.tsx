import { useEffect, useState } from 'react';
import { call } from '../api/agent';
import { ResultView } from './ResultView';
import { useToast } from './Toast';

type Choice = { key: string; source: 'builtin' | 'framework'; id: string; name: string; hint: string };
type Plan = { program: string; args: string[]; destination: string; note: string };
type Framework = { id: string; name: string; language: string; requires: string[] };

const BUILTIN_HINTS: Record<string, string> = {
  laravel: 'PHP · Composer',
  node: 'Node.js · npm',
  django: 'Python · Django',
  rust: 'Rust · Cargo',
  go: 'Go modules',
};
const title = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

/**
 * New project (Figma: "28 Projects — New project wizard"): the frozen built-in templates
 * plus the VSN Devify framework catalog (WordPress, Symfony, Next.js, …).
 */
export function NewProject() {
  const [choices, setChoices] = useState<Choice[]>([]);
  const [key, setKey] = useState('');
  const [path, setPath] = useState('');
  const [plan, setPlan] = useState<Plan | null>(null);
  const [result, setResult] = useState<unknown>(null);
  const [err, setErr] = useState('');
  const [busy, setBusy] = useState(false);
  const toast = useToast();
  const choice = choices.find((c) => c.key === key);

  useEffect(() => {
    Promise.allSettled([call<string[]>('project.templates'), call<Framework[]>('framework.catalog')]).then(
      ([builtin, catalog]) => {
        const list: Choice[] = [];
        if (builtin.status === 'fulfilled')
          for (const t of builtin.value)
            list.push({
              key: `builtin:${t}`,
              source: 'builtin',
              id: t,
              name: title(t),
              hint: BUILTIN_HINTS[t] ?? 'Built-in',
            });
        if (catalog.status === 'fulfilled')
          for (const f of catalog.value)
            list.push({
              key: `framework:${f.id}`,
              source: 'framework',
              id: f.id,
              name: f.name,
              hint: `${f.language} · needs ${f.requires.join(', ')}`,
            });
        setChoices(list);
        setKey((cur) => cur || list[0]?.key || '');
        if (builtin.status === 'rejected') setErr(String(builtin.reason));
      },
    );
  }, []);
  // A plan is only valid for the template and folder it was made for.
  useEffect(() => setPlan(null), [key, path]);

  async function review() {
    if (!choice) return;
    setErr('');
    setResult(null);
    try {
      if (choice.source === 'builtin') {
        const p = await call<{
          program: string;
          args: string[];
          destination: string;
          requires_network: boolean;
        }>('project.bootstrap-plan', { template: choice.id, path });
        setPlan({ ...p, note: p.requires_network ? 'downloads packages from the internet' : '' });
      } else {
        const p = await call<{ program: string; args: string[]; destination: string; requires: string[] }>(
          'framework.plan',
          { id: choice.id, path },
        );
        setPlan({ ...p, note: `needs ${p.requires.join(', ')} · downloads packages from the internet` });
      }
    } catch (e) {
      setErr(String(e));
    }
  }
  async function create() {
    if (!choice) return;
    setErr('');
    setBusy(true);
    try {
      const r =
        choice.source === 'builtin'
          ? await call<{ status_code: number | null }>('project.bootstrap', { template: choice.id, path })
          : await call<{ status_code: number | null }>('framework.create', { id: choice.id, path });
      setResult(r);
      const ok = r.status_code === 0;
      toast(
        ok ? `Created ${choice.name} project` : `${choice.name} finished with errors`,
        ok ? 'info' : 'danger',
      );
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="panel" aria-labelledby="new-project">
      <div className="panelHead">
        <div>
          <span className="kicker">Templates and frameworks</span>
          <h3 id="new-project">New project</h3>
        </div>
      </div>
      <div className="choiceGrid" role="radiogroup" aria-labelledby="new-project">
        {choices.map((c) => (
          <button
            key={c.key}
            type="button"
            role="radio"
            aria-checked={key === c.key}
            className="choice"
            onClick={() => setKey(c.key)}
          >
            {c.name}
            <small>{c.hint}</small>
          </button>
        ))}
      </div>
      <label className="stacked">
        Destination folder (inside a workspace root)
        <input value={path} onChange={(e) => setPath(e.target.value)} placeholder="C:\work\my-app" />
      </label>
      <div className="buttonRow">
        <button className="ghost" onClick={() => void review()} disabled={!choice || !path}>
          Review plan
        </button>
        <button className="primary" onClick={() => void create()} disabled={!plan || busy}>
          {busy ? 'Creating… (can take a few minutes)' : 'Create project'}
        </button>
      </div>
      {plan && (
        <div className="result">
          <h4>VSN will run</h4>
          <pre className="json">{[plan.program, ...plan.args].join(' ')}</pre>
          <p className="hint">
            For <code>{plan.destination}</code>
            {plan.note ? ` · ${plan.note}` : ''}
          </p>
        </div>
      )}
      <ResultView title="Result" value={result} error={err} />
    </section>
  );
}
