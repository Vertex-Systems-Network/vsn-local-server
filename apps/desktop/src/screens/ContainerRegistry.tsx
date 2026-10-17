import { useState, type ChangeEvent } from 'react';
import { call } from '../api/agent';
import { Badge, JsonView } from '../components/ui';

export function ContainerRegistry() {
  const [backend, setBackend] = useState('docker');
  const [source, setSource] = useState('local/app:latest');
  const [target, setTarget] = useState('registry.example.com/team/app:latest');
  const [push, setPush] = useState(false);
  const [result, setResult] = useState<any>(null);
  const [err, setErr] = useState('');
  async function publish() {
    setErr('');
    try {
      setResult(await call('container.registry-publish', { backend, source, target, push }));
    } catch (e) {
      setErr(String(e));
    }
  }
  return (
    <section className="panel">
      <div className="panelHead">
        <div>
          <span className="kicker">Registry publishing</span>
          <h3>Tag / push using existing Docker or Podman auth</h3>
        </div>
        <Badge ok label="No registry password in VSN request" />
      </div>
      <div className="formGrid">
        <label>
          Backend
          <select
            value={backend}
            onChange={(e: ChangeEvent<HTMLSelectElement>) => setBackend(e.target.value)}
          >
            <option>docker</option>
            <option>podman</option>
          </select>
        </label>
        <label>
          Source image
          <input value={source} onChange={(e: ChangeEvent<HTMLInputElement>) => setSource(e.target.value)} />
        </label>
        <label>
          Registry target
          <input value={target} onChange={(e: ChangeEvent<HTMLInputElement>) => setTarget(e.target.value)} />
        </label>
        <label className="check">
          <input
            type="checkbox"
            checked={push}
            onChange={(e: ChangeEvent<HTMLInputElement>) => setPush(e.target.checked)}
          />{' '}
          Push after tag
        </label>
      </div>
      <button className="primary" onClick={() => void publish()}>
        Tag / publish
      </button>
      <p className="hint">
        VSN does not accept registry passwords on the command line. Docker/Podman existing credential context
        is used by the backend.
      </p>
      {result && <JsonView value={result} />} {err && <div className="error">{err}</div>}
    </section>
  );
}
