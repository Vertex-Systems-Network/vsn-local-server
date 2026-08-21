import type {
  ContainerBackend,
  MachineIdentity,
  ProcessInfo,
  RemoteConfig,
  RuntimeDetection,
} from './contracts';

export type AgentConnection = 'connecting' | 'online' | 'offline';

export type AgentStatus = {
  health?: {
    healthy?: boolean;
    detail?: string;
    service?: string;
  };
  security?: {
    device_identity_ready?: boolean;
    ipc_secret_ready?: boolean;
    secure_store?: string;
  };
};

export type OverviewState = {
  connection: AgentConnection;
  machine: MachineIdentity | null;
  status: AgentStatus | null;
  runtimes: RuntimeDetection[];
  processes: ProcessInfo[];
  backends: ContainerBackend[];
  remote: RemoteConfig | null;
  coreError: string;
  optionalErrors: string[];
};

export type OptionalOverviewResults = {
  runtimes: PromiseSettledResult<RuntimeDetection[]>;
  processes: PromiseSettledResult<ProcessInfo[]>;
  backends: PromiseSettledResult<ContainerBackend[]>;
  remote: PromiseSettledResult<RemoteConfig>;
};

export const initialOverviewState: OverviewState = {
  connection: 'connecting',
  machine: null,
  status: null,
  runtimes: [],
  processes: [],
  backends: [],
  remote: null,
  coreError: '',
  optionalErrors: [],
};

function errorText(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}

function rejected(label: string, result: PromiseSettledResult<unknown>, errors: string[]): void {
  if (result.status === 'rejected') errors.push(`${label}: ${errorText(result.reason)}`);
}

export function onlineOverviewState(
  machine: MachineIdentity,
  status: AgentStatus,
  results: OptionalOverviewResults,
): OverviewState {
  const optionalErrors: string[] = [];
  rejected('runtimes', results.runtimes, optionalErrors);
  rejected('processes', results.processes, optionalErrors);
  rejected('containers', results.backends, optionalErrors);
  rejected('remote', results.remote, optionalErrors);

  return {
    connection: 'online',
    machine,
    status,
    runtimes: results.runtimes.status === 'fulfilled' ? results.runtimes.value : [],
    processes:
      results.processes.status === 'fulfilled' ? results.processes.value.slice(0, 250) : [],
    backends: results.backends.status === 'fulfilled' ? results.backends.value : [],
    remote: results.remote.status === 'fulfilled' ? results.remote.value : null,
    coreError: '',
    optionalErrors,
  };
}

export function offlineOverviewState(error: unknown): OverviewState {
  return {
    connection: 'offline',
    machine: null,
    status: null,
    runtimes: [],
    processes: [],
    backends: [],
    remote: null,
    coreError: `Agent unavailable: ${errorText(error)}`,
    optionalErrors: [],
  };
}

export function connectionLabel(connection: AgentConnection): string {
  if (connection === 'online') return 'Agent online';
  if (connection === 'offline') return 'Agent offline';
  return 'Agent connecting';
}
