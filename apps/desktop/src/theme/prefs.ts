// Client-side appearance and accessibility preferences (Settings → Appearance).
// Stored per machine in localStorage and applied as attributes on <html>.
export type Prefs = {
  density: 'comfortable' | 'compact';
  sidebar: 'expanded' | 'collapsed';
  motion: 'on' | 'system' | 'off';
  highContrast: boolean;
  shortcutHints: boolean;
  /** Space address template, like Laragon's hostname setting: {name}, {port}, {domain}. */
  urlPattern: string;
};
export const DEFAULT_PREFS: Prefs = {
  density: 'comfortable',
  sidebar: 'expanded',
  motion: 'on',
  highContrast: false,
  shortcutHints: true,
  urlPattern: 'http://{name}.localhost:{port}',
};
const KEY = 'vsn.prefs';
export const SIDEBAR_EVENT = 'vsn:sidebar';

export function readPrefs(): Prefs {
  const prefs = { ...DEFAULT_PREFS };
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) {
      const saved = JSON.parse(raw) as Partial<Prefs> & { reduceMotion?: boolean };
      Object.assign(prefs, saved);
      // Earlier builds stored a reduced-motion switch.
      if (saved.reduceMotion && !saved.motion) prefs.motion = 'off';
      delete (prefs as { reduceMotion?: boolean }).reduceMotion;
    }
    // The sidebar rail toggle is the source of truth for the sidebar preference.
    const rail = localStorage.getItem('vsn.sidebar.collapsed');
    if (rail !== null) prefs.sidebar = rail === '1' ? 'collapsed' : 'expanded';
  } catch {
    // Unavailable or corrupt storage falls back to defaults.
  }
  return prefs;
}

export function applyPrefs(p: Prefs) {
  const root = document.documentElement;
  root.dataset.density = p.density;
  root.toggleAttribute('data-contrast-high', p.highContrast);
  root.dataset.motion = p.motion;
  root.toggleAttribute('data-hide-shortcuts', !p.shortcutHints);
}

export function savePrefs(p: Prefs) {
  try {
    localStorage.setItem(KEY, JSON.stringify(p));
  } catch {
    // Preference persistence is best-effort.
  }
  applyPrefs(p);
}

/** Fill a URL pattern for a space. `{name}` is made DNS-safe. */
export function spaceUrl(pattern: string, name: string, port: number, domain?: string) {
  const host =
    name
      .toLowerCase()
      .replace(/[^a-z0-9-]+/g, '-')
      .replace(/^-+|-+$/g, '')
      .slice(0, 63) || 'app';
  return (pattern || DEFAULT_PREFS.urlPattern)
    .replaceAll('{name}', host)
    .replaceAll('{port}', String(port))
    .replaceAll('{domain}', domain || `${host}.test`);
}
