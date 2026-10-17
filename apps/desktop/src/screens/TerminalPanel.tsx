import { useState, type ChangeEvent } from 'react';
import { call } from '../api/agent';
import { Badge, JsonView } from '../components/ui';

export function TerminalPanel() {
  const [cwd, setCwd] = useState('');
  const [program, setProgram] = useState('php');
  const [args, setArgs] = useState('["-a"]');
  const [session, setSession] = useState('');
  const [input, setInput] = useState('');
  const [output, setOutput] = useState('');
  const [result, setResult] = useState<any>(null);
  const [err, setErr] = useState('');
  const [pty, setPty] = useState(true);
  const [rows, setRows] = useState('30');
  const [cols, setCols] = useState('120');
  const [scrollbacks, setScrollbacks] = useState<any[]>([]);
  const [scrollId, setScrollId] = useState('');
  const [scrollOffset, setScrollOffset] = useState(0);
  const [scrollText, setScrollText] = useState('');
  function parsedArgs() {
    const value = JSON.parse(args);
    if (!Array.isArray(value)) throw new Error('Args must be a JSON array');
    return value as string[];
  }
  async function run() {
    setErr('');
    try {
      setResult(
        await call('terminal.exec', { cwd, program, args: parsedArgs(), env: {}, timeout_ms: 30000 }),
      );
    } catch (e) {
      setErr(String(e));
    }
  }
  async function startSession() {
    setErr('');
    try {
      const r: any = await call(
        pty ? 'terminal.pty.start' : 'terminal.session.start',
        pty
          ? { cwd, program, args: parsedArgs(), env: {}, rows: Number(rows), cols: Number(cols) }
          : { cwd, program, args: parsedArgs(), env: {} },
      );
      setSession(r.session_id);
      setResult(r);
    } catch (e) {
      setErr(String(e));
    }
  }
  async function write() {
    setErr('');
    try {
      setResult(
        await call(pty ? 'terminal.pty.write' : 'terminal.session.write', { session_id: session, input }),
      );
      setInput('');
    } catch (e) {
      setErr(String(e));
    }
  }
  async function read() {
    setErr('');
    try {
      const r: any = await call(pty ? 'terminal.pty.read-wait' : 'terminal.session.read-wait', {
        session_id: session,
        max_bytes: 65536,
        wait_ms: 1000,
      });
      setOutput((v) => v + (r.output ?? '') + (r.stdout ?? '') + (r.stderr ?? ''));
      setResult(r);
    } catch (e) {
      setErr(String(e));
    }
  }
  async function resize() {
    setErr('');
    try {
      setResult(
        await call('terminal.pty.resize', { session_id: session, rows: Number(rows), cols: Number(cols) }),
      );
    } catch (e) {
      setErr(String(e));
    }
  }
  async function stop() {
    setErr('');
    try {
      setResult(await call(pty ? 'terminal.pty.stop' : 'terminal.session.stop', { session_id: session }));
    } catch (e) {
      setErr(String(e));
    }
  }
  async function loadScrollbacks() {
    setErr('');
    try {
      const rows = await call<any[]>('terminal.pty.scrollback.list', {});
      setScrollbacks(rows);
      if (!scrollId && rows[0]) setScrollId(rows[0].session_id);
    } catch (e) {
      setErr(String(e));
    }
  }
  async function readScrollback() {
    if (!scrollId) return;
    setErr('');
    try {
      const chunk = await call<any>('terminal.pty.scrollback.read', {
        session_id: scrollId,
        offset: scrollOffset,
        max_bytes: 262144,
      });
      const raw = atob(chunk.payload_base64 || '');
      const bytes = Uint8Array.from(raw, (c) => c.charCodeAt(0));
      setScrollText((prev) => prev + new TextDecoder().decode(bytes));
      setScrollOffset(Number(chunk.next_offset || scrollOffset));
      setResult(chunk);
    } catch (e) {
      setErr(String(e));
    }
  }
  async function removeScrollback() {
    if (!scrollId) return;
    setErr('');
    try {
      await call('terminal.pty.scrollback.remove', { session_id: scrollId });
      setScrollText('');
      setScrollOffset(0);
      setScrollId('');
      await loadScrollbacks();
    } catch (e) {
      setErr(String(e));
    }
  }
  return (
    <>
      <section className="panel">
        <div className="panelHead">
          <div>
            <span className="kicker">Agent terminal</span>
            <h3>{pty ? 'Native PTY session' : 'Process-pipe session'}</h3>
          </div>
          <Badge ok={!!session} label={session || 'No session'} />
        </div>
        <div className="formGrid">
          <label>
            Workspace cwd
            <input value={cwd} onChange={(e) => setCwd(e.target.value)} />
          </label>
          <label>
            Program
            <input value={program} onChange={(e) => setProgram(e.target.value)} />
          </label>
          <label className="check">
            <input type="checkbox" checked={pty} onChange={(e) => setPty(e.target.checked)} /> Use PTY /
            ConPTY backend
          </label>
          <label>
            Rows
            <input value={rows} onChange={(e) => setRows(e.target.value)} />
          </label>
          <label>
            Columns
            <input value={cols} onChange={(e) => setCols(e.target.value)} />
          </label>
        </div>
        <label className="stacked">
          Arguments JSON
          <textarea value={args} onChange={(e) => setArgs(e.target.value)} rows={3} />
        </label>
        <div className="buttonRow">
          <button className="primary" onClick={() => void startSession()}>
            Start session
          </button>
          <button className="ghost" onClick={() => void run()}>
            One-shot exec
          </button>
          <button className="ghost" disabled={!session || !pty} onClick={() => void resize()}>
            Resize PTY
          </button>
          <button className="danger" disabled={!session} onClick={() => void stop()}>
            Stop
          </button>
        </div>
        <textarea className="editor" value={output} readOnly rows={14} placeholder="Session output" />
        <div className="formRow">
          <input
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder="Input sent to the active session"
          />
          <button className="primary" disabled={!session} onClick={() => void write()}>
            Send
          </button>
          <button className="ghost" disabled={!session} onClick={() => void read()}>
            Read output
          </button>
        </div>
        {result && <JsonView value={result} />} {err && <div className="error">{err}</div>}
        <p className="hint">
          PTY mode is the terminal-capable path for interactive shells, resize and TUIs; the bounded pipe mode
          remains available for programs that do not need terminal semantics. New PTY sessions also keep
          bounded durable scrollback under the VSN data directory.
        </p>
      </section>
      <section className="panel">
        <div className="panelHead">
          <div>
            <span className="kicker">Durable evidence</span>
            <h3>PTY scrollback journals</h3>
          </div>
          <Badge ok={scrollbacks.length > 0} label={`${scrollbacks.length} journals`} />
        </div>
        <div className="buttonRow">
          <button className="ghost" onClick={() => void loadScrollbacks()}>
            Refresh journals
          </button>
          <button className="ghost" disabled={!scrollId} onClick={() => void readScrollback()}>
            Read next chunk
          </button>
          <button className="danger" disabled={!scrollId} onClick={() => void removeScrollback()}>
            Remove inactive journal
          </button>
        </div>
        <div className="formGrid">
          <label>
            Journal
            <select
              value={scrollId}
              onChange={(e: ChangeEvent<HTMLSelectElement>) => {
                setScrollId(e.target.value);
                setScrollOffset(0);
                setScrollText('');
              }}
            >
              <option value="">Select journal</option>
              {scrollbacks.map((j) => (
                <option key={j.session_id} value={j.session_id}>
                  {j.session_id} · {j.bytes} bytes · {j.active ? 'active' : 'inactive'}
                </option>
              ))}
            </select>
          </label>
          <label>
            Offset
            <input
              type="number"
              min={0}
              value={scrollOffset}
              onChange={(e: ChangeEvent<HTMLInputElement>) =>
                setScrollOffset(Math.max(0, Number(e.target.value) || 0))
              }
            />
          </label>
        </div>
        {scrollText && <pre className="terminalOutput">{scrollText}</pre>}
        <p className="hint">
          Scrollback survives Agent restart, but VSN never recreates the previous PTY or replays shell input
          automatically.
        </p>
      </section>
    </>
  );
}
