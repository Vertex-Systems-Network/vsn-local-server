import { useState, type ReactNode } from 'react';
import type { MachineIdentity } from '../contracts';
import { Tabs } from '../components/Tabs';
import { Switch } from '../components/Switch';
import { Badge } from '../components/ui';
import { WorkspaceRoots } from '../components/WorkspaceRoots';
import { applyTheme, readTheme, type ThemePref } from '../theme/theme';
import { readPrefs, savePrefs, SIDEBAR_EVENT, type Prefs } from '../theme/prefs';

const TABS = [
  ['appearance', 'Appearance'],
  ['roots', 'Workspace roots'],
  ['security', 'Security'],
  ['about', 'About'],
] as const;
type Tab = (typeof TABS)[number][0];

function Choices<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: [T, string, string][];
  onChange: (v: T) => void;
}) {
  const id = `choice-${label.toLowerCase()}`;
  return (
    <div className="choiceBlock">
      <h3 id={id}>{label}</h3>
      <div className="choiceGrid" role="radiogroup" aria-labelledby={id}>
        {options.map(([v, title, hint]) => (
          <button key={v} type="button" role="radio" aria-checked={value === v} className="choice" onClick={() => onChange(v)}>
            {title}
            <small>{hint}</small>
          </button>
        ))}
      </div>
    </div>
  );
}

function Toggle({ title, hint, checked, onChange }: { title: string; hint: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <div className="settingRow">
      <div>
        <b>{title}</b>
        <p>{hint}</p>
      </div>
      <Switch checked={checked} onChange={onChange} label={title} />
    </div>
  );
}

function Facts({ rows }: { rows: [string, ReactNode][] }) {
  return (
    <dl className="facts">
      {rows.map(([k, v]) => (
        <div key={k}>
          <dt>{k}</dt>
          <dd>{v}</dd>
        </div>
      ))}
    </dl>
  );
}

export function Settings({ machine, status }: { machine: MachineIdentity | null; status: any }) {
  const [tab, setTab] = useState<Tab>('appearance');
  const [theme, setTheme] = useState<ThemePref>(readTheme);
  const [prefs, setPrefs] = useState<Prefs>(readPrefs);
  const update = (patch: Partial<Prefs>) => {
    const next = { ...prefs, ...patch };
    setPrefs(next);
    savePrefs(next);
    if (patch.sidebar) window.dispatchEvent(new CustomEvent(SIDEBAR_EVENT, { detail: patch.sidebar === 'collapsed' }));
  };
  const sec = status?.security ?? {};
  return (
    <>
      <Tabs tabs={TABS} value={tab} onChange={setTab} label="Settings" panelId="settings-panel" />
      <div role="tabpanel" id="settings-panel" aria-labelledby={`settings-panel-tab-${tab}`}>
        {tab === 'appearance' && (
          <>
            <Choices<ThemePref>
              label="Theme"
              value={theme}
              onChange={(t) => {
                setTheme(t);
                applyTheme(t);
              }}
              options={[
                ['system', 'System', 'Follow Windows light/dark setting'],
                ['light', 'Light', 'Mist surfaces, indigo accent'],
                ['dark', 'Dark', 'Night surfaces, teal accent'],
              ]}
            />
            <div className="settingsCols">
              <Choices
                label="Density"
                value={prefs.density}
                onChange={(density) => update({ density })}
                options={[
                  ['comfortable', 'Comfortable', 'Default spacing'],
                  ['compact', 'Compact', 'More rows per screen'],
                ]}
              />
              <Choices
                label="Sidebar"
                value={prefs.sidebar}
                onChange={(sidebar) => update({ sidebar })}
                options={[
                  ['expanded', 'Expanded', 'Icons + labels'],
                  ['collapsed', 'Collapsed', 'Icons only'],
                ]}
              />
            </div>
            <section className="panel">
              <div className="panelHead">
                <h3>Accessibility</h3>
              </div>
              <Toggle
                title="Reduced motion"
                hint="Disable animations and transitions"
                checked={prefs.reduceMotion}
                onChange={(reduceMotion) => update({ reduceMotion })}
              />
              <Toggle
                title="High contrast borders"
                hint="Stronger outlines on cards and inputs"
                checked={prefs.highContrast}
                onChange={(highContrast) => update({ highContrast })}
              />
              <Toggle
                title="Show keyboard shortcuts"
                hint="Display shortcut hints in menus"
                checked={prefs.shortcutHints}
                onChange={(shortcutHints) => update({ shortcutHints })}
              />
            </section>
          </>
        )}
        {tab === 'roots' && <WorkspaceRoots />}
        {tab === 'security' && (
          <div className="settingsCols">
            <section className="panel">
              <div className="panelHead">
                <h3>Agent connection</h3>
                <Badge ok={!!sec.ipc_secret_ready} label={sec.ipc_secret_ready ? 'Authenticated' : 'Not authenticated'} />
              </div>
              <Facts
                rows={[
                  ['Authentication', sec.ipc_secret_ready ? 'Per-session token' : '—'],
                  ['Agent health', status?.health?.detail ?? '—'],
                  ['Allowed clients', 'Desktop, CLI'],
                ]}
              />
            </section>
            <section className="panel">
              <div className="panelHead">
                <h3>Device identity</h3>
                <Badge ok={!!sec.device_identity_ready} label={sec.device_identity_ready ? 'Ready' : 'Not ready'} />
              </div>
              <Facts
                rows={[
                  ['Secure store', sec.secure_store ?? '—'],
                  ['Device ID', machine?.device_id ? <code>{machine.device_id}</code> : '—'],
                  ['Operating system', machine?.os ?? '—'],
                ]}
              />
            </section>
          </div>
        )}
        {tab === 'about' && (
          <section className="panel">
            <div className="panelHead">
              <div>
                <span className="kicker">VSN Dev Platform</span>
                <h3>About</h3>
              </div>
            </div>
            <Facts
              rows={[
                ['Version', __APP_VERSION__],
                ['Build', 'Personal (unsigned)'],
              ]}
            />
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
