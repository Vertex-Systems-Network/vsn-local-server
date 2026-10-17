import { useEffect, useMemo, useState } from 'react';
import { call } from '../api/agent';
import type { SpaceSummary } from '../contracts';
import { Badge } from '../components/ui';
import { DataTable } from '../components/DataTable';
import { Tabs } from '../components/Tabs';
import { useToast } from '../components/Toast';

type Addr = { Name: string; Address: string };
type Summary = {
  ID: string;
  From: Addr | null;
  To: Addr[] | null;
  Subject: string;
  Created: string;
  Snippet: string;
  Read: boolean;
  Size: number;
};
type Message = {
  ID: string;
  From: Addr | null;
  To: Addr[] | null;
  Cc: Addr[] | null;
  Subject: string;
  Date: string;
  Text: string;
  HTML: string;
};
type Preview = { status: number; text: string | null };

const VIEWS = [
  ['html', 'HTML'],
  ['text', 'Text'],
  ['headers', 'Headers'],
] as const;
type View = (typeof VIEWS)[number][0];

const addr = (a: Addr | null | undefined) => (a ? (a.Name ? `${a.Name} <${a.Address}>` : a.Address) : '—');

/** Mailpit's web/API port is the space's SMTP port + 1 (see vsn-space service_plan). */
async function mailpit<T>(apiPort: number, path: string): Promise<T> {
  const r = await call<Preview>('preview.fetch', { port: apiPort, path, method: 'GET' });
  if (r.status >= 400 || r.text == null) throw new Error(`Mailpit returned HTTP ${r.status}`);
  return JSON.parse(r.text) as T;
}

/** Local email testing (DVF-05): every email a space's app sends is caught here, nothing leaves the PC. */
export function Mail() {
  const [spaces, setSpaces] = useState<SpaceSummary[]>([]);
  const [root, setRoot] = useState('');
  const [messages, setMessages] = useState<Summary[]>([]);
  const [selected, setSelected] = useState<Message | null>(null);
  const [view, setView] = useState<View>('html');
  const [err, setErr] = useState('');
  const toast = useToast();
  const withMail = useMemo(
    () => spaces.filter((s) => s.spec?.services.some((sv) => sv.kind === 'mailpit')),
    [spaces],
  );
  const space = withMail.find((s) => s.root === root) ?? withMail[0];
  const smtpPort = space?.spec?.services.find((sv) => sv.kind === 'mailpit')?.port;
  const apiPort = smtpPort ? smtpPort + 1 : undefined;

  useEffect(() => {
    call<SpaceSummary[]>('space.list')
      .then(setSpaces)
      .catch((e) => setErr(String(e)));
  }, []);

  async function load() {
    if (!apiPort) return;
    try {
      const r = await mailpit<{ messages: Summary[] }>(apiPort, '/api/v1/messages?limit=100');
      setMessages(r.messages ?? []);
      setErr('');
    } catch (e) {
      setMessages([]);
      setErr(`${String(e)}. Start the space's mailpit service in Spaces.`);
    }
  }
  useEffect(() => {
    setSelected(null);
    void load();
  }, [apiPort]);

  async function open(id: string) {
    if (!apiPort) return;
    try {
      setSelected(await mailpit<Message>(apiPort, `/api/v1/message/${encodeURIComponent(id)}`));
      setView('html');
    } catch (e) {
      toast(String(e), 'danger');
    }
  }
  async function clearAll() {
    if (!apiPort) return;
    try {
      await call('preview.request', { port: apiPort, path: '/api/v1/messages', method: 'DELETE' });
      setSelected(null);
      await load();
      toast('Inbox cleared');
    } catch (e) {
      toast(String(e), 'danger');
    }
  }

  if (!withMail.length)
    return (
      <section className="panel" aria-labelledby="mail-title">
        <div className="panelHead">
          <h3 id="mail-title">Mail</h3>
        </div>
        <div className="empty">
          {err || 'No space has a mail service yet. Add “mailpit” to a space in Spaces, then start it.'}
        </div>
      </section>
    );

  return (
    <>
      <section className="panel" aria-labelledby="mail-title">
        <div className="panelHead">
          <div>
            <span className="kicker">Local email testing</span>
            <h3 id="mail-title">Inbox</h3>
          </div>
          <div className="buttonRow">
            <button className="ghost" onClick={() => void load()}>
              Refresh
            </button>
            <button className="danger" onClick={() => void clearAll()} disabled={!messages.length}>
              Clear inbox
            </button>
          </div>
        </div>
        <div className="formRow">
          <label className="stacked">
            Space
            <select value={space?.root ?? ''} onChange={(e) => setRoot(e.target.value)}>
              {withMail.map((s) => (
                <option key={s.root} value={s.root}>
                  {s.spec?.name}
                </option>
              ))}
            </select>
          </label>
        </div>
        <p className="hint">
          Point the app&apos;s mailer at SMTP host <code>127.0.0.1</code>, port <code>{smtpPort}</code>, no
          TLS, no password. Every email it sends lands here instead of a real inbox.
        </p>
        <DataTable
          caption="Captured emails"
          rows={messages}
          rowKey={(m) => m.ID}
          empty={err || 'No emails yet.'}
          columns={[
            {
              key: 'subject',
              label: 'Subject',
              render: (m) => (
                <button className="linkButton" onClick={() => void open(m.ID)}>
                  {m.Read ? m.Subject || '(no subject)' : <b>{m.Subject || '(no subject)'}</b>}
                </button>
              ),
            },
            { key: 'from', label: 'From', className: 'hideSm', render: (m) => addr(m.From) },
            {
              key: 'to',
              label: 'To',
              className: 'hideSm',
              render: (m) => (m.To ?? []).map(addr).join(', ') || '—',
            },
            { key: 'date', label: 'Received', render: (m) => new Date(m.Created).toLocaleString() },
          ]}
        />
      </section>
      {selected && (
        <section className="panel" aria-labelledby="mail-message">
          <div className="panelHead">
            <div>
              <span className="kicker">{addr(selected.From)}</span>
              <h3 id="mail-message">{selected.Subject || '(no subject)'}</h3>
            </div>
            <Badge ok label={new Date(selected.Date).toLocaleString()} />
          </div>
          <Tabs tabs={VIEWS} value={view} onChange={setView} label="Message view" panelId="mail-view" />
          <div role="tabpanel" id="mail-view" aria-labelledby={`mail-view-tab-${view}`}>
            {view === 'html' &&
              (selected.HTML ? (
                <iframe className="mailFrame" title="Email HTML preview" sandbox="" srcDoc={selected.HTML} />
              ) : (
                <p className="hint">This email has no HTML part.</p>
              ))}
            {view === 'text' && <pre className="json">{selected.Text || '(no text part)'}</pre>}
            {view === 'headers' && (
              <dl className="facts kvList">
                <div>
                  <dt>From</dt>
                  <dd>{addr(selected.From)}</dd>
                </div>
                <div>
                  <dt>To</dt>
                  <dd>{(selected.To ?? []).map(addr).join(', ') || '—'}</dd>
                </div>
                <div>
                  <dt>Cc</dt>
                  <dd>{(selected.Cc ?? []).map(addr).join(', ') || '—'}</dd>
                </div>
                <div>
                  <dt>Date</dt>
                  <dd>{selected.Date}</dd>
                </div>
              </dl>
            )}
          </div>
          <p className="hint">Scripts never run in previews and remote images are blocked.</p>
        </section>
      )}
    </>
  );
}
