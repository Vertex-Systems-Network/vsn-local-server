/// <reference types="vite/client" />
import { useMemo, useState, type ReactNode } from 'react';
import { call } from '../api/agent';
import { useToast } from '../components/Toast';

/** The wiki pages in docs/wiki, bundled at build time so Help works offline. */
const PAGES = import.meta.glob('../../../../docs/wiki/*.md', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

type Check = { area: string; check: string; ok: boolean; detail: string; fix: string };

function pageId(path: string) {
  return path.split('/').pop()!.replace(/\.md$/, '');
}
const slug = (title: string) => title.trim().replace(/\s+/g, '-');

/** Inline markdown: **bold**, `code` and [[Wiki links]]. */
function inline(text: string, open: (id: string) => void): ReactNode[] {
  const out: ReactNode[] = [];
  const re = /(\*\*[^*]+\*\*|`[^`]+`|\[\[[^\]]+\]\])/g;
  let last = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text))) {
    if (m.index > last) out.push(text.slice(last, m.index));
    const t = m[0];
    if (t.startsWith('**')) out.push(<strong key={m.index}>{t.slice(2, -2)}</strong>);
    else if (t.startsWith('`')) out.push(<code key={m.index}>{t.slice(1, -1)}</code>);
    else {
      const title = t.slice(2, -2);
      out.push(
        <button key={m.index} className="linkButton" onClick={() => open(slug(title))}>
          {title}
        </button>,
      );
    }
    last = m.index + t.length;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

/** A small, safe markdown renderer: headings, paragraphs, lists, tables, quotes. */
function Markdown({ source, open }: { source: string; open: (id: string) => void }) {
  const blocks: ReactNode[] = [];
  const lines = source.split('\n');
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (!line.trim()) {
      i++;
      continue;
    }
    const h = /^(#{1,3})\s+(.*)$/.exec(line);
    if (h) {
      const level = h[1].length;
      const content = inline(h[2], open);
      blocks.push(level === 1 ? <h3 key={i}>{content}</h3> : <h4 key={i}>{content}</h4>);
      i++;
      continue;
    }
    if (line.startsWith('|')) {
      const rows: string[][] = [];
      while (i < lines.length && lines[i].startsWith('|')) {
        const cells = lines[i]
          .split('|')
          .slice(1, -1)
          .map((c) => c.trim());
        if (!cells.every((c) => /^:?-+:?$/.test(c))) rows.push(cells);
        i++;
      }
      const [head, ...body] = rows;
      blocks.push(
        <div className="tableWrap" key={i}>
          <table>
            <thead>
              <tr>
                {head.map((c, n) => (
                  <th key={n}>{inline(c, open)}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {body.map((r, rn) => (
                <tr key={rn}>
                  {r.map((c, n) => (
                    <td key={n}>{inline(c, open)}</td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>,
      );
      continue;
    }
    if (/^(\d+\.|-)\s/.test(line)) {
      const ordered = /^\d+\./.test(line);
      const items: string[] = [];
      while (i < lines.length && /^(\d+\.|-)\s/.test(lines[i])) {
        items.push(lines[i].replace(/^(\d+\.|-)\s+/, ''));
        i++;
      }
      const li = items.map((t, n) => <li key={n}>{inline(t, open)}</li>);
      blocks.push(ordered ? <ol key={i}>{li}</ol> : <ul key={i}>{li}</ul>);
      continue;
    }
    if (line.startsWith('>')) {
      blocks.push(
        <p className="hint" key={i}>
          {inline(line.replace(/^>\s?/, ''), open)}
        </p>,
      );
      i++;
      continue;
    }
    const para: string[] = [];
    while (i < lines.length && lines[i].trim() && !/^(#|\||-\s|\d+\.\s|>)/.test(lines[i])) {
      para.push(lines[i]);
      i++;
    }
    blocks.push(<p key={i}>{inline(para.join(' '), open)}</p>);
  }
  return <div className="helpDoc">{blocks}</div>;
}

/** Help (docs + Diagnose): the user guide from the wiki and a one-click health check. */
export function Help() {
  const toast = useToast();
  const pages = useMemo(
    () =>
      Object.entries(PAGES)
        .map(([path, text]) => ({ id: pageId(path), text }))
        .filter((p) => !p.id.startsWith('_'))
        .sort((a, b) => (a.id === 'Home' ? -1 : b.id === 'Home' ? 1 : a.id.localeCompare(b.id))),
    [],
  );
  const [current, setCurrent] = useState('Home');
  const [checks, setChecks] = useState<Check[] | null>(null);
  const [busy, setBusy] = useState(false);
  const page = pages.find((p) => p.id === current) ?? pages[0];

  async function diagnose() {
    setBusy(true);
    try {
      setChecks(await call<Check[]>('support.diagnose'));
    } catch (e) {
      toast(String(e), 'danger');
    } finally {
      setBusy(false);
    }
  }
  function report() {
    const text = (checks ?? [])
      .map((c) => `${c.ok ? 'OK ' : 'NO '} [${c.area}] ${c.check} — ${c.detail}${c.fix ? ` → ${c.fix}` : ''}`)
      .join('\n');
    void navigator.clipboard?.writeText(text).then(() => toast('Report copied'));
  }
  const problems = checks?.filter((c) => !c.ok).length ?? 0;

  return (
    <>
      <section className="panel" aria-labelledby="help-diagnose">
        <div className="panelHead">
          <div>
            <span className="kicker">Something not working?</span>
            <h3 id="help-diagnose">Diagnose</h3>
          </div>
          <div className="buttonRow">
            <button className="primary" disabled={busy} onClick={() => void diagnose()}>
              {busy ? 'Checking…' : 'Run Diagnose'}
            </button>
            {checks && (
              <button className="ghost" onClick={report}>
                Copy report
              </button>
            )}
          </div>
        </div>
        {checks && (
          <>
            <p className={problems ? 'error' : 'hint'}>
              {problems ? `${problems} item(s) need attention.` : 'Everything checked looks good.'}
            </p>
            <ul className="diagnoseList">
              {[...checks]
                .sort((a, b) => Number(a.ok) - Number(b.ok))
                .map((c, n) => (
                  <li key={n} className={c.ok ? 'ok' : 'bad'}>
                    <b>{c.ok ? 'OK' : 'Fix'}</b> <span className="muted">{c.area}</span> {c.check}
                    <small> — {c.detail}</small>
                    {c.fix && <div className="hint">{c.fix}</div>}
                  </li>
                ))}
            </ul>
          </>
        )}
      </section>
      <section className="panel" aria-labelledby="help-docs">
        <div className="panelHead">
          <div>
            <span className="kicker">User guide</span>
            <h3 id="help-docs">Documentation</h3>
          </div>
          <label className="inlineField">
            Page
            <select value={page?.id} onChange={(e) => setCurrent(e.target.value)}>
              {pages.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.id.replace(/-/g, ' ')}
                </option>
              ))}
            </select>
          </label>
        </div>
        {page && (
          <Markdown
            source={page.text}
            open={(id) => (pages.some((p) => p.id === id) ? setCurrent(id) : undefined)}
          />
        )}
      </section>
    </>
  );
}
