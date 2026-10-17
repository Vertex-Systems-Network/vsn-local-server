import { gsap } from './vendor/gsap/index.js';

// All UI motion goes through these helpers so reduced-motion is honoured in one place.
export function reducedMotion(): boolean {
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
    { y: 14 },
    // Transform only: content stays fully opaque so contrast checks are never taken mid-fade.
    { y: 0, duration: 0.45, ease: 'power3.out', stagger: 0.05, clearProps: 'transform' },
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
