import { useEffect, useState } from 'react';
import { call } from '../api/agent';
import { Dialog } from './Dialog';
import { useToast } from './Toast';
import { formatBytes } from '../screens/Analytics';

/** Config files Devify knows how to edit, relative to the space folder. */
const KNOWN = [
  '.env',
  '.env.local',
  '.env.example',
  '.devify/space.toml',
  'php.ini',
  'composer.json',
  'package.json',
  'vite.config.ts',
  'next.config.js',
  'nginx.conf',
  'my.cnf',
];

type Entry = { name: string; is_dir: boolean };

/** DVF-03: edit a space's config files and clear its package caches. */
export function SpaceConfig({ root }: { root: string }) {
  const [present, setPresent] = useState<string[]>([]);
  const [cache, setCache] = useState<number | null>(null);
  const [open, setOpen] = useState<{ file: string; content: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const toast = useToast();

  async function scan() {
    const found: string[] = [];
    try {
      const top = await call<Entry[]>('files.list', { path: root });
      const names = new Set(top.map((e) => e.name));
      for (const f of KNOWN) if (!f.includes('/') && names.has(f)) found.push(f);
      found.push('.devify/space.toml');
    } catch {
      // Folder not readable: leave the list empty.
    }
    setPresent(found);
    call<number>('space.cache.size', { path: root })
      .then(setCache)
      .catch(() => setCache(null));
  }
  useEffect(() => {
    void scan();
  }, [root]);

  async function edit(file: string) {
    try {
      const r = await call<{ content: string }>('files.read', { path: `${root}/${file}` });
      setOpen({ file, content: r.content });
    } catch (e) {
      toast(String(e), 'danger');
    }
  }
  async function save() {
    if (!open) return;
    setBusy(true);
    try {
      await call('files.write', { path: `${root}/${open.file}`, content: open.content });
      toast(`Saved ${open.file}. Restart the space's services to apply.`);
      setOpen(null);
    } catch (e) {
      toast(String(e), 'danger');
    } finally {
      setBusy(false);
    }
  }
  async function clearCaches() {
    if (
      !window.confirm(
        'Delete this space’s package caches (npm, composer, pip, Go, Cargo)? They download again when needed.',
      )
    )
      return;
    setBusy(true);
    try {
      const freed = await call<number>('space.cache.clear', { path: root });
      toast(`Freed ${formatBytes(freed)}`);
      await scan();
    } catch (e) {
      toast(String(e), 'danger');
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <h4>Config &amp; caches</h4>
      <div className="buttonRow">
        {present.map((f) => (
          <button key={f} className="ghost" onClick={() => void edit(f)}>
            Edit {f}
          </button>
        ))}
        <button className="ghost" disabled={busy || !cache} onClick={() => void clearCaches()}>
          Clear caches {cache != null ? `(${formatBytes(cache)})` : ''}
        </button>
      </div>
      {open && (
        <Dialog title={`Edit ${open.file}`} onClose={() => setOpen(null)}>
          <label className="stacked">
            {open.file}
            <textarea
              rows={18}
              spellCheck={false}
              className="codeArea"
              value={open.content}
              onChange={(e) => setOpen({ ...open, content: e.target.value })}
            />
          </label>
          {open.file.startsWith('.env') && (
            <p className="hint">
              This file usually holds secrets. Keep it out of git by listing it in your .gitignore.
            </p>
          )}
          <div className="buttonRow">
            <button className="primary" disabled={busy} onClick={() => void save()}>
              Save
            </button>
            <button className="ghost" onClick={() => setOpen(null)}>
              Cancel
            </button>
          </div>
        </Dialog>
      )}
    </>
  );
}
