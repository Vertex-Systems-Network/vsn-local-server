import { useState } from 'react';
import { call, openUrl } from '../api/agent';
import { Badge } from '../components/ui';
import { Logo } from '../components/Logo';
import { useToast } from '../components/Toast';

const WEBSITE = 'https://vertexsystemsnetwork.com';
const RELEASES = 'https://github.com/Vertex-Systems-Network/vsn-local-server/releases';
// Donation page (Patreon membership); change here if the link moves.
export const DONATE = 'https://www.patreon.com/cw/VertexSystemsNetwork/membership';

type Release = { tag: string | null; name?: string | null; url: string; published_at?: string | null };

/** Compares "v1.2.3" style tags; true when `tag` is newer than the running version. */
export function isNewer(tag: string, current: string) {
  const parse = (v: string) =>
    v
      .replace(/^v/i, '')
      .split(/[.-]/)
      .slice(0, 3)
      .map((n) => Number(n) || 0);
  const [a, b] = [parse(tag), parse(current)];
  for (let i = 0; i < 3; i++) if ((a[i] ?? 0) !== (b[i] ?? 0)) return (a[i] ?? 0) > (b[i] ?? 0);
  return false;
}

export function About() {
  const [release, setRelease] = useState<Release | null>(null);
  const [checking, setChecking] = useState(false);
  const [err, setErr] = useState('');
  const toast = useToast();
  const open = (url: string) => openUrl(url).catch((e) => toast(String(e), 'danger'));

  async function check() {
    setChecking(true);
    setErr('');
    try {
      setRelease(await call<Release>('app.update.check'));
    } catch (e) {
      setErr(`Could not reach GitHub: ${String(e)}`);
    } finally {
      setChecking(false);
    }
  }
  const newer = release?.tag ? isNewer(release.tag, __APP_VERSION__) : false;

  return (
    <>
      <section className="panel aboutHero" aria-labelledby="about-title">
        <Logo size={72} />
        <div>
          <span className="kicker">Vertex Systems Network</span>
          <h3 id="about-title">VSN Devify</h3>
          <p className="hint">
            A local-first development platform: isolated project spaces, every runtime and framework,
            databases, mail and API testing, sharing and remote access, all from one app on your own machine.
          </p>
          <div className="heroBadges">
            <Badge ok label={`Version ${__APP_VERSION__}`} />
            <Badge ok={false} label="Personal build (unsigned)" />
          </div>
        </div>
      </section>
      <div className="overviewGrid">
        <section className="panel" aria-labelledby="about-updates">
          <div className="panelHead">
            <div>
              <span className="kicker">Updates</span>
              <h3 id="about-updates">Check for updates</h3>
            </div>
            {release && (
              <Badge
                ok={!newer}
                label={
                  newer ? `${release.tag} available` : release.tag ? 'Up to date' : 'No public release yet'
                }
              />
            )}
          </div>
          <p className="hint">
            You are on <b>v{__APP_VERSION__}</b>. {release?.tag && `Latest public release: ${release.tag}.`}
          </p>
          {err && <div className="error">{err}</div>}
          <div className="buttonRow">
            <button onClick={() => void check()} disabled={checking}>
              {checking ? 'Checking…' : 'Check now'}
            </button>
            <button className="ghost" onClick={() => void open(release?.url ?? RELEASES)}>
              {newer ? 'Download update' : 'All releases'}
            </button>
          </div>
        </section>
        <section className="panel" aria-labelledby="about-company">
          <div className="panelHead">
            <div>
              <span className="kicker">Company</span>
              <h3 id="about-company">Vertex Systems Network</h3>
            </div>
          </div>
          <p className="hint">
            VSN builds developer tools that keep your code, data and keys on your own machine. Devify is free
            to use for personal work; donations keep development going.
          </p>
          <div className="buttonRow">
            <button className="ghost" onClick={() => void open(WEBSITE)}>
              Website
            </button>
            <button onClick={() => void open(DONATE)}>Donate</button>
          </div>
        </section>
      </div>
      <section className="panel" aria-labelledby="about-licenses">
        <div className="panelHead">
          <h3 id="about-licenses">Third-party software</h3>
        </div>
        <ul className="licenseList">
          <li>GSAP 3.12.7 — GreenSock standard license</li>
          <li>qrcode-generator 2.0.4 — MIT</li>
          <li>Plus Jakarta Sans, Bricolage Grotesque — SIL Open Font License 1.1</li>
          <li>React, Tauri — MIT / Apache-2.0</li>
        </ul>
      </section>
    </>
  );
}
