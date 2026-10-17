import { useEffect, useState } from 'react';
import { call } from '../api/agent';
import { QrCode } from './QrCode';
import { useToast } from './Toast';

type Tunnel = { port: number; url: string | null };
type Share = {
  id: number;
  target_port: number;
  listen_port: number;
  url: string | null;
  lan_ip: string | null;
  expires_at_unix: number;
};

/** Open a local dev server on the Wi-Fi/LAN with a QR code for phones (DVF-06). */
export function LanShare() {
  const [shares, setShares] = useState<Share[]>([]);
  const [port, setPort] = useState('8000');
  const [minutes, setMinutes] = useState('120');
  const [busy, setBusy] = useState(false);
  const [tunnel, setTunnel] = useState<Tunnel | null>(null);
  const toast = useToast();
  // A quick tunnel can take a little longer than the start call waits; poll for the link.
  useEffect(() => {
    if (!tunnel || tunnel.url) return;
    let tries = 0;
    const timer = window.setInterval(() => {
      tries += 1;
      call<Tunnel>('share.tunnel.status', { port: tunnel.port })
        .then((t) => t.url && setTunnel(t))
        .catch(() => undefined);
      if (tries >= 20) window.clearInterval(timer);
    }, 3000);
    return () => window.clearInterval(timer);
  }, [tunnel]);
  async function startTunnel() {
    setBusy(true);
    try {
      const t = await call<Tunnel>('share.tunnel.start', { port: Number(port) });
      setTunnel(t);
      toast(t.url ? `Public link ready: ${t.url}` : 'Creating the public link…');
    } catch (e) {
      toast(String(e), 'danger');
    } finally {
      setBusy(false);
    }
  }
  async function stopTunnel() {
    if (!tunnel) return;
    try {
      await call('share.tunnel.stop', { port: tunnel.port });
      setTunnel(null);
    } catch (e) {
      toast(String(e), 'danger');
    }
  }
  const load = () =>
    call<Share[]>('share.list')
      .then(setShares)
      .catch(() => setShares([]));
  useEffect(() => {
    void load();
  }, []);
  async function start() {
    setBusy(true);
    try {
      const s = await call<Share>('share.lan.start', { port: Number(port), minutes: Number(minutes) });
      toast(s.url ? `Shared on ${s.url}` : `Shared on port ${s.listen_port}`);
      await load();
    } catch (e) {
      toast(String(e), 'danger');
    } finally {
      setBusy(false);
    }
  }
  async function stop(id: number) {
    try {
      await call('share.lan.stop', { id });
      await load();
    } catch (e) {
      toast(String(e), 'danger');
    }
  }
  return (
    <section className="panel" aria-labelledby="share-title">
      <div className="panelHead">
        <div>
          <span className="kicker">Phones and other computers</span>
          <h3 id="share-title">Share this app</h3>
        </div>
      </div>
      <p className="hint">
        Anyone on the same Wi-Fi or LAN can open the link while the share is on. It turns itself off when the
        time runs out. Windows may ask once to allow VSN through the firewall.
      </p>
      <div className="formGrid">
        <label>
          Local port of the running app
          <input
            inputMode="numeric"
            value={port}
            onChange={(e) => setPort(e.target.value.replace(/\D/g, ''))}
          />
        </label>
        <label>
          Turn off after
          <select value={minutes} onChange={(e) => setMinutes(e.target.value)}>
            <option value="30">30 minutes</option>
            <option value="120">2 hours</option>
            <option value="480">8 hours</option>
            <option value="1440">24 hours</option>
          </select>
        </label>
      </div>
      <div className="buttonRow">
        <button className="primary" disabled={!port || busy} onClick={() => void start()}>
          Share on Wi-Fi / LAN
        </button>
        <button className="ghost" disabled={!port || busy || !!tunnel} onClick={() => void startTunnel()}>
          Create internet link
        </button>
      </div>
      {shares.map((s) => (
        <div className="shareCard" key={s.id}>
          {s.url && <QrCode value={s.url} label={`QR code for ${s.url}`} />}
          <div>
            <b>{s.url ?? `Port ${s.listen_port} (no network address found)`}</b>
            <p className="hint">
              Forwards to 127.0.0.1:{s.target_port} · turns off at{' '}
              {new Date(s.expires_at_unix * 1000).toLocaleTimeString()}
            </p>
            <div className="buttonRow">
              {s.url && (
                <button
                  className="ghost"
                  onClick={() =>
                    void navigator.clipboard?.writeText(s.url ?? '').then(() => toast('Link copied'))
                  }
                >
                  Copy link
                </button>
              )}
              <button className="danger" onClick={() => void stop(s.id)}>
                Stop sharing
              </button>
            </div>
          </div>
        </div>
      ))}
      {tunnel && (
        <div className="shareCard">
          {tunnel.url && <QrCode value={tunnel.url} label={`QR code for ${tunnel.url}`} />}
          <div>
            <b>{tunnel.url ?? 'Creating a public HTTPS link…'}</b>
            <p className="hint">
              Public internet link through a free Cloudflare quick tunnel to 127.0.0.1:{tunnel.port}. Anyone
              with the link can open it; stop it when you are done. Needs cloudflared installed.
            </p>
            <div className="buttonRow">
              {tunnel.url && (
                <button
                  className="ghost"
                  onClick={() =>
                    void navigator.clipboard?.writeText(tunnel.url ?? '').then(() => toast('Link copied'))
                  }
                >
                  Copy link
                </button>
              )}
              <button className="danger" onClick={() => void stopTunnel()}>
                Stop internet link
              </button>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}
