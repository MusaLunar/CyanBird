import { invoke } from '@tauri-apps/api/core';
import type { SessionSummary, SourceLocation } from '../types/domain';
export function discoverSources(): Promise<SourceLocation[]> {
  return invoke<SourceLocation[]>('discover_sources');
}
export function discoverSessions(provider: string): Promise<SessionSummary[]> {
  return invoke<SessionSummary[]>('discover_sessions', { provider });
}
