import { useEffect, useMemo, useState } from 'react';
import { call } from '../api/agent';
import type { SpaceSummary } from '../contracts';
import { Badge } from '../components/ui';
import { Tabs } from '../components/Tabs';
import { useToast } from '../components/Toast';

/** One saved request: `<space>/.devify/api/<name>.json`, plain JSON so it diffs well in git. */
type ApiRequest = {
  name: string;
  method: string;
  port: number;
  path: string;
  headers: Record<string, string>;
  body: string;
};
type FileEntry = { name: string; is_dir: boolean };
type HttpResponse = {
  status: number;
  headers: Record<string, string>;
  text: string | null;
  truncated: boolean;
};

const METHODS = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'];
const VIEWS = [
  ['body', 'Body'],
  ['headers', 'Headers'],
] as const;
type View = (typeof VIEWS)[number][0];
const NAME_RE = /^[A-Za-z0-9][A-Za-z0-9 _.-]{0,63}$/;

const blank = (port: number): ApiRequest => ({
  name: 'new-request',
  method: 'GET',
  port,
  path: '/',
  headers: { accept: 'application/json' },
  body: '',
});

/** `{{VAR}}` comes from the space: its env, `PORT_<KIND>` for each service, `DOMAIN`. */
export function spaceVars(space: SpaceSummary | undefined): Record<string, string> {
  const vars: Record<string, string> = { ...(space?.spec?.env ?? {}) };
  if (space?.spec?.domain) vars.DOMAIN = space.spec.domain;
  for (const s of space?.spec?.services ?? [])
    if (s.port) vars[`PORT_${s.kind.toUpperCase()}`] = String(s.port);
  return vars;
}
export const fill = (text: string, vars: Record<string, string>) =>
  text.replace(/\{\{\s*([A-Za-z_][A-Za-z0-9_]*)\s*\}\}/g, (m, k: string) => vars[k] ?? m);

const headersToText = (h: Record<string, string>) =>
  Object.entries(h)
    .map(([k, v]) => `${k}: ${v}`)
    .join('\n');
function textToHeaders(text: string): Record<string, string> {
  const out: Record<string, string> = {};
  for (const line of text.split('\n')) {
    const i = line.indexOf(':');
    if (i > 0) out[line.slice(0, i).trim().toLowerCase()] = line.slice(i + 1).trim();
  }
  return out;
}
function toBase64(text: string) {
  let binary = '';
  for (const byte of new TextEncoder().encode(text)) binary += String.fromCharCode(byte);
  return btoa(binary);
}
function pretty(text: string | null) {
  if (text == null) return '(binary body)';
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch {
    return text;
  }
}

