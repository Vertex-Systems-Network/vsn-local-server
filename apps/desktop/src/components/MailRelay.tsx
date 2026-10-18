import { useEffect, useState } from 'react';
import { call } from '../api/agent';
import { Badge } from './ui';
import { useToast } from './Toast';

export type MailRelaySettings = {
  host: string;
  port: number;
  security: 'starttls' | 'tls' | 'none';
  username: string;
  override_from?: string | null;
  relay_all: boolean;
  has_password?: boolean;
};

const PRESETS: Record<string, Partial<MailRelaySettings>> = {
  gmail: { host: 'smtp.gmail.com', port: 587, security: 'starttls' },
  outlook: { host: 'smtp.office365.com', port: 587, security: 'starttls' },
  custom: {},
};
const EMPTY: MailRelaySettings = {
  host: '',
  port: 587,
  security: 'starttls',
  username: '',
  relay_all: false,
};

/** Outgoing SMTP (Gmail or any server) so captured emails can also be sent for real. */
export function MailRelayPanel({ root }: { root: string }) {
  const [relay, setRelay] = useState<MailRelaySettings>(EMPTY);
  const [saved, setSaved] = useState<MailRelaySettings | null>(null);
  const [password, setPassword] = useState('');
  const [busy, setBusy] = useState(false);
  const toast = useToast();
  useEffect(() => {
    call<MailRelaySettings | null>('space.mail.relay.get', { path: root })
      .then((r) => {
        setSaved(r);
        setRelay(r ?? EMPTY);
      })
      .catch(() => setSaved(null));
  }, [root]);
  const set = (patch: Partial<MailRelaySettings>) => setRelay((r) => ({ ...r, ...patch }));

  async function save() {
    setBusy(true);
    try {
      const r = await call<MailRelaySettings>('space.mail.relay.set', {
        path: root,
        relay: { ...relay, override_from: relay.override_from || null },
        ...(password ? { password } : {}),
      });
      setSaved(r);
      setPassword('');
      toast('Outgoing SMTP saved. Restart the mailpit service in Spaces to apply.');
    } catch (e) {
      toast(String(e), 'danger');
    } finally {
      setBusy(false);
    }
  }
  async function clear() {
    try {
      await call('space.mail.relay.clear', { path: root });
      setSaved(null);
      setRelay(EMPTY);
      toast('Outgoing SMTP removed');
    } catch (e) {
      toast(String(e), 'danger');
    }
  }

  return (
    <section className="panel" aria-labelledby="mail-relay">
      <div className="panelHead">
        <div>
          <span className="kicker">Send for real</span>
          <h3 id="mail-relay">Outgoing SMTP</h3>
        </div>
        <Badge ok={!!saved} label={saved ? `${saved.host}:${saved.port}` : 'Not set'} />
      </div>
      <div className="formRow">
        <label className="stacked">
          Provider
          <select
            defaultValue=""
            onChange={(e) => e.target.value && set(PRESETS[e.target.value] ?? {})}
            aria-label="SMTP provider preset"
          >
            <option value="">Choose a preset…</option>
            <option value="gmail">Gmail</option>
            <option value="outlook">Outlook / Microsoft 365</option>
            <option value="custom">Other SMTP server</option>
          </select>
        </label>
        <label className="stacked">
          SMTP host
          <input value={relay.host} onChange={(e) => set({ host: e.target.value.trim() })} />
        </label>
        <label className="stacked">
          Port
          <input
            type="number"
            min={1}
            max={65535}
            value={relay.port}
            onChange={(e) => set({ port: Number(e.target.value) })}
          />
        </label>
        <label className="stacked">
          Security
          <select
            value={relay.security}
            onChange={(e) => set({ security: e.target.value as MailRelaySettings['security'] })}
          >
            <option value="starttls">STARTTLS (587)</option>
            <option value="tls">SSL/TLS (465)</option>
            <option value="none">None</option>
          </select>
        </label>
      </div>
      <div className="formRow">
        <label className="stacked">
          Username / email
          <input
            value={relay.username}
            autoComplete="off"
            onChange={(e) => set({ username: e.target.value.trim() })}
          />
        </label>
        <label className="stacked">
          Password {saved?.has_password ? '(saved — leave empty to keep)' : ''}
          <input
            type="password"
            autoComplete="new-password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </label>
        <label className="stacked">
          Send as (optional)
          <input
            value={relay.override_from ?? ''}
            placeholder="you@gmail.com"
            onChange={(e) => set({ override_from: e.target.value.trim() })}
          />
        </label>
      </div>
      <label className="checkRow">
        <input
          type="checkbox"
          checked={relay.relay_all}
          onChange={(e) => set({ relay_all: e.target.checked })}
        />
        Send every captured email for real (otherwise use “Send for real” on one email)
      </label>
      <p className="hint">
        Gmail: turn on 2-Step Verification, create an <b>App password</b> (Google Account → Security → App
        passwords) and use it here, not your normal password. Settings are kept in VSN&apos;s data folder, not
        in the project.
      </p>
      <div className="buttonRow">
        <button className="primary" onClick={() => void save()} disabled={busy || !relay.host}>
          {busy ? 'Saving…' : 'Save'}
        </button>
        {saved && (
          <button className="ghost" onClick={() => void clear()}>
            Remove
          </button>
        )}
      </div>
    </section>
  );
}
