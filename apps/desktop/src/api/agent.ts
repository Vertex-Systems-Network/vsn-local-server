import { invoke } from '@tauri-apps/api/core';
import type { AgentCommand } from '../contracts';

export async function call<T>(command: AgentCommand, params: Record<string, unknown> = {}): Promise<T> {
  return invoke<T>('agent_call', { command, params });
}

/** Start the bundled vsn-agent when nothing answers (per-user installs have no Windows service). */
export async function startAgent(): Promise<boolean> {
  return invoke<boolean>('agent_start');
}

/** Opens an https link in the default browser (the webview itself has no network access). */
export async function openUrl(url: string): Promise<void> {
  return invoke<void>('open_url', { url });
}

/** Route a space's name.test domain to a local port (one UAC prompt on Windows). Returns the exit code. */
export async function routeDomain(domain: string, port: number): Promise<number> {
  return invoke<number>('route_domain', { domain, port });
}
