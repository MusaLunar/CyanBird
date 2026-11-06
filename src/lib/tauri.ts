import { invoke } from '@tauri-apps/api/core';
import type { SessionSummary, SourceLocation } from '../types/domain';
import { sessions as fixtureSessions, sources as fixtureSources } from '../providers/fixtures';

export async function discoverSources(): Promise<SourceLocation[]> {
  try { return await invoke<SourceLocation[]>('discover_sources'); } catch { return fixtureSources; }
}
export async function discoverSessions(provider: string): Promise<SessionSummary[]> {
  try { return await invoke<SessionSummary[]>('discover_sessions', { provider }); } catch { return fixtureSessions.filter((session) => session.source === provider); }
}
