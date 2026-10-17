import { useLayoutEffect, useRef } from 'react';
import { moveInk } from '../motion';

export function Tabs<T extends string>({
  tabs,
  value,
  onChange,
  label,
  panelId,
}: {
  tabs: readonly (readonly [T, string])[];
  value: T;
  onChange: (next: T) => void;
  label: string;
  /** id of the element with role="tabpanel" that shows the active tab's content. */
  panelId: string;
}) {
  const listRef = useRef<HTMLDivElement>(null);
  const inkRef = useRef<HTMLSpanElement>(null);
  useLayoutEffect(() => {
    const active = listRef.current?.querySelector<HTMLElement>('[aria-selected="true"]') ?? null;
    moveInk(inkRef.current, active);
  }, [value]);
  return (
    <div className="tabs" role="tablist" aria-label={label} ref={listRef}>
      {tabs.map(([id, text]) => (
        <button
          key={id}
          id={`${panelId}-tab-${id}`}
          role="tab"
          aria-selected={id === value}
          aria-controls={panelId}
          tabIndex={id === value ? 0 : -1}
          onClick={() => onChange(id)}
        >
          {text}
        </button>
      ))}
      <span className="tabInk" ref={inkRef} aria-hidden="true" />
    </div>
  );
}
