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

/** Database results as a plain grid: `{columns, rows}` grids, arrays of objects, or `{rows|documents: [...]}`. */
export type Grid = { columns: string[]; rows: unknown[][] };
export function toGrid(value: unknown): Grid | null {
  if (!value || typeof value !== 'object') return null;
  const v = value as Record<string, unknown>;
  if (Array.isArray(v.columns) && Array.isArray(v.rows) && v.rows.every(Array.isArray))
    return { columns: v.columns.map(String), rows: v.rows as unknown[][] };
  const list = Array.isArray(value) ? value : Array.isArray(v.rows) ? v.rows : Array.isArray(v.documents) ? v.documents : null;
  if (!list || !list.length || !list.every((r) => r && typeof r === 'object' && !Array.isArray(r))) return null;
  const columns = [...new Set(list.flatMap((r) => Object.keys(r as object)))];
  return { columns, rows: list.map((r) => columns.map((c) => (r as Record<string, unknown>)[c] ?? null)) };
}
function csvCell(v: unknown) {
  const s = v === null || v === undefined ? '' : typeof v === 'object' ? JSON.stringify(v) : String(v);
  return /[",\r\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
}
export function gridToCsv(grid: Grid): string {
  return [grid.columns, ...grid.rows].map((r) => r.map(csvCell).join(',')).join('\r\n') + '\r\n';
}
/** RFC 4180 CSV: quoted fields, doubled quotes, CRLF or LF; blank lines are skipped. */
export function parseCsv(text: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let cell = '';
  let quoted = false;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (quoted) {
      if (c === '"' && text[i + 1] === '"') (cell += '"'), i++;
      else if (c === '"') quoted = false;
      else cell += c;
    } else if (c === '"') quoted = true;
    else if (c === ',') row.push(cell), (cell = '');
    else if (c === '\n' || c === '\r') {
      if (c === '\r' && text[i + 1] === '\n') i++;
      row.push(cell);
      if (row.some((x) => x !== '')) rows.push(row);
      (row = []), (cell = '');
    } else cell += c;
  }
  row.push(cell);
  if (row.some((x) => x !== '')) rows.push(row);
  return rows;
}
/** CSV text to insert payloads, using the first row as column names; empty cells become NULL. */
export function csvToInserts(text: string): Record<string, string | null>[] {
  const [head, ...body] = parseCsv(text.replace(/^﻿/, ''));
  if (!head) return [];
  return body.map((r) => Object.fromEntries(head.map((h, i) => [h.trim(), r[i] ? r[i] : null])));
}

/** Table designer: one column of a CREATE TABLE statement. */
export type DesignColumn = { name: string; type: string; primary: boolean; nullable: boolean; default: string };
export type SqlDialect = 'postgres' | 'mysql' | 'sqlite';
function quoteIdent(name: string, dialect: SqlDialect) {
  return dialect === 'mysql' ? `\`${name.replace(/`/g, '``')}\`` : `"${name.replace(/"/g, '""')}"`;
}
/** CREATE TABLE for the chosen engine; blank column names are skipped. */
export function createTableSql(table: string, columns: DesignColumn[], dialect: SqlDialect): string {
  const cols = columns.filter((c) => c.name.trim());
  const keys = cols.filter((c) => c.primary).map((c) => quoteIdent(c.name.trim(), dialect));
  const lines = cols.map((c) => {
    const parts = [quoteIdent(c.name.trim(), dialect), c.type.trim() || 'TEXT'];
    if (!c.nullable && !c.primary) parts.push('NOT NULL');
    if (c.default.trim()) parts.push(`DEFAULT ${c.default.trim()}`);
    return `  ${parts.join(' ')}`;
  });
  if (keys.length) lines.push(`  PRIMARY KEY (${keys.join(', ')})`);
  return `CREATE TABLE ${quoteIdent(table.trim() || 'new_table', dialect)} (\n${lines.join(',\n')}\n);\n`;
}
