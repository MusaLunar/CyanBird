import { useState } from 'react';
import type { ConflictPolicy, ProviderId, SessionSummary } from '../types/domain';
export const sessionKey = (session: Pick<SessionSummary, 'source' | 'id'>) => `${session.source}:${session.id}`;
export function useWorkbenchState() {
  const [source, setSourceState] = useState<ProviderId>('claude-code');
  const [selected, setSelected] = useState<string[]>([]);
  const [target, setTarget] = useState<ProviderId>('dsh');
  const [policy, setPolicy] = useState<ConflictPolicy>('new-id');
  const [backup, setBackup] = useState(true);
  const setSource = (next: ProviderId) => { setSourceState(next); setSelected([]); if (next === target) setTarget(next === 'dsh' ? 'claude-code' : 'dsh'); };
  const toggle = (id: string) => setSelected((items) => items.includes(id) ? items.filter((item) => item !== id) : [...items, id]);
  return { source, selected, target, policy, backup, setSource, setTarget, setPolicy, setBackup, toggle, setSelected };
}
