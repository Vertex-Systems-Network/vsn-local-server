import { invoke } from '@tauri-apps/api/core';
import type { AgentCommand } from '../contracts';

export async function call<T>(command: AgentCommand, params: Record<string, unknown> = {}): Promise<T> {
  return invoke<T>('agent_call', { command, params });
}

/** Start the bundled vsn-agent when nothing answers (per-user installs have no Windows service). */
export async function startAgent(): Promise<boolean> {
  return invoke<boolean>('agent_start');
}
