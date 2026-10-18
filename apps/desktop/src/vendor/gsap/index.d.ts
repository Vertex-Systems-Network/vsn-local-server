// Minimal typings for the vendored GSAP core (see ./VENDOR.md).
export interface GsapTween {
  kill(): void;
}
export interface GsapTimeline extends GsapTween {
  fromTo(
    targets: unknown,
    from: Record<string, unknown>,
    to: Record<string, unknown>,
    position?: number | string,
  ): GsapTimeline;
}
export interface GsapApi {
  timeline(vars?: Record<string, unknown>): GsapTimeline;
  to(targets: unknown, vars: Record<string, unknown>): GsapTween;
  from(targets: unknown, vars: Record<string, unknown>): GsapTween;
  fromTo(targets: unknown, from: Record<string, unknown>, to: Record<string, unknown>): GsapTween;
  set(targets: unknown, vars: Record<string, unknown>): GsapTween;
  killTweensOf(targets: unknown): void;
}
export const gsap: GsapApi;
export default gsap;
