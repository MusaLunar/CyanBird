import { useMemo, useState } from 'react';
import { Archive, ArrowDown, ArrowRight, Check, CircleAlert, CloudDownload, Copy, FolderOpen, Languages, LayoutGrid, Moon, RefreshCw, Search, Settings2, ShieldCheck, Sparkles, Sun, Upload, X } from 'lucide-react';
import { PROVIDERS } from '../types/domain';
import { sessions, sources, parseFixture } from '../providers/fixtures';
import { useI18n } from '../i18n/I18nProvider';
import { useWorkbenchState } from './useWorkbenchState';

export function App() {
  const { t, locale, setLocale } = useI18n();
  const state = useWorkbenchState();
  const [query, setQuery] = useState('');
  const [dark, setDark] = useState(false);
  const activeSource = sources.find((item) => item.provider === state.source)!;
  const source = PROVIDERS.find((item) => item.id === state.source)!;
  const target = PROVIDERS.find((item) => item.id === state.target)!;
  const visibleSessions = sessions.filter((item) => item.source === state.source && item.title.toLowerCase().includes(query.toLowerCase()));
  const selectedSessions = sessions.filter((item) => state.selected.includes(item.id)).map(parseFixture);
  const stats = useMemo(() => selectedSessions.reduce((result, item) => ({ messages: result.messages + item.messages.length, tools: result.tools + item.toolCalls.length }), { messages: 0, tools: 0 }), [selectedSessions]);

  return <div className={dark ? 'app dark' : 'app'} data-ark-theme="ark" data-ark-depth="moderate">
    <aside className="rail" aria-label="Primary navigation">
      <div className="rail-brand"><span className="bird-mark" aria-hidden="true"><i /><i /><i /></span><span className="rail-brand-name">CYANBIRD</span><span className="rail-brand-sub">SESSION SYSTEM</span></div>
      <div className="rail-divider" />
      <nav className="rail-nav">
        <div className="rail-section-label">WORKSPACE</div>
        <button className="rail-item active"><LayoutGrid size={16} aria-hidden="true"/><span>{t('nav.migration')}</span><b>01</b></button>
        <button className="rail-item"><Archive size={16} aria-hidden="true"/><span>{t('nav.backups')}</span><b>04</b></button>
        <button className="rail-item"><Settings2 size={16} aria-hidden="true"/><span>{t('nav.settings')}</span><b>··</b></button>
      </nav>
      <div className="rail-foot"><div className="rail-foot-label">LOCAL / WIN</div><div className="rail-foot-state"><span className="signal-dot" /> {t('ready')}</div><div className="rail-foot-rule" /><div className="rail-foot-meta">CYB-OPS / 0.1.0</div></div>
    </aside>

    <main className="stage">
      <header className="stage-header">
        <div className="stage-title"><div className="micro-label">CYANBIRD / MIGRATION WORKSPACE</div><h1>{locale === 'zh-CN' ? '把会话带到下一处工作区' : 'Move sessions forward'}</h1><p>{locale === 'zh-CN' ? '把上下文完整带走，源文件保持在原地。' : 'Carry context forward while keeping source files in place.'}</p></div>
        <div className="stage-actions"><button className="square-button" onClick={() => setDark(!dark)} aria-label="Toggle theme" title="Toggle theme">{dark ? <Sun size={16}/> : <Moon size={16}/>}</button><button className="language-button" onClick={() => setLocale(locale === 'zh-CN' ? 'en-US' : 'zh-CN')}><Languages size={15}/>{locale === 'zh-CN' ? '中 / EN' : 'EN / 中'}</button><button className="signal-button" title={t('scan')}><CloudDownload size={15}/>{t('scan')}</button></div>
      </header>

      <section className="status-strip" aria-label="Workspace status"><span><span className="signal-dot" /> LOCAL WORKSPACE CONNECTED</span><span className="status-rule" /><span>LAST SCAN · 02 MIN AGO</span><span className="status-spacer" /><span className="status-mark">C/B</span></section>

      <section className="workspace" aria-label="Migration workspace">
        <section className="source-zone zone"><div className="zone-header"><div><div className="zone-index">01</div><h2>{t('sources')}</h2><span className="zone-caption">DISCOVERED AGENTS</span></div><button className="quiet-button" aria-label={t('refresh')}><RefreshCw size={15}/></button></div><div className="source-list">{sources.map((item) => { const provider = PROVIDERS.find((entry) => entry.id === item.provider)!; return <button key={item.id} onClick={() => state.setSource(item.provider)} className={`source-line ${state.source === item.provider ? 'selected' : ''}`}><span className={`provider-mark ${item.provider}`}>{provider.mark}</span><span className="source-line-copy"><b>{provider.name}</b><small>{item.path}</small></span><span className={`state-dot ${item.status}`} /><span className="source-line-count">{item.status === 'unavailable' ? '—' : item.sessionCount}</span></button>; })}</div><div className="zone-footer"><FolderOpen size={14}/><span>{t('sourcePath')}</span><ArrowDown size={13}/></div></section>

        <section className="session-zone zone"><div className="zone-header"><div><div className="zone-index">02</div><h2>{source.name}</h2><span className="zone-caption">{activeSource.path}</span></div><div className="zone-status"><span className={`state-dot ${activeSource.status}`} /> {activeSource.status === 'ready' ? t('ready') : t('unavailable')}</div></div><div className="search-line"><Search size={15}/><input aria-label={t('search')} value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t('search')}/>{query && <button onClick={() => setQuery('')} aria-label="Clear search"><X size={14}/></button>}</div><div className="session-toolbar"><span>{visibleSessions.length} {locale === 'zh-CN' ? '个会话' : 'SESSIONS'}</span><button onClick={() => visibleSessions.forEach((item) => !state.selected.includes(item.id) && state.toggle(item.id))}>{t('selected')} {state.selected.length}</button></div><div className="session-list">{activeSource.status === 'unavailable' ? <div className="empty-state"><CircleAlert size={25}/><b>{t('unavailable')}</b><span>{locale === 'zh-CN' ? '该 Agent 尚未发现本地会话目录。' : 'No local session directory found yet.'}</span></div> : visibleSessions.map((item) => <button key={item.id} className={`session-line ${state.selected.includes(item.id) ? 'checked' : ''}`} onClick={() => state.toggle(item.id)}><span className="checkbox">{state.selected.includes(item.id) && <Check size={12}/>}</span><span className="session-copy"><b>{item.title}</b><small>{item.workspace}</small><span className="session-data"><em>{item.messageCount} {t('messages')}</em><em>{item.toolCallCount} {t('tools')}</em></span></span><span className="session-date">{item.updatedAt.slice(5, 10).replace('-', '/')}</span></button>)}</div></section>

        <section className="plan-zone zone"><div className="zone-header"><div><div className="zone-index">03</div><h2>{t('plan')}</h2><span className="zone-caption">TARGET CONFIGURATION</span></div><div className="zone-status"><span className="state-dot ready" /> {t('ready')}</div></div>{selectedSessions.length === 0 ? <div className="plan-empty"><div className="plan-empty-mark"><ArrowRight size={24}/></div><h3>{t('noSelection')}</h3><p>{locale === 'zh-CN' ? '从中间列表选择会话，生成目标格式预览。' : 'Select sessions to generate a target format preview.'}</p></div> : <><div className="route-line"><div className="route-node"><span className={`provider-mark ${source.id}`}>{source.mark}</span><div><b>{source.name}</b><small>{selectedSessions.length} {t('sessions')}</small></div></div><ArrowRight size={16}/><div className="route-node"><span className={`provider-mark ${target.id}`}>{target.mark}</span><div><b>{target.name}</b><small>{state.backup ? 'BACKUP ON' : 'BACKUP OFF'}</small></div></div></div><div className="plan-field"><label>{t('target')} <span>TARGET AGENT</span></label><div className="target-grid">{PROVIDERS.filter((item) => item.id !== state.source).map((item) => <button key={item.id} className={state.target === item.id ? 'chosen' : ''} onClick={() => state.setTarget(item.id)}><span className={`provider-mark ${item.id}`}>{item.mark}</span>{item.name}{state.target === item.id && <Check size={13}/>}</button>)}</div></div><div className="plan-field inline"><label>{t('conflict')} <span>WRITE POLICY</span></label><select value={state.policy} onChange={(event) => state.setPolicy(event.target.value as any)}><option value="new-id">{t('newId')}</option><option value="overwrite">{t('overwrite')}</option><option value="skip">{t('skip')}</option></select></div><label className="backup-toggle"><input type="checkbox" checked={state.backup} onChange={(event) => state.setBackup(event.target.checked)}/><span className="toggle-track"/><span>{t('backup')}</span></label><div className="stat-row"><div><b>{stats.messages}</b><span>{t('messages')}</span></div><div><b>{stats.tools}</b><span>{t('tools')}</span></div><div><b>{selectedSessions.length}</b><span>{t('sessions')}</span></div></div><div className="notice"><CircleAlert size={15}/><span>{t('warning')} <b>metadata.unknownField</b></span></div><div className="plan-actions"><button className="outline-button"><Copy size={15}/>{t('preview')}</button><button className="signal-button commit"><Upload size={15}/>{t('execute')}<ArrowRight size={15}/></button></div></>}</section>
      </section>

      <footer className="stage-footer"><span><ShieldCheck size={14}/> {t('safe')}</span><span className="footer-spacer" /><span><Sparkles size={13}/> BUILT FOR MULTI-AGENT WORKFLOWS</span></footer>
    </main>
  </div>;
}
