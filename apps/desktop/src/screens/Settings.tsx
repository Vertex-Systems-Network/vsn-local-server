import { useState } from 'react';
import { Tabs } from '../components/Tabs';
import { Badge } from '../components/ui';
import { reducedMotion } from '../motion';
import { applyTheme, readTheme, type ThemePref } from '../theme/theme';

const TABS = [
  ['general', 'General'],
  ['updates', 'Updates'],
  ['about', 'About'],
] as const;
type Tab = (typeof TABS)[number][0];
const THEMES: [ThemePref, string][] = [
  ['system', 'Match system'],
  ['light', 'Light'],
  ['dark', 'Dark'],
];
const REPO = 'https://github.com/Vertex-Systems-Network/vsn-local-server';

export function Settings() {
  const [tab, setTab] = useState<Tab>('general');
  const [theme, setTheme] = useState<ThemePref>(readTheme);
  const choose = (t: ThemePref) => {
    setTheme(t);
    applyTheme(t);
  };
  return (
    <>
      <Tabs tabs={TABS} value={tab} onChange={setTab} label="Settings" panelId="settings-panel" />
      <div role="tabpanel" id="settings-panel" aria-labelledby={`settings-panel-tab-${tab}`}>
        {tab === 'general' && (
          <section className="panel">
            <div className="panelHead">
              <div>
                <span className="kicker">Appearance</span>
                <h3>General</h3>
              </div>
            </div>
            <div className="settingRow">
              <div>
                <b id="theme-label">Theme</b>
                <p>Light and Dark use the VSN brand colours. Match system follows Windows.</p>
              </div>
              <div className="choiceRow" role="radiogroup" aria-labelledby="theme-label">
                {THEMES.map(([id, label]) => (
                  <button
                    key={id}
                    type="button"
                    role="radio"
                    aria-checked={theme === id}
                    className="choice"
                    onClick={() => choose(id)}
                  >
                    {label}
                  </button>
                ))}
              </div>
            </div>
            <div className="settingRow">
              <div>
                <b>Animations</b>
                <p>Follows the Windows “Show animations” setting (prefers-reduced-motion).</p>
              </div>
              <Badge ok={!reducedMotion()} label={reducedMotion() ? 'Reduced' : 'On'} />
            </div>
          </section>
        )}
        {tab === 'updates' && (
          <section className="panel">
            <div className="panelHead">
              <div>
                <span className="kicker">Personal channel</span>
                <h3>Updates</h3>
              </div>
              <Badge ok label={`v${__APP_VERSION__}`} />
            </div>
            <p>
              This is a personal, unsigned build. To update, run the <b>Personal Windows Build (unsigned)</b> workflow
              and install the new installer over this one; your data is kept.
            </p>
            <p className="hint">
              Automatic updates arrive with the signed public release (PKG-03 03.22–03.25). Guide:{' '}
              <code>docs/PERSONAL-INSTALL-WINDOWS.md</code> in <code>{REPO.replace('https://', '')}</code>.
            </p>
          </section>
        )}
        {tab === 'about' && (
          <section className="panel">
            <div className="panelHead">
              <div>
                <span className="kicker">VSN Dev Platform</span>
                <h3>About</h3>
              </div>
            </div>
            <dl className="facts">
              <div>
                <dt>Version</dt>
                <dd>{__APP_VERSION__}</dd>
              </div>
              <div>
                <dt>Build</dt>
                <dd>Personal (unsigned)</dd>
              </div>
            </dl>
            <h4>Third-party software</h4>
            <ul className="licenseList">
              <li>GSAP 3.12.7 — GreenSock standard license</li>
              <li>Plus Jakarta Sans, Bricolage Grotesque — SIL Open Font License 1.1</li>
              <li>React, Tauri — MIT / Apache-2.0</li>
            </ul>
          </section>
        )}
      </div>
    </>
  );
}
