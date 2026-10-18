import {
  attentionItems,
  createTableSql,
  csvToInserts,
  filterProcesses,
  gridToCsv,
  parseCsv,
  resultFacts,
  scalarText,
  toGrid,
} from '../src/screenLogic.js';

// Same dependency-free style as agentOverviewState.acceptance.ts (no @types/node in the locked graph).
function same(actual: unknown, expected: unknown, message = 'values differ') {
  const a = JSON.stringify(actual);
  const e = JSON.stringify(expected);
  if (a !== e) throw new Error(`${message}: expected ${e}, got ${a}`);
}
declare const console: { log: (message: string) => void };

const ready = { security: { device_identity_ready: true, ipc_secret_ready: true } };
const docker = { id: 'docker', installed: true, daemon_reachable: true };

// Needs attention
same(attentionItems(ready, [docker], null), [], 'healthy machine has nothing to attend to');
const broken = attentionItems({}, [{ id: 'podman', installed: true, daemon_reachable: false }], {
  enabled: true,
  poll_interval_ms: 1000,
  allow_remote_terminal: false,
  allow_remote_file_write: false,
  allow_remote_database_query: false,
  allow_remote_preview_interactive: false,
});
same(
  broken.map((a) => a.tone),
  ['danger', 'danger', 'warning', 'warning'],
  'identity, IPC, daemon and remote issues are all reported, most severe first',
);
same(attentionItems(ready, [], null)[0].title, 'No container engine');

// Processes filter + sort
const procs = [
  { pid: 30, name: 'node', command: 'node server.js' },
  { pid: 4, name: 'System' },
  { pid: 12, name: 'php', command: 'php -S 127.0.0.1:8000' },
];
same(filterProcesses(procs, '', 'pid').map((p) => p.pid), [4, 12, 30]);
same(filterProcesses(procs, '', 'name').map((p) => p.name), ['node', 'php', 'System']);
same(filterProcesses(procs, '8000', 'name').map((p) => p.name), ['php'], 'matches command');
same(filterProcesses(procs, ' SYS ', 'name').map((p) => p.pid), [4], 'case-insensitive, trimmed');
same(filterProcesses(procs, '12', 'pid').map((p) => p.pid), [12], 'matches PID');

// Result summary
same(resultFacts({ state: 'Running', pid: 7, tls: true, nested: { a: 1 }, note: null }), [
  ['State', 'Running'],
  ['Pid', '7'],
  ['Tls', 'Yes'],
  ['Note', '—'],
]);
same(resultFacts([1, 2]), []);
same(scalarText(false), 'No');
same(scalarText({}), null);

// Database export/import
same(toGrid({ columns: ['id', 'n'], rows: [[1, 'a']], row_count: 1 }), { columns: ['id', 'n'], rows: [[1, 'a']] });
same(toGrid([{ a: 1 }, { b: 2 }]), { columns: ['a', 'b'], rows: [[1, null], [null, 2]] }, 'object rows');
same(toGrid({ documents: [{ _id: 'x' }] }), { columns: ['_id'], rows: [['x']] }, 'mongo documents');
same(toGrid({ state: 'ok' }), null);
same(
  gridToCsv({ columns: ['a', 'b'], rows: [['x,y', 'say "hi"'], [null, { k: 1 }]] }),
  'a,b\r\n"x,y","say ""hi"""\r\n,"{""k"":1}"\r\n',
);
same(parseCsv('a,b\r\n"x,y","say ""hi"""\r\n\n"multi\nline",2'), [
  ['a', 'b'],
  ['x,y', 'say "hi"'],
  ['multi\nline', '2'],
]);
same(csvToInserts('﻿name, age\nAda,36\nBob'), [
  { name: 'Ada', age: '36' },
  { name: 'Bob', age: null },
]);

// Table designer
const designCols = [
  { name: 'id', type: 'BIGINT', primary: true, nullable: false, default: '' },
  { name: 'email', type: 'VARCHAR(255)', primary: false, nullable: false, default: '' },
  { name: 'note', type: 'TEXT', primary: false, nullable: true, default: "''" },
  { name: ' ', type: 'TEXT', primary: false, nullable: true, default: '' },
];
same(
  createTableSql('users', designCols, 'postgres'),
  'CREATE TABLE "users" (\n  "id" BIGINT,\n  "email" VARCHAR(255) NOT NULL,\n  "note" TEXT DEFAULT \'\',\n  PRIMARY KEY ("id")\n);\n',
);
same(createTableSql('t`x', [designCols[0]], 'mysql'), 'CREATE TABLE `t``x` (\n  `id` BIGINT,\n  PRIMARY KEY (`id`)\n);\n');

console.log('screenLogic: all assertions passed');
