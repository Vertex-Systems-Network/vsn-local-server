import type { ContainerBackend, ProcessInfo, RemoteConfig } from './contracts';

export type Attention = { tone: 'danger' | 'warning' | 'info'; title: string; detail: string };

/** Items for the Overview "Needs attention" panel, most severe first. */
export function attentionItems(status: any, backends: ContainerBackend[], remote: RemoteConfig | null): Attention[] {
  const items: Attention[] = [];
  if (!status?.security?.device_identity_ready)
    items.push({ tone: 'danger', title: 'Device identity not ready', detail: 'The agent has not provisioned a device key.' });
  if (!status?.security?.ipc_secret_ready)
    items.push({ tone: 'danger', title: 'Authenticated IPC not ready', detail: 'Desktop ↔ agent calls are not authenticated yet.' });
  for (const b of backends.filter((b) => b.installed && b.daemon_reachable === false))
    items.push({ tone: 'warning', title: `${b.id} daemon unreachable`, detail: 'Installed but not responding. Start the engine.' });
  if (!backends.some((b) => b.installed))
    items.push({ tone: 'info', title: 'No container engine', detail: 'Install Docker or Podman to manage containers.' });
  if (remote?.enabled && !remote.control_plane_url)
    items.push({ tone: 'warning', title: 'Remote enabled without control plane', detail: 'Set a control-plane URL in Remote.' });
  return items;
}

export type ProcessSort = 'pid' | 'name';

/** Processes table: case-insensitive filter over PID, name and command, then sort. */
export function filterProcesses(rows: ProcessInfo[], query: string, sort: ProcessSort): ProcessInfo[] {
  const q = query.trim().toLowerCase();
  const filtered = q ? rows.filter((p) => `${p.pid} ${p.name} ${p.command ?? ''}`.toLowerCase().includes(q)) : rows;
  return [...filtered].sort((a, b) => (sort === 'pid' ? a.pid - b.pid : a.name.localeCompare(b.name)));
}

/** ResultView: top-level scalar fields of an agent response, as label/value pairs. */
export function resultFacts(value: unknown): [string, string][] {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return [];
  return Object.entries(value as Record<string, unknown>).flatMap(([k, v]): [string, string][] => {
    const s = scalarText(v);
    return s === null ? [] : [[factLabel(k), s]];
  });
}
export function scalarText(v: unknown): string | null {
  if (v === null || v === undefined) return '—';
  if (typeof v === 'string' || typeof v === 'number') return String(v);
  if (typeof v === 'boolean') return v ? 'Yes' : 'No';
  return null;
}
function factLabel(k: string) {
  const s = k.replace(/_/g, ' ');
  return s.charAt(0).toUpperCase() + s.slice(1);
}
