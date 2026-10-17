import { useState } from 'react';
import { call } from '../api/agent';
import type { FileEntry } from '../contracts';
import { Badge } from '../components/ui';
import { ResultView } from '../components/ResultView';

export function FilesPanel() {
  const [path, setPath] = useState('');
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [file, setFile] = useState('');
  const [content, setContent] = useState('');
  const [opPath, setOpPath] = useState('');
  const [destination, setDestination] = useState('');
  const [transferId, setTransferId] = useState('');
  const [result, setResult] = useState<any>(null);
  const [err, setErr] = useState('');
  async function list() {
    setErr('');
    try {
      setEntries(await call<FileEntry[]>('files.list', { path }));
    } catch (e) {
      setErr(String(e));
    }
  }
  async function read() {
    setErr('');
    try {
      const r: any = await call('files.read', { path: file });
      setContent(r.content ?? '');
      setResult(r);
    } catch (e) {
      setErr(String(e));
    }
  }
  async function write() {
    setErr('');
    try {
      setResult(await call('files.write', { path: file, content }));
    } catch (e) {
      setErr(String(e));
    }
  }
  async function mutate(action: 'mkdir' | 'move' | 'delete') {
    setErr('');
    try {
      if (action === 'mkdir') setResult(await call('files.mkdir', { path: opPath }));
      else if (action === 'move') setResult(await call('files.move', { source: opPath, destination }));
      else setResult(await call('files.delete', { path: opPath, recursive: true }));
      await list();
    } catch (e) {
      setErr(String(e));
    }
  }
  async function digest() {
    setErr('');
    try {
      setResult(await call('files.digest', { path: file }));
    } catch (e) {
      setErr(String(e));
    }
  }
  async function uploadStatus() {
    setErr('');
    try {
      setResult(await call('files.binary.status', { path: file, transfer_id: transferId }));
    } catch (e) {
      setErr(String(e));
    }
  }
  return (
    <>
      <section className="panel">
        <div className="panelHead">
          <div>
            <span className="kicker">Workspace-contained</span>
            <h3>File browser</h3>
          </div>
          <Badge ok label="Root sandbox enforced" />
        </div>
        <div className="formRow">
          <input value={path} onChange={(e) => setPath(e.target.value)} placeholder="Workspace directory" />
          <button className="primary" onClick={() => void list()}>
            List
          </button>
        </div>
        <div className="fileList">
          {entries.map((e) => (
            <button
              key={e.path}
              onClick={() => {
                if (e.is_dir) {
                  setPath(e.path);
                  setOpPath(e.path);
                } else {
                  setFile(e.path);
                  setOpPath(e.path);
                }
              }}
            >
              <b>{e.is_dir ? 'DIR' : 'FILE'}</b>
              <span>{e.name}</span>
              <small>{e.is_dir ? '' : `${e.size} B`}</small>
            </button>
          ))}
        </div>
      </section>
      <section className="panel">
        <div className="panelHead">
          <div>
            <span className="kicker">Filesystem operations</span>
            <h3>Create / move / delete</h3>
          </div>
          <Badge ok label="Workspace only" />
        </div>
        <div className="formGrid">
          <label>
            Source / new path
            <input
              value={opPath}
              onChange={(e) => setOpPath(e.target.value)}
              placeholder="Absolute path inside a workspace"
            />
          </label>
          <label>
            Move destination
            <input
              value={destination}
              onChange={(e) => setDestination(e.target.value)}
              placeholder="Absolute destination inside a workspace"
            />
          </label>
        </div>
        <div className="buttonRow">
          <button className="primary" onClick={() => void mutate('mkdir')}>
            Create directory
          </button>
          <button className="ghost" onClick={() => void mutate('move')}>
            Move
          </button>
          <button className="danger" onClick={() => void mutate('delete')}>
            Delete recursively
          </button>
        </div>
        <p className="hint">
          Configured workspace roots themselves cannot be moved or deleted. Binary transfer continues to use
          resumable bounded chunks and optional final SHA-256 verification.
        </p>
      </section>
      <section className="panel">
        <div className="formRow">
          <input value={file} onChange={(e) => setFile(e.target.value)} placeholder="Workspace file" />
          <button className="ghost" onClick={() => void read()}>
            Read text
          </button>
          <button className="ghost" onClick={() => void digest()}>
            SHA-256
          </button>
        </div>
        <div className="formRow">
          <input
            value={transferId}
            onChange={(e) => setTransferId(e.target.value)}
            placeholder="Binary upload transfer ID"
          />
          <button className="ghost" disabled={!transferId || !file} onClick={() => void uploadStatus()}>
            Upload offset/status
          </button>
        </div>
        <textarea
          className="editor"
          aria-label="File content editor"
          value={content}
          onChange={(e) => setContent(e.target.value)}
          rows={18}
        />
        <button className="primary" onClick={() => void write()}>
          Atomic write
        </button>
        <ResultView value={result} error={err} />
      </section>
    </>
  );
}
