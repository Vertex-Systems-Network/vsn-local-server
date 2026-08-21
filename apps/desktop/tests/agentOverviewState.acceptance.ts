import {
  connectionLabel,
  initialOverviewState,
  offlineOverviewState,
  onlineOverviewState,
} from '../src/agentOverviewState.js';

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

function fulfilled<T>(value: T): PromiseFulfilledResult<T> {
  return { status: 'fulfilled', value };
}

function rejected(reason: unknown): PromiseRejectedResult {
  return { status: 'rejected', reason };
}

assert(initialOverviewState.connection === 'connecting', 'initial state must be connecting');
assert(connectionLabel('connecting') === 'Agent connecting', 'connecting label mismatch');
assert(connectionLabel('online') === 'Agent online', 'online label mismatch');
assert(connectionLabel('offline') === 'Agent offline', 'offline label mismatch');

const machine = {
  device_id: 'device-0204-acceptance',
  display_name: 'Acceptance machine',
  os: 'linux',
  public_key: 'public-key',
} as any;
const status = {
  health: { healthy: true, detail: 'ready', service: 'vsn-core' },
  security: { device_identity_ready: true, ipc_secret_ready: true, secure_store: 'test' },
};
const processes = Array.from({ length: 300 }, (_, pid) => ({ pid, name: `p-${pid}` })) as any[];

const online = onlineOverviewState(machine, status, {
  runtimes: fulfilled([{ id: 'php', installed: true }] as any[]),
  processes: fulfilled(processes as any),
  backends: fulfilled([{ id: 'docker', installed: false }] as any[]),
  remote: fulfilled({ enabled: false } as any),
});
assert(online.connection === 'online', 'successful core refresh must be online');
assert(online.machine?.device_id === machine.device_id, 'online state must retain current machine');
assert(online.status?.health?.healthy === true, 'online state must retain current status');
assert(online.processes.length === 250, 'process snapshot must remain bounded to 250');
assert(online.optionalErrors.length === 0, 'fully successful refresh must have no optional errors');

const partial = onlineOverviewState(machine, status, {
  runtimes: rejected(new Error('runtime provider unavailable')),
  processes: fulfilled([] as any[]),
  backends: rejected('container daemon probe failed'),
  remote: fulfilled({ enabled: false } as any),
});
assert(partial.connection === 'online', 'optional subsystem failure must not mark Agent offline');
assert(partial.machine?.device_id === machine.device_id, 'optional failure must preserve core machine state');
assert(partial.runtimes.length === 0, 'failed optional runtime data must fail closed to an empty list');
assert(partial.backends.length === 0, 'failed optional container data must fail closed to an empty list');
assert(partial.optionalErrors.length === 2, 'optional failures must remain visible to the operator');
assert(partial.optionalErrors[0]?.startsWith('runtimes:'), 'runtime failure must be labeled');
assert(partial.optionalErrors[1]?.startsWith('containers:'), 'container failure must be labeled');

const offline = offlineOverviewState(new Error('connection refused'));
assert(offline.connection === 'offline', 'core bridge failure must mark Agent offline');
assert(offline.machine === null, 'offline transition must clear stale machine data');
assert(offline.status === null, 'offline transition must clear stale status data');
assert(offline.runtimes.length === 0, 'offline transition must clear stale runtime data');
assert(offline.processes.length === 0, 'offline transition must clear stale process data');
assert(offline.backends.length === 0, 'offline transition must clear stale container data');
assert(offline.remote === null, 'offline transition must clear stale remote data');
assert(offline.coreError.includes('connection refused'), 'offline state must expose the core bridge error');

console.log('02.04 Agent Overview state acceptance passed');
