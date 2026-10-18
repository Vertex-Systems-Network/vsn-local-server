import { useEffect, useRef, useState } from 'react';
import { call } from '../api/agent';
import type { SpaceSummary } from '../contracts';
import { Badge } from '../components/ui';
import { useToast } from '../components/Toast';

type Msg = { role: 'user' | 'assistant'; content: string };
type Status = { configured: boolean; default_model: string; models: string[] };
type Reply = {
  text: string;
  model: string;
  stop_reason: string;
  input_tokens: number;
  output_tokens: number;
};

const MODEL_LABEL: Record<string, string> = {
  'claude-opus-5-5': 'Claude Opus 5.5 (best)',
  'claude-sonnet-5-5': 'Claude Sonnet 5.5 (faster)',
  'claude-haiku-5-5': 'Claude Haiku 5.5 (cheapest)',
};

/** AI assistant (DVF-11): Claude with your own Anthropic API key, aware of the chosen space. */
export function Assistant() {
  const [status, setStatus] = useState<Status | null>(null);
  const [key, setKey] = useState('');
  const [model, setModel] = useState('claude-opus-5-5');
  const [effort, setEffort] = useState('medium');
  const [spaces, setSpaces] = useState<SpaceSummary[]>([]);
  const [space, setSpace] = useState('');
  const [thread, setThread] = useState<Msg[]>([]);
  const [input, setInput] = useState('');
  const [busy, setBusy] = useState(false);
  const [usage, setUsage] = useState('');
  const toast = useToast();
  const end = useRef<HTMLDivElement>(null);

  const load = () =>
    call<Status>('ai.status')
      .then(setStatus)
      .catch(() => setStatus(null));
  useEffect(() => {
    void load();
    call<SpaceSummary[]>('space.list')
      .then((l) => setSpaces(l.filter((s) => s.spec)))
      .catch(() => setSpaces([]));
  }, []);
  useEffect(() => end.current?.scrollIntoView({ block: 'end' }), [thread]);

  async function saveKey() {
    try {
      setStatus(await call<Status>('ai.key.set', { key: key.trim() }));
      setKey('');
      toast('API key saved in the VSN vault');
    } catch (e) {
      toast(String(e), 'danger');
    }
  }
  async function removeKey() {
    try {
      await call('ai.key.clear');
      await load();
      toast('API key removed');
    } catch (e) {
      toast(String(e), 'danger');
    }
  }
  async function send() {
    const text = input.trim();
    if (!text || busy) return;
    const next: Msg[] = [...thread, { role: 'user', content: text }];
    setThread(next);
    setInput('');
    setBusy(true);
    try {
      const r = await call<Reply>('ai.chat', { messages: next, model, effort, ...(space ? { space } : {}) });
      setThread([...next, { role: 'assistant', content: r.text || '(empty reply)' }]);
      setUsage(`${r.model} · ${r.input_tokens} in / ${r.output_tokens} out tokens`);
    } catch (e) {
      // Keep the question so the user can retry; drop it from the thread to keep roles alternating.
      setThread(thread);
      setInput(text);
      toast(String(e), 'danger');
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <section className="panel" aria-labelledby="ai-title">
        <div className="panelHead">
          <div>
            <span className="kicker">Your own Anthropic API key</span>
            <h3 id="ai-title">AI assistant</h3>
          </div>
          <Badge ok={!!status?.configured} label={status?.configured ? 'Key saved' : 'No key yet'} />
        </div>
        {!status?.configured ? (
          <>
            <label className="stacked">
              Anthropic API key
              <input
                type="password"
                autoComplete="off"
                placeholder="sk-ant-…"
                value={key}
                onChange={(e) => setKey(e.target.value)}
              />
            </label>
            <p className="hint">
              Create a key at console.anthropic.com → API keys. It is stored in the VSN vault on this PC and
              only sent to api.anthropic.com; usage is billed to your Anthropic account.
            </p>
            <button className="primary" disabled={!key.trim()} onClick={() => void saveKey()}>
              Save key
            </button>
          </>
        ) : (
          <div className="formRow">
            <label className="stacked">
              Model
              <select value={model} onChange={(e) => setModel(e.target.value)}>
                {(status.models ?? []).map((m) => (
                  <option key={m} value={m}>
                    {MODEL_LABEL[m] ?? m}
                  </option>
                ))}
              </select>
            </label>
            <label className="stacked">
              Effort
              <select value={effort} onChange={(e) => setEffort(e.target.value)}>
                <option value="low">Low (quick answers)</option>
                <option value="medium">Medium</option>
                <option value="high">High (harder problems)</option>
              </select>
            </label>
            <label className="stacked">
              Project context
              <select value={space} onChange={(e) => setSpace(e.target.value)}>
                <option value="">None</option>
                {spaces.map((s) => (
                  <option key={s.root} value={s.root}>
                    {s.spec?.name}
                  </option>
                ))}
              </select>
            </label>
            <button className="ghost" onClick={() => void removeKey()}>
              Remove key
            </button>
          </div>
        )}
      </section>
      {status?.configured && (
        <section className="panel" aria-labelledby="ai-chat">
          <div className="panelHead">
            <div>
              <span className="kicker">{usage || 'New conversation'}</span>
              <h3 id="ai-chat">Chat</h3>
            </div>
            <button className="ghost" disabled={!thread.length || busy} onClick={() => setThread([])}>
              New chat
            </button>
          </div>
          <div className="chatLog" aria-live="polite">
            {thread.length === 0 && (
              <p className="hint">
                Ask about your project: “Why does my Laravel app return 500?”, “Write a migration for orders”,
                “How do I deploy this space over SFTP?”. With a project selected, Claude knows its versions,
                services and ports (never your env values).
              </p>
            )}
            {thread.map((m, i) => (
              <div key={i} className={`chatMsg ${m.role}`}>
                <b>{m.role === 'user' ? 'You' : 'Claude'}</b>
                <pre>{m.content}</pre>
              </div>
            ))}
            {busy && <p className="hint">Claude is thinking…</p>}
            <div ref={end} />
          </div>
          <label className="stacked">
            Message
            <textarea
              rows={3}
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) void send();
              }}
            />
          </label>
          <div className="buttonRow">
            <button className="primary" disabled={!input.trim() || busy} onClick={() => void send()}>
              {busy ? 'Sending…' : 'Send (Ctrl+Enter)'}
            </button>
          </div>
        </section>
      )}
    </>
  );
}
