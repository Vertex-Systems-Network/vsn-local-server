import { gsap } from './vendor/gsap/index.js';

// All UI motion goes through these helpers so the motion preference is honoured in one place.
// Settings → Appearance → Animations: 'on' (default), 'system' (follow Windows) or 'off'.
export function reducedMotion(): boolean {
  const pref = document.documentElement.dataset.motion;
  if (pref === 'off') return true;
  if (pref !== 'system') return false;
  try {
    return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  } catch {
    return false;
  }
}

/** Staggered fade/slide-in for the direct children of a content area. */
export function enterContent(container: HTMLElement | null) {
  if (!container || reducedMotion()) return;
  const items = Array.from(container.children);
  gsap.killTweensOf(items);
  gsap.fromTo(
    items,
    { y: 32, opacity: 0, scale: 0.98 },
    { y: 0, opacity: 1, scale: 1, duration: 0.6, ease: 'expo.out', stagger: 0.07, clearProps: 'transform,opacity' },
  );
}

/** Slide the mobile navigation drawer in. */
export function enterDrawer(drawer: HTMLElement | null) {
  if (!drawer || reducedMotion()) return;
  gsap.fromTo(
    drawer,
    { x: -40, autoAlpha: 0.6 },
    { x: 0, autoAlpha: 1, duration: 0.28, ease: 'power3.out', clearProps: 'transform,opacity,visibility' },
  );
}

/** Subtle pulse on the sidebar after collapsing or expanding. */
export function settleSidebar(sidebar: HTMLElement | null) {
  if (!sidebar || reducedMotion()) return;
  gsap.fromTo(
    sidebar.querySelectorAll('.navItem'),
    { autoAlpha: 0.4, x: -4 },
    {
      autoAlpha: 1,
      x: 0,
      duration: 0.25,
      stagger: 0.015,
      ease: 'power1.out',
      clearProps: 'transform,opacity,visibility',
    },
  );
}

/** Scale/fade a dialog or palette panel in. */
export function popIn(el: HTMLElement | null) {
  if (!el || reducedMotion()) return;
  gsap.fromTo(
    el,
    { scale: 0.96, y: 8, opacity: 0 },
    {
      scale: 1,
      y: 0,
      opacity: 1,
      duration: 0.22,
      ease: 'power2.out',
      clearProps: 'transform,opacity',
    },
  );
}

/** Slide a toast in from the right. */
export function slideIn(el: HTMLElement | null) {
  if (!el || reducedMotion()) return;
  gsap.fromTo(
    el,
    { x: 24, autoAlpha: 0 },
    { x: 0, autoAlpha: 1, duration: 0.3, ease: 'power3.out', clearProps: 'transform,opacity,visibility' },
  );
}

/** Move the tab underline to the active tab. */
export function moveInk(ink: HTMLElement | null, target: HTMLElement | null) {
  if (!ink || !target) return;
  const vars = { x: target.offsetLeft, width: target.offsetWidth };
  if (reducedMotion()) gsap.set(ink, vars);
  else gsap.to(ink, { ...vars, duration: 0.3, ease: 'power3.out' });
}

/** Count a number up from 0; the final text is always the exact value. */
export function countUp(el: HTMLElement | null, value: number) {
  if (!el) return;
  if (reducedMotion() || !Number.isFinite(value) || value === 0) {
    el.textContent = String(value);
    return;
  }
  const state = { n: 0 };
  gsap.to(state, {
    n: value,
    duration: 1.2,
    ease: 'power3.out',
    onUpdate: () => {
      el.textContent = String(Math.round(state.n));
    },
    onComplete: () => {
      el.textContent = String(value);
    },
  });
}

/** Brief pulse when a status changes. */
export function pulse(el: HTMLElement | null) {
  if (!el || reducedMotion()) return;
  gsap.fromTo(el, { scale: 1 }, { scale: 1.08, duration: 0.15, yoyo: true, repeat: 1, ease: 'power1.inOut' });
}

/** Stagger table rows / list items in on first load and when the row set changes. */
export function staggerRows(container: HTMLElement | null, selector = 'tbody tr') {
  if (!container || reducedMotion()) return;
  const rows = Array.from(container.querySelectorAll(selector));
  if (!rows.length) return;
  gsap.killTweensOf(rows);
  gsap.fromTo(
    rows,
    { x: -18, opacity: 0 },
    { x: 0, opacity: 1, duration: 0.45, ease: 'power3.out', stagger: 0.045, clearProps: 'transform,opacity' },
  );
}

/** Crossfade between Light and Dark tokens via a short-lived CSS transition class (colour tokens are CSS variables, which GSAP cannot interpolate across a theme swap). */
export function crossfadeTheme(apply: () => void) {
  const root = document.documentElement;
  if (reducedMotion()) return apply();
  root.classList.add('themeFade');
  apply();
  window.setTimeout(() => root.classList.remove('themeFade'), 400);
}

let glowTween: { kill: () => void } | null = null;
/** Slow ambient drift of the background glow; off with reduced motion. */
export function driftGlow(el: HTMLElement | null) {
  glowTween?.kill();
  glowTween = null;
  if (!el || reducedMotion()) return;
  glowTween = gsap.to(el, {
    x: 120,
    y: -80,
    scale: 1.15,
    rotation: 8,
    duration: 9,
    ease: 'sine.inOut',
    yoyo: true,
    repeat: -1,
  });
}

/** Smooth tween of a progress bar's width to `percent`. */
export function tweenProgress(bar: HTMLElement | null, percent: number) {
  if (!bar) return;
  const width = `${Math.max(0, Math.min(100, percent))}%`;
  if (reducedMotion()) {
    bar.style.width = width;
    return;
  }
  gsap.to(bar, { width, duration: 0.6, ease: 'power2.out' });
}

/** App intro: sidebar items, top bar and the first screen arrive in sequence. */
export function introApp(shell: HTMLElement | null) {
  if (!shell || reducedMotion()) return;
  const tl = gsap.timeline({ defaults: { ease: 'expo.out' } });
  tl.fromTo(shell.querySelector('.sidebar'), { x: -40, opacity: 0 }, { x: 0, opacity: 1, duration: 0.7, clearProps: 'transform,opacity' })
    .fromTo(
      shell.querySelectorAll('.navItem'),
      { x: -16, opacity: 0 },
      { x: 0, opacity: 1, duration: 0.5, stagger: 0.035, clearProps: 'transform,opacity' },
      '-=0.45',
    )
    .fromTo(shell.querySelector('.topbar'), { y: -24, opacity: 0 }, { y: 0, opacity: 1, duration: 0.6, clearProps: 'transform,opacity' }, 0.15);
}

/** Title swap in the top bar on section change. */
export function swapTitle(el: HTMLElement | null) {
  if (!el || reducedMotion()) return;
  gsap.fromTo(el, { y: 12, opacity: 0 }, { y: 0, opacity: 1, duration: 0.45, ease: 'power3.out', clearProps: 'transform,opacity' });
}
