import { crossfadeTheme } from '../motion';

export type ThemePref = 'system' | 'light' | 'dark';
const KEY = 'vsn.theme';

export function readTheme(): ThemePref {
  try {
    const v = localStorage.getItem(KEY);
    if (v === 'light' || v === 'dark') return v;
  } catch {
    // Storage may be unavailable; fall back to the OS setting.
  }
  return 'system';
}

function setAttr(pref: ThemePref) {
  const root = document.documentElement;
  if (pref === 'system') root.removeAttribute('data-theme');
  else root.setAttribute('data-theme', pref);
}

/** Apply without animation (startup). */
export function initTheme() {
  setAttr(readTheme());
}

export function applyTheme(pref: ThemePref) {
  try {
    if (pref === 'system') localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, pref);
  } catch {
    // Preference persistence is best-effort.
  }
  crossfadeTheme(() => setAttr(pref));
}
