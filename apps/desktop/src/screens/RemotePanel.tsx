import { useEffect, useState } from 'react';
import { call } from '../api/agent';
import type { RemoteConfig } from '../contracts';
import { Badge } from '../components/ui';
import { ResultView } from '../components/ResultView';

export function RemotePanel({ config }: { config: RemoteConfig | null }) {
  const [url, setUrl] = useState(config?.control_plane_url ?? '');
  const [key, setKey] = useState(config?.control_plane_public_key ?? '');
  const [enabled, setEnabled] = useState(config?.enabled ?? false);
  const [allowTerminal, setAllowTerminal] = useState(config?.allow_remote_terminal ?? false);
  const [allowFileWrite, setAllowFileWrite] = useState(config?.allow_remote_file_write ?? false);
  const [allowDbQuery, setAllowDbQuery] = useState(config?.allow_remote_database_query ?? false);
  const [allowInteractivePreview, setAllowInteractivePreview] = useState(
    config?.allow_remote_preview_interactive ?? false,
  );
  const [nonce, setNonce] = useState('');
  const [result, setResult] = useState<any>(null);
  const [err, setErr] = useState('');
  useEffect(() => {
    if (config) {
      setUrl(config.control_plane_url ?? '');
      setKey(config.control_plane_public_key ?? '');
      setEnabled(config.enabled);
      setAllowTerminal(config.allow_remote_terminal ?? false);
      setAllowFileWrite(config.allow_remote_file_write ?? false);
      setAllowDbQuery(config.allow_remote_database_query ?? false);
      setAllowInteractivePreview(config.allow_remote_preview_interactive ?? false);
    }
  }, [config]);
  async function save() {
    setErr('');
    try {
      setResult(
        await call('remote.configure', {
          url,
          public_key: key,
          enabled,
          poll_interval_ms: 2500,
          allow_remote_terminal: allowTerminal,
          allow_remote_file_write: allowFileWrite,
          allow_remote_database_query: allowDbQuery,
          allow_remote_preview_interactive: allowInteractivePreview,
        }),
      );
    } catch (e) {
      setErr(String(e));
    }
  }
  async function enroll() {
    setErr('');
    try {
      setResult(await call('remote.enroll', { pairing_nonce: nonce }));
    } catch (e) {
      setErr(String(e));
    }
  }
  return (
    <>
      <section className="hero remoteHero">
        <div>
          <span className="kicker">Outbound only</span>
          <h2>{enabled ? 'Remote channel enabled' : 'Remote channel disabled'}</h2>
          <p>
            The Agent initiates the connection and verifies signed, short-lived, device-bound commands before
            local policy dispatch. Delivery retries are idempotent through the Agent result cache.
          </p>
        </div>
        <Badge ok={enabled} label={enabled ? 'Configured' : 'Local only'} />
      </section>
      <section className="panel">
        <div className="panelHead">
          <div>
            <span className="kicker">Machine pairing</span>
            <h3>Control Plane configuration</h3>
          </div>
        </div>
        <div className="formGrid">
          <label>
            Control Plane URL
            <input
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="https://control.example.com"
            />
          </label>
          <label>
            Control Plane public key
            <input
              value={key}
              onChange={(e) => setKey(e.target.value)}
              placeholder="Base64 Ed25519 public key"
            />
          </label>
          <label>
            Pairing nonce
            <input value={nonce} onChange={(e) => setNonce(e.target.value)} placeholder="pair_..." />
          </label>
          <label className="check">
            <input type="checkbox" checked={enabled} onChange={(e) => setEnabled(e.target.checked)} /> Enable
            outbound polling
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={allowTerminal}
              onChange={(e) => setAllowTerminal(e.target.checked)}
            />{' '}
            Allow scoped remote terminal commands
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={allowFileWrite}
              onChange={(e) => setAllowFileWrite(e.target.checked)}
            />{' '}
            Allow scoped remote file writes
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={allowDbQuery}
              onChange={(e) => setAllowDbQuery(e.target.checked)}
            />{' '}
            Allow scoped remote database queries
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={allowInteractivePreview}
              onChange={(e) => setAllowInteractivePreview(e.target.checked)}
            />{' '}
            Allow interactive remote preview WebSocket bridge
          </label>
        </div>
        <div className="buttonRow">
          <button className="primary" onClick={() => void save()}>
            Save remote config
          </button>
          <button className="ghost" disabled={!nonce} onClick={() => void enroll()}>
            Enroll device
          </button>
        </div>
        {err && <div className="error">{err}</div>}
        <ResultView value={result} />
      </section>
    </>
  );
}
