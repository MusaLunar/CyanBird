import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { discoverSessions, discoverSources } from './tauri';
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
describe('native discovery contract', () => {
  beforeEach(() => { vi.mocked(invoke).mockReset(); });
  it('preserves an empty result without substituting example sessions', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    expect(await discoverSessions('antigravity')).toEqual([]);
    expect(invoke).toHaveBeenCalledWith('discover_sessions', { provider: 'antigravity' });
  });
  it('surfaces native discovery failures', async () => {
    vi.mocked(invoke).mockImplementation(async () => { throw new Error('permission denied'); });
    const outcome = await discoverSources().then(() => 'unexpected success', (error: Error) => error.message);
    expect(outcome).toBe('permission denied');
  });
});
