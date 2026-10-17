import { useEffect, useId, useRef, type ReactNode } from 'react';
import { popIn } from '../motion';
import { Icon } from './Icon';

export function Dialog({
  title,
  onClose,
  children,
  top = false,
}: {
  title: string;
  onClose: () => void;
  children: ReactNode;
  top?: boolean;
}) {
  const panelRef = useRef<HTMLDivElement>(null);
  const titleId = useId();
  // Keep the latest onClose without re-running the mount effect (focus + animation) on every parent render.
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    popIn(panelRef.current);
    (
      panelRef.current?.querySelector<HTMLElement>('input') ??
      panelRef.current?.querySelector<HTMLElement>('button, [tabindex]')
    )?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') closeRef.current();
    };
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('keydown', onKey);
      previous?.focus();
    };
  }, []);
  return (
    <div
      className={`overlay${top ? ' top' : ''}`}
      onMouseDown={(e) => e.target === e.currentTarget && onClose()}
    >
      <div className="dialog" role="dialog" aria-modal="true" aria-labelledby={titleId} ref={panelRef}>
        <header>
          <h2 id={titleId}>{title}</h2>
          <button className="iconButton" onClick={onClose} aria-label="Close">
            <Icon name="x" size={16} />
          </button>
        </header>
        {children}
      </div>
    </div>
  );
}
