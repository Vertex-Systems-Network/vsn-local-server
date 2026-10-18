import { useEffect, useRef } from 'react';
import { Logo } from './Logo';
import { splashIn, splashOut } from '../motion';

/**
 * Start-up loader: covers the app until the agent has answered (online or offline),
 * shown for at least ~1.4 s so the GSAP intro plays through, then fades away.
 */
export function Splash({ ready, onDone }: { ready: boolean; onDone: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const shownAt = useRef(performance.now());
  const doneRef = useRef(onDone);
  doneRef.current = onDone;
  useEffect(() => splashIn(ref.current), []);
  useEffect(() => {
    if (!ready) return;
    const wait = Math.max(0, 1400 - (performance.now() - shownAt.current));
    const t = window.setTimeout(() => splashOut(ref.current, () => doneRef.current()), wait);
    return () => window.clearTimeout(t);
  }, [ready]);
  return (
    <div className="splash" ref={ref} role="status" aria-live="polite">
      <div className="splashMark">
        <span className="splashRing" aria-hidden="true" />
        <Logo size={84} />
      </div>
      <div className="splashWord">
        <b>VSN</b> Devify
      </div>
      <div className="splashBar" aria-hidden="true">
        <i />
      </div>
      <small>{ready ? 'Ready' : 'Starting the VSN Agent…'}</small>
    </div>
  );
}