/** Postman-style API client per space (DVF-05). Requests go only to 127.0.0.1 through preview.request. */
export function ApiClient() {
  const [spaces, setSpaces] = useState<SpaceSummary[]>([]);
  const [root, setRoot] = useState('');
  const [saved, setSaved] = useState<string[]>([]);
  const [req, setReq] = useState<ApiRequest>(blank(3000));
  const [headerText, setHeaderText] = useState(headersToText(req.headers));
  const [res, setRes] = useState<HttpResponse | null>(null);
  const [ms, setMs] = useState(0);
  const [view, setView] = useState<View>('body');
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');
  const toast = useToast();
  const valid = useMemo(() => spaces.filter((s) => s.spec), [spaces]);
  const space = valid.find((s) => s.root === root) ?? valid[0];
  const vars = useMemo(() => spaceVars(space), [space]);
  const dir = space ? `${space.root}/.devify/api` : '';

  useEffect(() => {
    call<SpaceSummary[]>('space.list')
      .then(setSpaces)
      .catch((e) => setErr(String(e)));
  }, []);

  async function loadList() {
    if (!dir) return;
    try {
      const entries = await call<FileEntry[]>('files.list', { path: dir });
      setSaved(
        entries
          .filter((e) => !e.is_dir && e.name.endsWith('.json'))
          .map((e) => e.name.slice(0, -5))
          .sort(),
      );
    } catch {
      setSaved([]);
    }
  }
  useEffect(() => {
    setRes(null);
    void loadList();
  }, [dir]);

  function edit(next: ApiRequest) {
    setReq(next);
    setHeaderText(headersToText(next.headers));
  }
  async function open(name: string) {
    try {
      const f = await call<{ content: string }>('files.read', { path: `${dir}/${name}.json` });
      edit({ ...blank(3000), ...(JSON.parse(f.content) as Partial<ApiRequest>), name });
      setRes(null);
    } catch (e) {
      toast(String(e), 'danger');
    }
  }
  async function save() {
    if (!NAME_RE.test(req.name)) {
      toast('Name: letters, digits, space, dot, dash or underscore (max 64).', 'danger');
      return;
    }
    const body: ApiRequest = { ...req, headers: textToHeaders(headerText) };
    try {
      await call('files.write', {
        path: `${dir}/${req.name}.json`,
        content: JSON.stringify(body, null, 2) + '\n',
      });
      toast(`Saved .devify/api/${req.name}.json`);
      await loadList();
    } catch (e) {
      toast(String(e), 'danger');
    }
  }
  async function send() {
    setBusy(true);
    setErr('');
    const headers = Object.fromEntries(
      Object.entries(textToHeaders(headerText)).map(([k, v]) => [k, fill(v, vars)]),
    );
    const started = performance.now();
    try {
      const port = Number(fill(String(req.port), vars));
      const r = await call<HttpResponse>('preview.request', {
        port,
        path: fill(req.path, vars),
        method: req.method,
        headers,
        ...(req.body ? { body_base64: toBase64(fill(req.body, vars)) } : {}),
      });
      setRes(r);
      setView('body');
    } catch (e) {
      setRes(null);
      setErr(String(e));
    } finally {
      setMs(Math.round(performance.now() - started));
      setBusy(false);
    }
  }

  if (!valid.length)
    return (
      <section className="panel" aria-labelledby="api-title">
        <div className="panelHead">
          <h3 id="api-title">API client</h3>
        </div>
        <div className="empty">
          {err || 'Create a space in Spaces first; its requests are saved inside it.'}
        </div>
      </section>
    );

  return (
    <>
      <section className="panel" aria-labelledby="api-title">
        <div className="panelHead">
          <div>
            <span className="kicker">Per-space requests · saved in .devify/api</span>
            <h3 id="api-title">API client</h3>
          </div>
          <button className="ghost" onClick={() => edit(blank(req.port))}>
            New request
          </button>
        </div>
        <div className="formRow">
          <label className="stacked">
            Space
            <select value={space?.root ?? ''} onChange={(e) => setRoot(e.target.value)}>
              {valid.map((s) => (
                <option key={s.root} value={s.root}>
                  {s.spec?.name}
                </option>
              ))}
            </select>
          </label>
          <label className="stacked">
            Saved requests
            <select value="" onChange={(e) => e.target.value && void open(e.target.value)}>
              <option value="">{saved.length ? 'Open…' : 'None saved yet'}</option>
              {saved.map((n) => (
                <option key={n} value={n}>
                  {n}
                </option>
              ))}
            </select>
          </label>
        </div>
        <div className="formRow">
          <label className="stacked">
            Name
            <input value={req.name} onChange={(e) => setReq({ ...req, name: e.target.value })} />
          </label>
          <label className="stacked">
            Method
            <select value={req.method} onChange={(e) => setReq({ ...req, method: e.target.value })}>
              {METHODS.map((m) => (
                <option key={m}>{m}</option>
              ))}
            </select>
          </label>
          <label className="stacked">
            Port (127.0.0.1)
            <input
              type="number"
              min={1}
              max={65535}
              value={req.port}
              onChange={(e) => setReq({ ...req, port: Number(e.target.value) })}
            />
          </label>
          <label className="stacked">
            Path
            <input value={req.path} onChange={(e) => setReq({ ...req, path: e.target.value })} />
          </label>
        </div>
        <label className="stacked">
          Headers (one “name: value” per line)
          <textarea rows={3} value={headerText} onChange={(e) => setHeaderText(e.target.value)} />
        </label>
        <label className="stacked">
          Body
          <textarea rows={6} value={req.body} onChange={(e) => setReq({ ...req, body: e.target.value })} />
        </label>
        <p className="hint">
          Variables:{' '}
          {Object.keys(vars).length
            ? Object.keys(vars)
                .map((k) => `{{${k}}}`)
                .join(' ')
            : 'none'}{' '}
          — taken from the space's env, service ports and domain.
        </p>
        <div className="buttonRow">
          <button onClick={() => void send()} disabled={busy}>
            {busy ? 'Sending…' : 'Send'}
          </button>
          <button className="ghost" onClick={() => void save()}>
            Save to space
          </button>
        </div>
        {err && <div className="error">{err}</div>}
      </section>
      {res && (
        <section className="panel" aria-labelledby="api-res">
          <div className="panelHead">
            <div>
              <span className="kicker">
                {ms} ms{res.truncated ? ' · truncated' : ''}
              </span>
              <h3 id="api-res">Response</h3>
            </div>
            <Badge ok={res.status < 400} label={`HTTP ${res.status}`} />
          </div>
          <Tabs tabs={VIEWS} value={view} onChange={setView} label="Response view" panelId="api-view" />
          <pre className="json" id="api-view" role="tabpanel" aria-labelledby={`api-view-tab-${view}`}>
            {view === 'body' ? pretty(res.text) : headersToText(res.headers)}
          </pre>
        </section>
      )}
    </>
  );
}
