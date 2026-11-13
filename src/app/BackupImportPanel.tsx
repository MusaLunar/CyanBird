import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { X } from 'lucide-react';
import { PROVIDERS, type ProviderId } from '../types/domain';
import type { BackupRecord } from '../lib/tauri';
import { useI18n } from '../i18n/I18nProvider';

interface ImportItem { sourceId: string; targetId: string; status: string; path: string; error?: string }
interface Props { backup: BackupRecord; onClose: () => void; onBusy: (busy: boolean) => void; onImported: () => void }
export function BackupImportPanel({ backup, onClose, onBusy, onImported }: Props) {
  const { locale } = useI18n();
  const tr = (zh: string, en: string) => locale === 'zh-CN' ? zh : en;
  const [ids, setIds] = useState(backup.sessions.map(s => s.id));
  const [target, setTarget] = useState<ProviderId>(PROVIDERS.find(p => p.id === backup.provider)?.id ?? 'claude-code');
  const [workspace, setWorkspace] = useState(backup.sessions[0]?.workspace ?? '');
  const [plan, setPlan] = useState<{ key: string; targetDirectory: string; messageCount: number }>();
  const [acknowledged, setAcknowledged] = useState(false);
  const [busy, setBusy] = useState(false);
  const requestLock = useRef(false);
  const [error, setError] = useState('');
  const [results, setResults] = useState<ImportItem[]>([]);
  const key = JSON.stringify([backup.id, ids, target, workspace]);
  useEffect(() => { setPlan(undefined); setAcknowledged(false); setResults([]); }, [key]);
  const lock = (value: boolean) => { requestLock.current=value;setBusy(value); onBusy(value); };
  const parameters = { backupId: backup.id, ids, target, workspace };
  const prepare = async () => {
    if (requestLock.current) return;
    lock(true); setError('');
    try { const result = await invoke<{targetDirectory:string;messageCount:number}>('preview_backup_import', parameters); setPlan({key,...result}); }
    catch (reason) { setError(String(reason)); } finally { lock(false); }
  };
  const execute = async () => {
    if (requestLock.current || !acknowledged || plan?.key !== key) return;
    lock(true); setError('');
    try { const result = await invoke<{items:ImportItem[]}>('import_backup', {...parameters, acknowledged:true}); setResults(result.items); setPlan(undefined); setAcknowledged(false); onImported(); }
    catch (reason) { setError(String(reason)); } finally { lock(false); }
  };
  return <section className="backup-import-panel" aria-label={tr('从备份导入 Agent','Import backup into agent')}>
    <header><h3>{tr('从备份导入 Agent','Import backup into agent')}</h3><button className="square-button" disabled={busy} onClick={onClose} aria-label={tr('关闭备份导入','Close backup import')}><X size={16}/></button></header>
    <fieldset disabled={busy}><legend>{tr('选择要导入的会话','Select sessions to import')}</legend><div className="backup-import-sessions">{backup.sessions.map(session => <label key={session.id}><input type="checkbox" checked={ids.includes(session.id)} onChange={() => setIds(current => current.includes(session.id) ? current.filter(id => id !== session.id) : [...current,session.id])}/><span>{session.title || tr('未命名会话','Untitled session')}<small>{session.workspace}</small></span></label>)}</div></fieldset>
    <div className="backup-import-fields"><label className="settings-field">{tr('导入目标 Agent','Destination agent')}<select disabled={busy} value={target} onChange={event => setTarget(event.target.value as ProviderId)}>{PROVIDERS.map(provider => <option key={provider.id} value={provider.id}>{provider.name}</option>)}</select></label><label className="settings-field">{tr('导入项目文件夹','Import project directory')}<input disabled={busy} value={workspace} onChange={event => setWorkspace(event.target.value)} placeholder="E:/Projects/MyProject"/></label></div>
    <p className="zone-help">{target === 'antigravity' ? tr('请先完全退出 Antigravity。只导入用户消息和助手正文，工具与推理原始记录保留在备份中。','Fully quit Antigravity first. Only user/assistant text is imported; raw tools and reasoning remain in the backup.') : tr('从备份文件读取历史内容，生成新会话；不会覆盖已有会话。','Reads history from the backup and creates new sessions without replacing existing ones.')}</p>
    {plan?.key === key && <div className="migration-review"><p>{plan.messageCount} {tr('条历史消息','historical messages')}</p><p className="output-path">{plan.targetDirectory}</p><label><input type="checkbox" checked={acknowledged} disabled={busy} onChange={event => setAcknowledged(event.target.checked)}/>{tr('确认导入为新会话，不覆盖已有会话。','Import as new sessions without overwriting existing sessions.')}</label></div>}
    {error && <p role="alert" className="backup-import-error">{error}</p>}
    <div className="backup-import-actions"><button className="outline-button" disabled={busy || !ids.length || !workspace.trim()} onClick={prepare}>{tr('预览导入','Preview import')}</button><button className="signal-button" disabled={busy || !acknowledged || plan?.key !== key} onClick={execute}>{busy ? tr('处理中…','Working…') : tr('确认导入','Import sessions')}</button></div>
    {!!results.length && <ul className="migration-results" role="status">{results.map(item => <li key={item.targetId}><b>{backup.sessions.find(s => s.id === item.sourceId)?.title} · {item.status === 'written' ? tr('已导入','Imported') : tr('失败','Failed')}</b><small>{item.error || item.path}</small></li>)}</ul>}
  </section>;
}
