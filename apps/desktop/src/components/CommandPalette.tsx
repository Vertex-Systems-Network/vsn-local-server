import { useMemo, useState } from 'react';
import { Dialog } from './Dialog';
import { Icon, type IconName } from './Icon';

export type Command = { id: string; label: string; hint?: string; icon: IconName; run: () => void };

export function CommandPalette({ commands, onClose }: { commands: Command[]; onClose: () => void }) {
  const [query, setQuery] = useState('');
  const [index, setIndex] = useState(0);
  const results = useMemo(() => {
    const q = query.trim().toLowerCase();
    return q ? commands.filter((c) => `${c.label} ${c.hint ?? ''}`.toLowerCase().includes(q)) : commands;
  }, [commands, query]);
  const run = (c: Command | undefined) => {
    if (!c) return;
    onClose();
    c.run();
  };
  return (
    <Dialog title="Command palette" onClose={onClose} top>
      <div className="palette">
        <input
          value={query}
          aria-label="Search commands"
          placeholder="Search or run a command…"
          onChange={(e) => {
            setQuery(e.target.value);
            setIndex(0);
          }}
          onKeyDown={(e) => {
            if (e.key === 'ArrowDown') {
              e.preventDefault();
              setIndex((i) => Math.min(i + 1, results.length - 1));
            } else if (e.key === 'ArrowUp') {
              e.preventDefault();
              setIndex((i) => Math.max(i - 1, 0));
            } else if (e.key === 'Enter') {
              run(results[index]);
            }
          }}
        />
        {results.length === 0 ? (
          <p className="empty">No matching commands</p>
        ) : (
          <ul role="listbox" aria-label="Commands">
            {results.map((c, i) => (
              <li key={c.id} role="none">
                <button
                  role="option"
                  aria-selected={i === index}
                  onClick={() => run(c)}
                  onMouseEnter={() => setIndex(i)}
                >
                  <Icon name={c.icon} size={18} />
                  {c.label}
                  {c.hint && <small>{c.hint}</small>}
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </Dialog>
  );
}
