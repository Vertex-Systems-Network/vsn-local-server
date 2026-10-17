import { useEffect, useState } from 'react';
import { call } from '../api/agent';
import { Badge } from './ui';
import { Dialog } from './Dialog';
import { useToast } from './Toast';

type VaultStatus = { format_version: number; key_id: string; entries: number };
type AuditResult = { valid: boolean; events?: number; error?: string };

/** Secrets vault status + master-key rotation (Figma: Settings → Security → Secrets vault). */
export function VaultPanel() {
  const [vault, setVault] = useState<VaultStatus | null>(null);
  const [err, setErr] = useState('');
  const [confirm, setConfirm] = useState(false);
  const toast = useToast();
  const load = () =>
    call<VaultStatus>('vault.status')
      .then((v) => {
        setVault(v);
        setErr('');
      })
      .catch((e) => setErr(String(e)));
  useEffect(() => {
    void load();
  }, []);
  async function rotate() {
    setConfirm(false);
    try {
      const r = await call<{ entries_reencrypted: number }>('vault.rotate');
      toast(`Vault key rotated · ${r.entries_reencrypted} secret(s) re-encrypted`);
      await load();
    } catch (e) {
      toast(String(e), 'danger');
    }
  }
  return (
    <section className="panel" aria-labelledby="vault-title">
      <div className="panelHead">
        <h3 id="vault-title">Secrets vault</h3>
        <button className="ghost" onClick={() => setConfirm(true)} disabled={!vault}>
          Rotate key
        </button>
      </div>
      {vault ? (
        <dl className="facts kvList">
          <div>
            <dt>Key ID</dt>
            <dd>
              <code>{vault.key_id}</code>
            </dd>
          </div>
          <div>
            <dt>Secrets stored</dt>
            <dd>{vault.entries}</dd>
          </div>
          <div>
            <dt>Format</dt>
            <dd>v{vault.format_version}</dd>
          </div>
        </dl>
      ) : (
        <p className="hint">{err || 'Loading…'}</p>
      )}
      {confirm && (
        <Dialog title="Rotate the vault key?" onClose={() => setConfirm(false)}>
          <p>Every stored secret is re-encrypted with a new master key. The old key is kept for recovery.</p>
          <div className="buttonRow">
            <button className="ghost" onClick={() => setConfirm(false)}>
              Cancel
            </button>
            <button className="primary" onClick={() => void rotate()}>
              Rotate key
            </button>
          </div>
        </Dialog>
      )}
    </section>
  );
}

/** Tamper check of the signed local audit log (Figma: Settings → Security → Audit log). */
export function AuditPanel() {
  const [audit, setAudit] = useState<AuditResult | null>(null);
  const [busy, setBusy] = useState(false);
  async function verify() {
    setBusy(true);
    try {
      setAudit(await call<AuditResult>('audit.verify'));
    } catch (e) {
      setAudit({ valid: false, error: String(e) });
    } finally {
      setBusy(false);
    }
  }
  useEffect(() => {
    void verify();
  }, []);
  return (
    <section className="panel" aria-labelledby="audit-title">
      <div className="panelHead">
        <h3 id="audit-title">Audit log</h3>
        <button className="ghost" onClick={() => void verify()} disabled={busy}>
          {busy ? 'Verifying…' : 'Verify'}
        </button>
      </div>
      {audit && (
        <div className="serviceState">
          <Badge ok={audit.valid} label={audit.valid ? 'Signature chain intact' : 'Verification failed'} />
          <span>{audit.valid ? `${audit.events ?? 0} signed event(s)` : audit.error}</span>
        </div>
      )}
    </section>
  );
}
