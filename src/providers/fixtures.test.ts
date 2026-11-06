import { describe, expect, it } from 'vitest';
import { PROVIDERS } from '../types/domain';
import { sources, sessions, parseFixture } from './fixtures';
describe('provider fixtures',()=>{ it('registers five providers',()=>expect(PROVIDERS).toHaveLength(5)); it('preserves normalized counts',()=>{const s=parseFixture(sessions[0]); expect(s.messages).toHaveLength(sessions[0].messageCount); expect(s.toolCalls).toHaveLength(sessions[0].toolCallCount); expect(s.metadata.unknownField).toBe('preserved')}); it('discovers source locations',()=>expect(sources.map(s=>s.provider)).toEqual(PROVIDERS.map(p=>p.id))) });
