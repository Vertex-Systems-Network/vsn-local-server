import { useState, type ChangeEvent } from 'react';
import { call } from '../api/agent';
import { WorkspaceRoots } from '../components/WorkspaceRoots';
import { NewProject } from '../components/NewProject';
import { ResultView } from '../components/ResultView';

export function Projects() {
  const [path, setPath] = useState('');
  const [result, setResult] = useState<any>(null);
  const [previewPort, setPreviewPort] = useState('8000');
  const [previewPath, setPreviewPath] = useState('/');
  const [previewMethod, setPreviewMethod] = useState('GET');
  const [previewBody, setPreviewBody] = useState('');
  const [err, setErr] = useState('');
  async function inspect() {
    setErr('');
    try {
      setResult({
        project: await call('project.detect', { path }),
        dependencies: await call('project.dependencies', { path }),
      });
    } catch (e) {
      setErr(String(e));
    }
  }
  async function preview() {
    setErr('');
    try {
      setResult(await call('preview.fetch', { port: Number(previewPort), path: previewPath, method: 'GET' }));
    } catch (e) {
      setErr(String(e));
    }
  }
  async function previewRequest() {
    setErr('');
    try {
      const bytes = new TextEncoder().encode(previewBody);
      let binary = '';
      for (const byte of bytes) binary += String.fromCharCode(byte);
      setResult(
        await call('preview.request', {
          port: Number(previewPort),
          path: previewPath,
          method: previewMethod,
          headers: { 'content-type': 'application/json' },
          body_base64: btoa(binary),
        }),
      );
    } catch (e) {
      setErr(String(e));
    }
  }
  return (
    <>
      <WorkspaceRoots onInspect={setPath} />
      <NewProject />
      <section className="panel">
        <div className="panelHead">
          <div>
            <span className="kicker">Detection</span>
            <h3>Inspect project</h3>
          </div>
        </div>
        <div className="formRow">
          <input
            value={path}
            onChange={(e: ChangeEvent<HTMLInputElement>) => setPath(e.target.value)}
            placeholder="Absolute project directory"
          />
          <button className="primary" onClick={() => void inspect()}>
            Detect
          </button>
        </div>
        <div className="formGrid">
          <label>
            Preview port
            <input
              value={previewPort}
              onChange={(e: ChangeEvent<HTMLInputElement>) => setPreviewPort(e.target.value)}
            />
          </label>
          <label>
            Preview path
            <input
              value={previewPath}
              onChange={(e: ChangeEvent<HTMLInputElement>) => setPreviewPath(e.target.value)}
            />
          </label>
          <label>
            Local proxy method
            <select value={previewMethod} onChange={(e) => setPreviewMethod(e.target.value)}>
              <option>GET</option>
              <option>HEAD</option>
              <option>POST</option>
              <option>PUT</option>
              <option>PATCH</option>
              <option>DELETE</option>
              <option>OPTIONS</option>
            </select>
          </label>
        </div>
        <label className="stacked">
          Local proxy body
          <textarea
            value={previewBody}
            onChange={(e) => setPreviewBody(e.target.value)}
            rows={4}
            placeholder="Optional UTF-8 body; local-only advanced preview request"
          />
        </label>
        <div className="buttonRow">
          <button className="ghost" onClick={() => void preview()}>
            Fetch private read-only preview
          </button>
          <button className="primary" onClick={() => void previewRequest()}>
            Send local advanced preview
          </button>
        </div>
        <p className="hint">
          Advanced preview supports bounded request bodies and filtered headers against 127.0.0.1 only.
          Mutation-capable preview.request is not in the remote signed-command allowlist.
        </p>
        <ResultView title="Project details" value={result} error={err} />
      </section>
    </>
  );
}
