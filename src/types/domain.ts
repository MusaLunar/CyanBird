export type ProviderId = 'claude-code' | 'antigravity' | 'codex' | 'zcode' | 'dsh';
export type Locale = 'zh-CN' | 'en-US';
export type ConflictPolicy = 'new-id' | 'overwrite' | 'skip';
export type ProviderStatus = 'ready' | 'unavailable' | 'scanning';
export interface SourceLocation { id:string; provider:ProviderId; label:string; path:string; sessionCount:number; status:ProviderStatus; lastScanned?:string; }
export interface Message { id:string; role:'user'|'assistant'|'system'; content:string; createdAt:string; }
export interface ToolCall { id:string; name:string; input:unknown; output?:unknown; }
export interface NormalizedSession { id:string; title:string; source:ProviderId; workspace?:string; createdAt?:string; updatedAt?:string; messages:Message[]; toolCalls:ToolCall[]; reasoning:string[]; attachments:string[]; metadata:Record<string,unknown>; }
export interface SessionSummary { id:string; title:string; source:ProviderId; workspace?:string; updatedAt:string; messageCount:number; toolCallCount:number; status:'ready'|'warning'|'error'; }
export interface MigrationPlan { sourceProvider?:ProviderId; targetProvider:ProviderId; sessionIds:string[]; targetWorkspace:string; conflictPolicy:ConflictPolicy; createBackup:boolean; }
export const PROVIDERS: Array<{id:ProviderId;name:string;mark:string;description:string}> = [
 {id:'claude-code',name:'Claude Code',mark:'C',description:'Anthropic coding sessions'},
 {id:'antigravity',name:'Antigravity',mark:'A',description:'Google agent workspace'},
 {id:'codex',name:'Codex',mark:'✦',description:'ChatGPT coding sessions'},
 {id:'zcode',name:'Zcode',mark:'Z',description:'Zhipu agent sessions'},
 {id:'dsh',name:'DeepSeek Harness',mark:'D',description:'Resumable DSH sessions'}
];
