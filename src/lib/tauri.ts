import { invoke } from '@tauri-apps/api/core';
import type { SessionSummary, SourceLocation } from '../types/domain';
export interface Preferences { backupDirectory: string; exportDirectory: string; }
export interface StoredFile { original: string; stored: string; bytes: number; sha256: string; }
export interface BackupRecord { id: string; createdAt: string; provider: string; sessions: SessionSummary[]; files: StoredFile[]; directory: string; kind?: 'sessions' | 'target-index'; }
export interface ExportReceipt { directory: string; files: string[]; }
export const discoverSources = () => invoke<SourceLocation[]>('discover_sources');
export const discoverSessions = (provider: string) => invoke<SessionSummary[]>('discover_sessions', { provider });
export const getPreferences = () => invoke<Preferences>('get_preferences');
export const savePreferences = (value: Preferences) => invoke<Preferences>('save_preferences', { value });
export const createBackup = (provider: string, ids: string[]) => invoke<BackupRecord>('create_backup', { provider, ids });
export const exportMarkdown = (provider: string, ids: string[]) => invoke<ExportReceipt>('export_markdown', { provider, ids });
export const listBackups = () => invoke<BackupRecord[]>('list_backups');
export const openOutputDirectory = (kind: 'backup' | 'export') => invoke<void>('open_output_directory', { kind });
