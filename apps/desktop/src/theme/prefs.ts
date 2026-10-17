// Client-side appearance and accessibility preferences (Settings → Appearance).
// Stored per machine in localStorage and applied as attributes on <html>.
export type Prefs = {
  density: 'comfortable' | 'compact';
  sidebar: 'expanded' | 'collapsed';
  reduceMotion: boolean;
  highContrast: boolean;
  shortcutHints: boolean;
};
export const DEFAULT_PREFS: Prefs = {
  density: 'comfortable',
  sidebar: 'expanded',
  reduceMotion: false,
  highContrast: false,
  shortcutHints: true,
};
const KEY = 'vsn.prefs';
export const SIDEBAR_EVENT = 'vsn:sidebar';

export function readPrefs(): Prefs {
  const prefs = { ...DEFAULT_PREFS };
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) Object.assign(prefs, JSON.parse(raw) as Partial<Prefs>);
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
  root.toggleAttribute('data-reduce-motion', p.reduceMotion);
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
