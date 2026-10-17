import { useEffect, useState } from 'react';
import { call } from '../api/agent';
import { Badge, JsonView } from '../components/ui';

export function NativePostgresJobs() {
  const [connection, setConnection] = useState('host=localhost user=postgres dbname=postgres');
  const [sql, setSql] = useState('SELECT pg_sleep(10), now()');
  const [jobs, setJobs] = useState<any[]>([]);
  const [selectedJob, setSelectedJob] = useState('');
  const [txn, setTxn] = useState('');
  const [result, setResult] = useState<any>(null);
  const [err, setErr] = useState('');
  async function refresh() {
    setErr('');
    try {
      const rows = await call<any[]>('database.native.postgres.job.list', {});
      setJobs(rows);
      if (!selectedJob && rows[0]) setSelectedJob(rows[0].job_id);
    } catch (e) {
      setErr(String(e));
    }
  }
  async function start() {
    setErr('');
    try {
      const row = await call<any>('database.native.postgres.job.start', {
        connection_string: connection,
        sql,
      });
      setSelectedJob(row.job_id);
      setResult(row);
      await refresh();
    } catch (e) {
      setErr(String(e));
    }
  }
  async function cancel() {
    if (!selectedJob) return;
    setErr('');
    try {
      setResult(await call('database.native.postgres.job.cancel', { job_id: selectedJob }));
      await refresh();
    } catch (e) {
      setErr(String(e));
    }
  }
  async function jobStatus() {
    if (!selectedJob) return;
    setErr('');
    try {
      setResult(await call('database.native.postgres.job.status', { job_id: selectedJob }));
      await refresh();
    } catch (e) {
      setErr(String(e));
    }
  }
  async function txnStart() {
    setErr('');
    try {
      const row = await call<any>('database.native.postgres.txn.start', {
        connection_string: connection,
        ttl_seconds: 30,
      });
      setTxn(row.transaction_id);
      setResult(row);
    } catch (e) {
      setErr(String(e));
    }
  }
  async function txnQuery() {
    if (!txn) return;
    setErr('');
    try {
      setResult(await call('database.native.postgres.txn.query', { transaction_id: txn, sql }));
    } catch (e) {
      setErr(String(e));
    }
  }
  async function txnStatus() {
    if (!txn) return;
    setErr('');
    try {
      setResult(await call('database.native.postgres.txn.status', { transaction_id: txn }));
    } catch (e) {
      setErr(String(e));
    }
  }
  async function txnClose(commit = false) {
    if (!txn) return;
    setErr('');
    try {
      setResult(await call('database.native.postgres.txn.close', { transaction_id: txn, commit }));
      setTxn('');
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
          <span className="kicker">Native PostgreSQL control</span>
          <h3>Server cancel + read-only transaction sessions</h3>
        </div>
        <Badge ok label="Loopback / read-only" />
      </div>
      <div className="formGrid">
        <label>
          Connection
          <input value={connection} onChange={(e) => setConnection(e.target.value)} />
        </label>
        <label>
          Job
          <select value={selectedJob} onChange={(e) => setSelectedJob(e.target.value)}>
            <option value="">Select job</option>
            {jobs.map((j) => (
              <option key={j.job_id} value={j.job_id}>
                {j.job_id} · {j.state}
                {j.cancel_requested ? ' · cancel requested' : ''}
              </option>
            ))}
          </select>
        </label>
        <label>
          Transaction
          <input value={txn} readOnly placeholder="No open transaction" />
        </label>
      </div>
      <label className="stacked">
        Read-only SQL
        <textarea value={sql} onChange={(e) => setSql(e.target.value)} rows={4} />
      </label>
      <div className="buttonRow">
        <button className="primary" onClick={() => void start()}>
          Start native job
        </button>
        <button className="ghost" onClick={() => void refresh()}>
          Refresh jobs
        </button>
        <button className="ghost" disabled={!selectedJob} onClick={() => void jobStatus()}>
          Job status
        </button>
        <button className="danger" disabled={!selectedJob} onClick={() => void cancel()}>
          Server cancel request
        </button>
        <button className="primary" onClick={() => void txnStart()}>
          Begin READ ONLY
        </button>
        <button className="ghost" disabled={!txn} onClick={() => void txnQuery()}>
          Query transaction
        </button>
        <button className="ghost" disabled={!txn} onClick={() => void txnStatus()}>
          Txn status
        </button>
        <button className="ghost" disabled={!txn} onClick={() => void txnClose(true)}>
          Commit
        </button>
        <button className="danger" disabled={!txn} onClick={() => void txnClose(false)}>
          Rollback
        </button>
      </div>
      <p className="hint">
        Native PostgreSQL jobs request server-side cancellation through PostgreSQL's cancellation protocol.
        Transaction sessions are BEGIN READ ONLY, bounded by TTL, statement count and server statement/idle
        timeouts. They are in-memory and intentionally not reconstructed after Agent restart.
      </p>
      {result && <JsonView value={result} />} {err && <div className="error">{err}</div>}
    </section>
  );
}
