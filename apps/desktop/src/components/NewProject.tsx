import { useEffect, useState } from 'react';
import { call } from '../api/agent';
import { ResultView } from './ResultView';
import { useToast } from './Toast';

type Plan = {
  template: string;
  destination: string;
  program: string;
  args: string[];
  requires_network: boolean;
};
const TEMPLATE_HINTS: Record<string, string> = {
  laravel: 'PHP · Composer',
  node: 'Node.js · npm',
  django: 'Python · Django',
  rust: 'Rust · Cargo',
  go: 'Go modules',
};

/** New project from a built-in template (Figma: "28 Projects — New project wizard"). */
export function NewProject() {
  const [templates, setTemplates] = useState<string[]>([]);
  const [template, setTemplate] = useState('');
  const [path, setPath] = useState('');
  const [plan, setPlan] = useState<Plan | null>(null);
  const [result, setResult] = useState<unknown>(null);
  const [err, setErr] = useState('');
  const [busy, setBusy] = useState(false);
  const toast = useToast();
  useEffect(() => {
    call<string[]>('project.templates')
      .then((t) => {
        setTemplates(t);
        setTemplate((cur) => cur || t[0] || '');
      })
      .catch((e) => setErr(String(e)));
  }, []);
  // A plan is only valid for the template and folder it was made for.
  useEffect(() => setPlan(null), [template, path]);
  async function review() {
    setErr('');
    setResult(null);
    try {
      setPlan(await call<Plan>('project.bootstrap-plan', { template, path }));
    } catch (e) {
      setErr(String(e));
    }
  }
  async function create() {
    setErr('');
    setBusy(true);
    try {
      const r = await call<{ status_code: number | null }>('project.bootstrap', { template, path });
      setResult(r);
      toast(
        r.status_code === 0 ? `Created ${template} project` : `${template} bootstrap finished with errors`,
        r.status_code === 0 ? 'info' : 'danger',
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
          <span className="kicker">Templates</span>
          <h3 id="new-project">New project</h3>
        </div>
      </div>
      <div className="choiceGrid" role="radiogroup" aria-labelledby="new-project">
        {templates.map((t) => (
          <button
            key={t}
            type="button"
            role="radio"
            aria-checked={template === t}
            className="choice"
            onClick={() => setTemplate(t)}
          >
            {t.charAt(0).toUpperCase() + t.slice(1)}
            <small>{TEMPLATE_HINTS[t] ?? 'Built-in template'}</small>
          </button>
        ))}
      </div>
      <label className="stacked">
        Destination folder (inside a workspace root)
        <input value={path} onChange={(e) => setPath(e.target.value)} placeholder="C:\work\my-app" />
      </label>
      <div className="buttonRow">
        <button className="ghost" onClick={() => void review()} disabled={!template || !path}>
          Review plan
        </button>
        <button className="primary" onClick={() => void create()} disabled={!plan || busy}>
          {busy ? 'Creating…' : 'Create project'}
        </button>
      </div>
      {plan && (
        <div className="result">
          <h4>VSN will run</h4>
          <pre className="json">{[plan.program, ...plan.args].join(' ')}</pre>
          <p className="hint">
            In <code>{plan.destination}</code>
            {plan.requires_network ? ' · downloads packages from the internet' : ''}
          </p>
        </div>
      )}
      <ResultView title="Bootstrap result" value={result} error={err} />
    </section>
  );
}
