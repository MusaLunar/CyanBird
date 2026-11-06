import type { NormalizedSession, ProviderId, SessionSummary, SourceLocation } from '../types/domain'; import { PROVIDERS } from '../types/domain';
const base = new Date('2026-11-06T08:00:00Z').toISOString();
export const sources:SourceLocation[] = PROVIDERS.map((p,i)=>({id:p.id,provider:p.id,label:p.name,path:`C:/Users/admin/.${p.id}/sessions`,sessionCount:[24,0,12,8,17][i],status:i===1?'unavailable':'ready',lastScanned:base}));
export const sessions:SessionSummary[] = [
 {id:'cb-claude-01',title:'Refactor auth middleware',source:'claude-code',workspace:'C:/Projects/atlas',updatedAt:'2026-11-06T07:42:00Z',messageCount:38,toolCallCount:16,status:'ready'},
 {id:'cb-claude-02',title:'CyanBird migration design',source:'claude-code',workspace:'E:/Programming/CyanBird',updatedAt:'2026-11-05T22:18:00Z',messageCount:24,toolCallCount:9,status:'ready'},
 {id:'cb-codex-01',title:'Optimize image pipeline',source:'codex',workspace:'C:/Projects/vision',updatedAt:'2026-11-05T16:04:00Z',messageCount:52,toolCallCount:21,status:'warning'},
 {id:'cb-dsh-01',title:'Ship the new parser',source:'dsh',workspace:'C:/Projects/shipyard',updatedAt:'2026-11-04T11:38:00Z',messageCount:71,toolCallCount:34,status:'ready'},
 {id:'cb-zcode-01',title:'API contract review',source:'zcode',workspace:'C:/Projects/console',updatedAt:'2026-11-03T09:12:00Z',messageCount:19,toolCallCount:7,status:'ready'}
];
export function parseFixture(summary:SessionSummary):NormalizedSession { return {id:summary.id,title:summary.title,source:summary.source,workspace:summary.workspace,createdAt:summary.updatedAt,updatedAt:summary.updatedAt,messages:Array.from({length:summary.messageCount},(_,i)=>({id:`${summary.id}-m${i}`,role:i%2?'assistant':'user',content:i===0?summary.title:'Session content preserved by CyanBird.',createdAt:summary.updatedAt})),toolCalls:Array.from({length:summary.toolCallCount},(_,i)=>({id:`${summary.id}-t${i}`,name:i%2?'read_file':'run_command',input:{},output:{ok:true}})),reasoning:['Reasoning trace preserved'],attachments:[],metadata:{providerVersion:'fixture',unknownField:'preserved'}}; }
