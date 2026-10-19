import { useEffect, useState } from 'react';
import { call } from '../api/agent';
import { formatBytes } from '../screens/Analytics';

type Hot = { function: string; location: string; self_ms: number; percent: number };
type Profile = {
  name: string;
  path: string;
  bytes: number;
  modified_unix: number;
  kind: 'node' | 'php';
  total_ms: number | null;
  hot: Hot[];
};

/** DVF-10: profiles written by processes started with "Profile" on, with Node hot spots. */
export function SpaceProfiles({ root, refresh }: { root: string; refresh: number }) {
  const [list, setList] = useState<Profile[]>([]);
  const [open, setOpen] = useState<string | null>(null);

  useEffect(() => {
    call<Profile[]>('space.profiles', { path: root })
      .then((l) => {
        setList(l);
        setOpen((cur) => cur ?? l.find((p) => p.kind === 'node')?.name ?? null);
      })
      .catch(() => setList([]));
  }, [root, refresh]);

  if (list.length === 0)
    return (
      <p className="hint">
        No profiles yet. Start a command with “Profile” on; Node writes a .cpuprofile when the process
        exits, PHP writes cachegrind files when Xdebug is installed.
      </p>
    );
  const shown = list.find((p) => p.name === open);
  return (
    <>
      <ul className="historyList">
        {list.map((p) => (
          <li key={p.name}>
            <button className="linkButton" onClick={() => setOpen(p.name)}>
              <b>{p.name}</b>
            </button>
            <small>
              {p.kind === 'node' ? 'Node' : 'PHP (open in KCachegrind/QCachegrind)'} · {formatBytes(p.bytes)} ·{' '}
              {new Date(p.modified_unix * 1000).toLocaleString()}
              {p.total_ms != null ? ` · ${(p.total_ms / 1000).toFixed(1)} s sampled` : ''}
            </small>
          </li>
        ))}
      </ul>
      {shown && shown.hot.length > 0 && (
        <table className="dataTable" aria-label={`Slowest functions in ${shown.name}`}>
          <thead>
            <tr>
              <th scope="col">Function</th>
              <th scope="col">Where</th>
              <th scope="col">Self time</th>
              <th scope="col">Share</th>
            </tr>
          </thead>
          <tbody>
            {shown.hot.map((h) => (
              <tr key={`${h.function}-${h.location}`}>
                <td>{h.function}</td>
                <td>
                  <small>{h.location || '—'}</small>
                </td>
                <td>{h.self_ms.toFixed(1)} ms</td>
                <td>{h.percent.toFixed(1)}%</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </>
  );
}
