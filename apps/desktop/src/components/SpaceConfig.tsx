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
type Step = { label: string; program: string; args: string[] };
type StepResult = { framework: string; status_code: number | null; timed_out: boolean; output_tail: string };

/** DVF-03: edit a space's config files and clear its package caches. */
export function SpaceConfig({ root }: { root: string }) {
  const [present, setPresent] = useState<string[]>([]);
  const [cache, setCache] = useState<number | null>(null);
  const [open, setOpen] = useState<{ file: string; content: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [steps, setSteps] = useState<Step[]>([]);
  const [updating, setUpdating] = useState(false);
  const [updateLog, setUpdateLog] = useState<StepResult[] | null>(null);
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
    call<Step[]>('framework.update.plan', { path: root })
      .then(setSteps)
      .catch(() => setSteps([]));
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
  async function runUpdate() {
    const list = steps.map((s) => `• ${s.label}: ${s.program} ${s.args.join(' ')}`).join('\n');
    if (
      !window.confirm(`Update this project?\n\n${list}\n\nCommit or back up first — updates can change code.`)
    )
      return;
    setUpdating(true);
    setUpdateLog(null);
    try {
      const r = await call<StepResult[]>('framework.update', { path: root });
      setUpdateLog(r);
      const ok = r.every((x) => x.status_code === 0 && !x.timed_out);
      toast(ok ? 'Project updated' : 'An update step failed — see the log', ok ? undefined : 'danger');
    } catch (e) {
      toast(String(e), 'danger');
    } finally {
      setUpdating(false);
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
        {steps.length > 0 && (
          <button className="ghost" disabled={updating} onClick={() => void runUpdate()}>
            {updating ? 'Updating…' : `Update ${steps.map((s) => s.label).join(', ')}`}
          </button>
        )}
        <button className="ghost" disabled={busy || !cache} onClick={() => void clearCaches()}>
          Clear caches {cache != null ? `(${formatBytes(cache)})` : ''}
        </button>
      </div>
      {updateLog && (
        <pre className="json">
          {updateLog
            .map(
              (r) =>
                `== ${r.framework}: ${r.timed_out ? 'timed out' : r.status_code === 0 ? 'done' : `failed (${r.status_code})`}\n${r.output_tail.trim()}`,
            )
            .join('\n\n')}
        </pre>
      )}
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
