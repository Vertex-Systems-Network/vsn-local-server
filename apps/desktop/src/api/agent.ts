import { invoke } from '@tauri-apps/api/core';
import type { AgentCommand } from '../contracts';

export async function call<T>(command: AgentCommand, params: Record<string, unknown> = {}): Promise<T> {
  return invoke<T>('agent_call', { command, params });
}
