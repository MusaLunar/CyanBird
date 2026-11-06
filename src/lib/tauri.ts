import { invoke } from '@tauri-apps/api/core';
import type { SourceLocation } from '../types/domain';
import { sources } from '../providers/fixtures';
export async function discoverSources():Promise<SourceLocation[]> { try { return await invoke<SourceLocation[]>('discover_sources'); } catch { return sources; } }
