import { useState, type ChangeEvent } from 'react';
import { call } from '../api/agent';
import { Badge } from '../components/ui';
import { Dialog } from '../components/Dialog';
import { ResultView } from '../components/ResultView';
import { useToast } from '../components/Toast';

export function Services() {
  const [name, setName] = useState('VSN-');
  const [result, setResult] = useState<any>(null);
  const [err, setErr] = useState('');
  const [confirmStop, setConfirmStop] = useState(false);
  const toast = useToast();
  async function run(action: 'status' | 'start' | 'stop' | 'restart') {
    setErr('');
    try {
      setResult(
        action === 'status'
          ? await call('service.status', { name })
          : await call('service.action', { name, action }),
      );
      if (action !== 'status') toast(`${name}: ${action} requested`, 'info');
    } catch (e) {
      setErr(String(e));
    }
  }
  return (
    <section className="panel">
      <div className="panelHead">
        <div>
          <span className="kicker">OS service boundary</span>
          <h3>VSN-managed services</h3>
        </div>
        <Badge ok label="VSN-* mutations only" />
      </div>
      <div className="formRow">
        <input
          value={name}
          onChange={(e: ChangeEvent<HTMLInputElement>) => setName(e.target.value)}
          placeholder="VSN-Postgres"
        />
        <button className="primary" onClick={() => void run('status')}>
          Status
        </button>
      </div>
      <div className="buttonRow">
        <button className="ghost" onClick={() => void run('start')}>
          Start
        </button>
        <button className="ghost" onClick={() => void run('restart')}>
          Restart
        </button>
        <button className="danger" onClick={() => setConfirmStop(true)}>
          Stop
        </button>
      </div>
      {result?.state && (
        <div className="serviceState">
          <b>{name}</b>
          <Badge ok={String(result.state).toLowerCase() === 'running'} label={String(result.state)} />
        </div>
      )}
      <ResultView title="Service response" value={result} error={err} />
      {confirmStop && (
        <Dialog title={`Stop ${name}?`} onClose={() => setConfirmStop(false)}>
          <p>Anything that depends on this service will lose its connection until it is started again.</p>
          <div className="buttonRow">
            <button className="ghost" onClick={() => setConfirmStop(false)}>
              Cancel
            </button>
            <button
              className="danger"
              onClick={() => {
                setConfirmStop(false);
                void run('stop');
              }}
            >
              Stop service
            </button>
          </div>
        </Dialog>
      )}
    </section>
  );
}
