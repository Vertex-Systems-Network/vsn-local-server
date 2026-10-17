import { useEffect, useState, type ChangeEvent } from 'react';
import { call } from '../api/agent';
import { Badge, JsonView } from '../components/ui';

export function DatabaseJobs() {
  const [connectionJson, setConnectionJson] = useState(
    '{"engine":"postgresql","host":"127.0.0.1","port":5432,"user":"postgres","database":"postgres","credential_file":null,"service":null}',
  );
  const [sql, setSql] = useState('SELECT pg_sleep(10), now()');
  const [jobs, setJobs] = useState<any[]>([]);
  const [selected, setSelected] = useState('');
  const [result, setResult] = useState<any>(null);
  const [err, setErr] = useState('');
  const [artifactOffset, setArtifactOffset] = useState(0);
  const [artifactText, setArtifactText] = useState('');
  async function refresh() {
    setErr('');
    try {
      const rows = await call<any[]>('database.cli.job.list', {});
      setJobs(rows);
      if (!selected && rows[0]) setSelected(rows[0].job_id);
    } catch (e) {
      setErr(String(e));
    }
  }
  async function start() {
    setErr('');
    try {
      const connection = JSON.parse(connectionJson);
      const row = await call<any>('database.cli.job.start', { connection, sql });
      setResult(row);
      setSelected(row.job_id);
      setArtifactOffset(0);
      setArtifactText('');
      await refresh();
    } catch (e) {
      setErr(String(e));
    }
  }
  async function status() {
    if (!selected) return;
    setErr('');
    try {
      setResult(await call('database.cli.job.status', { job_id: selected }));
      await refresh();
    } catch (e) {
      setErr(String(e));
    }
  }
  async function cancel() {
    if (!selected) return;
    setErr('');
    try {
      setResult(await call('database.cli.job.cancel', { job_id: selected }));
      await refresh();
    } catch (e) {
      setErr(String(e));
    }
  }
  async function readArtifact() {
    if (!selected) return;
    setErr('');
    try {
      const chunk = await call<any>('database.cli.job.output', {
        job_id: selected,
        offset: artifactOffset,
        max_bytes: 262144,
      });
      const raw = atob(chunk.payload_base64 || '');
      const bytes = Uint8Array.from(raw, (c) => c.charCodeAt(0));
      setArtifactText((prev) => prev + new TextDecoder().decode(bytes));
      setArtifactOffset(Number(chunk.next_offset || artifactOffset));
      setResult(chunk);
    } catch (e) {
      setErr(String(e));
    }
  }
  async function removeArtifact() {
    if (!selected) return;
    setErr('');
    try {
      await call('database.cli.job.output-remove', { job_id: selected });
      setArtifactOffset(0);
      setArtifactText('');
      await status();
    } catch (e) {
      setErr(String(e));
    }
  }
  useEffect(() => {
    void refresh();
  }, []);
  return (
    <section className="panel">
      <div className="panelHead">
        <div>
          <span className="kicker">Durable cancellable reads</span>
          <h3>External database query jobs</h3>
        </div>
        <Badge ok label="No shell / actual process kill" />
      </div>
      <label className="stacked">
        Connection JSON
        <textarea
          value={connectionJson}
          onChange={(e: ChangeEvent<HTMLTextAreaElement>) => setConnectionJson(e.target.value)}
          rows={4}
        />
      </label>
      <label className="stacked">
        Read-only SQL
        <textarea
          value={sql}
          onChange={(e: ChangeEvent<HTMLTextAreaElement>) => setSql(e.target.value)}
          rows={4}
        />
      </label>
      <div className="buttonRow">
        <button className="primary" onClick={() => void start()}>
          Start job
        </button>
        <button className="ghost" onClick={() => void refresh()}>
          Refresh
        </button>
        <button className="ghost" disabled={!selected} onClick={() => void status()}>
          Status
        </button>
        <button className="danger" disabled={!selected} onClick={() => void cancel()}>
          Cancel
        </button>
        <button className="ghost" disabled={!selected} onClick={() => void readArtifact()}>
          Read result chunk
        </button>
        <button className="danger" disabled={!selected} onClick={() => void removeArtifact()}>
          Remove result artifact
        </button>
      </div>
      <div className="formGrid">
        <label>
          Job
          <select
            value={selected}
            onChange={(e: ChangeEvent<HTMLSelectElement>) => {
              setSelected(e.target.value);
              setArtifactOffset(0);
              setArtifactText('');
            }}
          >
            <option value="">Select job</option>
            {jobs.map((j) => (
              <option value={j.job_id} key={j.job_id}>
                {j.job_id} · {j.engine} · {j.state}
                {j.artifact ? ` · ${j.artifact.bytes} bytes` : ''}
              </option>
            ))}
          </select>
        </label>
        <label>
          Artifact offset
          <input
            type="number"
            min={0}
            value={artifactOffset}
            onChange={(e: ChangeEvent<HTMLInputElement>) =>
              setArtifactOffset(Math.max(0, Number(e.target.value) || 0))
            }
          />
        </label>
      </div>
      <p className="hint">
        Jobs are journaled before execution. Agent restart marks an orphaned Running journal Interrupted
        without re-executing it. Successful output may be spooled up to 64 MiB with SHA-256 and read back in
        bounded chunks.
      </p>
      {artifactText && <pre className="terminalOutput">{artifactText}</pre>}
      {result && <JsonView value={result} />} {err && <div className="error">{err}</div>}
    </section>
  );
}
