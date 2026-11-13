import { useEffect, useMemo, useRef, useState } from 'react';
import { Archive, ArrowRight, Check, CircleAlert, Download, Eye, Minus, Square, ExternalLink, FolderOpen, Languages, LayoutGrid, Moon, RefreshCw, Search, Settings2, ShieldCheck, Sun, X } from 'lucide-react';
import { PROVIDERS, type SessionSummary, type SourceLocation } from '../types/domain';
import { createBackup, discoverSessions, discoverSources, exportMarkdown, getPreferences, listBackups, openOutputDirectory, savePreferences, type BackupRecord, type Preferences } from '../lib/tauri';
import { useI18n } from '../i18n/I18nProvider';
import { sessionKey, useWorkbenchState } from './useWorkbenchState';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { AgentIcon } from './AgentIcon';
import { BackupImportPanel } from './BackupImportPanel';

const APP_VERSION = '0.9.0-beta';
type Page = 'migration' | 'backups' | 'settings';
type Update = { status: 'idle' | 'checking' | 'latest' | 'available' | 'none' | 'error'; version?: string; url?: string };

export function App() {
  const { t, locale, setLocale } = useI18n();
  const tr = (zh: string, en: string) => locale === 'zh-CN' ? zh : en;
  const state = useWorkbenchState();
  const [page, setPage] = useState<Page>('migration');
  const [query, setQuery] = useState('');
  const [dark, setDark] = useState(() => localStorage.getItem('cyanbird-theme') === 'dark');
  const [sources, setSources] = useState<SourceLocation[]>([]);
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [backups, setBackups] = useState<BackupRecord[]>([]);
  const [backupFilter, setBackupFilter] = useState<'all' | 'sessions' | 'index'>('all');
  const [backupQuery, setBackupQuery] = useState('');
  const [selectedBackupId, setSelectedBackupId] = useState('');
  const [backupImport, setBackupImport] = useState<BackupRecord>();
  const [prefs, setPrefs] = useState<Preferences>({ backupDirectory: '', exportDirectory: '' });
  const [prefsLoaded, setPrefsLoaded] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [output, setOutput] = useState<{ path: string; kind: 'backup' | 'export' }>();
  const [loading, setLoading] = useState(true);
  const [scanning, setScanning] = useState(false);
  const [busy, setBusy] = useState(false);
  const [lastScan, setLastScan] = useState<Date>();
  const [preview, setPreview] = useState<SessionSummary>();
  const [previewText, setPreviewText] = useState('');
  const previewRequest = useRef(0);
  const showPreview = async (item: SessionSummary) => {
    const token = ++previewRequest.current;
    setPreview(item);
    setPreviewText('');
    try {
      const text = await invoke<string>('preview_session', { provider: item.source, id: item.id });
      if (token === previewRequest.current) setPreviewText(text);
    } catch (reason) {
      if (token === previewRequest.current) setPreviewText(String(reason));
    }
  };
  const windowAction = (action: 'minimize' | 'toggleMaximize' | 'close') => { getCurrentWindow()[action]().catch(fail); };
  const [workspace, setWorkspace] = useState('');
  const [migrationPlan, setMigrationPlan] = useState<{ key: string; targetDirectory: string; messageCount: number }>();
  const [acknowledged, setAcknowledged] = useState(false);
  const [migrationResults, setMigrationResults] = useState<Array<{ targetId: string; path: string; status: string; error?: string }>>([]);
  const [update, setUpdate] = useState<Update>({ status: 'idle' });
  const request = useRef(0);
  const actionLock = useRef(false);
  const source = PROVIDERS.find((item) => item.id === state.source)!;
  const target = PROVIDERS.find((item) => item.id === state.target)!;
  const activeSource = sources.find((item) => item.provider === state.source);
  const visible = sessions.filter((item) => `${item.title} ${item.workspace ?? ''}`.toLowerCase().includes(query.toLowerCase()));
  const selected = sessions.filter((item) => state.selected.includes(sessionKey(item)));
  const allSelected = visible.length > 0 && visible.every((item) => state.selected.includes(sessionKey(item)));
  const stats = useMemo(() => selected.reduce((result, item) => ({ messages: result.messages + item.messageCount, tools: result.tools + item.toolCallCount }), { messages: 0, tools: 0 }), [selected]);
  const fail = (reason: unknown) => setError(String(reason));
  const date = (value: string) => new Date(/^\d+$/.test(value) ? Number(value) : value).toLocaleString(locale);
  const refreshBackups = () => listBackups().then(setBackups).catch(fail);
  const sessionBackups = useMemo(() => backups.filter((b) => b.kind !== 'target-index'), [backups]);
  const indexBackups = useMemo(() => backups.filter((b) => b.kind === 'target-index'), [backups]);
  const filteredBackups = useMemo(() => {
    const q = backupQuery.trim().toLowerCase();
    return backups.filter((record) => {
      if (backupFilter === 'sessions' && record.kind === 'target-index') return false;
      if (backupFilter === 'index' && record.kind !== 'target-index') return false;
      if (!q) return true;
      const providerName = PROVIDERS.find((p) => p.id === record.provider)?.name ?? record.provider;
      const titles = record.sessions.map((s) => `${s.title} ${s.workspace ?? ''}`).join(' ');
      return `${record.id} ${providerName} ${titles}`.toLowerCase().includes(q);
    });
  }, [backups, backupFilter, backupQuery]);
  const activeBackup = useMemo(() => {
    if (backupImport && filteredBackups.some((b) => b.id === backupImport.id)) return backupImport;
    return filteredBackups.find((b) => b.id === selectedBackupId) ?? filteredBackups[0];
  }, [filteredBackups, selectedBackupId, backupImport]);
  const activeBackupProvider = activeBackup ? PROVIDERS.find((p) => p.id === activeBackup.provider) : undefined;
  const formatKb = (record: BackupRecord) => (record.files.reduce((n, f) => n + f.bytes, 0) / 1024).toFixed(1);
  const backupTitle = (record: BackupRecord) => (
    record.kind === 'target-index'
      ? tr('目标索引备份', 'Target index snapshot')
      : record.sessions.length === 1
        ? (record.sessions[0].title || tr('未命名会话', 'Untitled session'))
        : tr(`${record.sessions.length} 个会话备份`, `${record.sessions.length} backed-up sessions`)
  );
  const loadSessions = (provider: string) => {
    const token = ++request.current;
    setLoading(true);
    setSessions([]);
    setError('');
    return discoverSessions(provider)
      .then((items) => { if (token === request.current) setSessions(items); })
      .catch((reason) => { if (token === request.current) fail(reason); })
      .finally(() => { if (token === request.current) setLoading(false); });
  };
  const scan = () => {
    if (scanning) return;
    setScanning(true);
    discoverSources().then((items) => { setSources(items); setLastScan(new Date()); }).catch(fail).finally(() => setScanning(false));
    void loadSessions(state.source);
  };
  useEffect(() => {
    discoverSources().then((items) => { setSources(items); setLastScan(new Date()); }).catch(fail);
    getPreferences().then((value) => { setPrefs(value); setPrefsLoaded(true); }).catch(fail);
  }, []);
  useEffect(() => { void loadSessions(state.source); setPreview(undefined); }, [state.source]);
  useEffect(() => { if (page === 'backups') void refreshBackups(); }, [page]);
  useEffect(() => { localStorage.setItem('cyanbird-theme', dark ? 'dark' : 'light'); }, [dark]);

  const runAction = async (kind: 'backup' | 'export') => {
    if (actionLock.current || !selected.length || !prefsLoaded) return;
    actionLock.current = true;
    setBusy(true);
    setError('');
    setNotice('');
    setOutput(undefined);
    try {
      const ids = selected.map((item) => item.id);
      const result = kind === 'backup' ? await createBackup(state.source, ids) : await exportMarkdown(state.source, ids);
      setOutput({ path: result.directory, kind });
      setNotice(kind === 'backup' ? tr('备份已创建并校验。', 'Backup created and verified.') : tr('Markdown 已导出。', 'Markdown exported.'));
      if (kind === 'backup') await refreshBackups();
    } catch (reason) {
      fail(reason);
    } finally {
      actionLock.current = false;
      setBusy(false);
    }
  };
  const save = async (event: React.FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setError('');
    setNotice('');
    try {
      setPrefs(await savePreferences(prefs));
      setNotice(tr('设置已保存。', 'Settings saved.'));
    } catch (reason) {
      fail(reason);
    } finally {
      setBusy(false);
    }
  };
  const migrationKey = JSON.stringify([state.source, state.target, selected.map((s) => s.id), workspace]);
  useEffect(() => { setAcknowledged(false); setMigrationPlan(undefined); setMigrationResults([]); }, [migrationKey]);
  useEffect(() => { if (!workspace && selected[0]?.workspace) setWorkspace(selected[0].workspace); }, [selected[0]?.workspace]);

  const prepareMigration = async () => {
    setBusy(true);
    setError('');
    try {
      const plan = await invoke<{ targetDirectory: string; messageCount: number }>('preview_migration', {
        provider: state.source,
        ids: selected.map((s) => s.id),
        target: state.target,
        workspace
      });
      setMigrationPlan({ key: migrationKey, ...plan });
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };
  const migrate = async () => {
    if (actionLock.current || !acknowledged || migrationPlan?.key !== migrationKey) return;
    actionLock.current = true;
    setBusy(true);
    setError('');
    setNotice('');
    try {
      const result = await invoke<{ backupDirectory: string; items: typeof migrationResults }>('execute_migration', {
        provider: state.source,
        ids: selected.map((s) => s.id),
        target: state.target,
        workspace,
        acknowledged
      });
      setMigrationResults(result.items);
      const failed = result.items.filter((i) => i.status === 'failed').length;
      setNotice(tr(`已写入 ${result.items.length - failed} 项，失败 ${failed} 项。请在目标 Agent 中重新打开或恢复会话。`, `Wrote ${result.items.length - failed} sessions; ${failed} failed. Reopen or resume in the target agent.`));
      setOutput({ path: result.backupDirectory, kind: 'backup' });
      void refreshBackups();
    } catch (e) {
      fail(e);
    } finally {
      actionLock.current = false;
      setBusy(false);
    }
  };
  const checkUpdates = async () => {
    setUpdate({ status: 'checking' });
    try {
      const response = await fetch('https://api.github.com/repos/MusaLunar/CyanBird/releases/latest', { headers: { Accept: 'application/vnd.github+json' }, signal: AbortSignal.timeout(15000) });
      if (response.status === 404) { setUpdate({ status: 'none' }); return; }
      if (!response.ok) throw new Error(`GitHub ${response.status}`);
      const release = await response.json();
      if (typeof release.tag_name !== 'string') throw new Error('Invalid release');
      const parts = (v: string) => v.replace(/^v/, '').split(/[.-]/).map((n) => Number.parseInt(n) || 0);
      const current = parts(APP_VERSION), latest = parts(release.tag_name);
      const difference = latest.findIndex((n, i) => n !== (current[i] ?? 0));
      setUpdate({ status: difference >= 0 && latest[difference] > (current[difference] ?? 0) ? 'available' : 'latest', version: release.tag_name, url: `https://github.com/MusaLunar/CyanBird/releases/tag/${encodeURIComponent(release.tag_name)}` });
    } catch {
      setUpdate({ status: 'error' });
    }
  };
  const updateLabel = update.status === 'checking' ? t('updateChecking') : update.status === 'available' ? `${t('updateAvailable')} ${update.version}` : update.status === 'latest' ? t('updateLatest') : update.status === 'none' ? t('updateNone') : update.status === 'error' ? t('updateError') : t('updateCheck');
  const chooseSource = (id: typeof state.source) => { state.setSource(id); setWorkspace(''); setQuery(''); setPreview(undefined); };

  const pipelineStep = !selected.length ? 1 : !migrationPlan || migrationPlan.key !== migrationKey ? 2 : !acknowledged ? 3 : 4;

  return (
    <div className={dark ? 'app dark' : 'app'} data-theme={dark ? 'dark' : 'light'}>
      <div className="window-titlebar" data-tauri-drag-region>
        <div className="titlebar-brand" data-tauri-drag-region>
          <i className="titlebar-pip" aria-hidden="true" />
          <span data-tauri-drag-region>CyanBird</span>
          <small className="titlebar-tag" data-tauri-drag-region>v{APP_VERSION}</small>
        </div>
        <div className="titlebar-controls">
          <button aria-label={tr('最小化', 'Minimize')} onClick={() => windowAction('minimize')}><Minus size={14} /></button>
          <button aria-label={tr('最大化或还原', 'Maximize or restore')} onClick={() => windowAction('toggleMaximize')}><Square size={12} /></button>
          <button className="window-close" aria-label={tr('关闭应用', 'Close app')} onClick={() => windowAction('close')}><X size={16} /></button>
        </div>
      </div>

      <aside className="rail" aria-label={tr('主导航', 'Primary navigation')}>
        <div className="rail-brand">
          <img className="brand-logo" src="/cyanbird-logo.png" alt="CyanBird" />
          <span className="rail-brand-name">CYANBIRD</span>
          <span className="rail-brand-sub">SESSION RELAY // {APP_VERSION}</span>
        </div>
        <div className="rail-divider" />
        <nav className="rail-nav">
          {(['migration', 'backups', 'settings'] as const).map((name, idx) => (
            <button
              key={name}
              className={`rail-item ${page === name ? 'active' : ''}`}
              aria-current={page === name ? 'page' : undefined}
              onClick={() => setPage(name)}
            >
              <b aria-hidden="true">0{idx + 1}</b>
              {name === 'migration' ? <LayoutGrid size={16} /> : name === 'backups' ? <Archive size={16} /> : <Settings2 size={16} />}
              <span>{t(`nav.${name}`)}</span>
            </button>
          ))}
        </nav>
        <div className="rail-instrument" aria-hidden="true">
          <div className="rail-instrument-row">
            <span>SOURCE</span>
            <strong>{source.id.toUpperCase()}</strong>
          </div>
          <div className="rail-instrument-row">
            <span>TARGET</span>
            <strong>{target.id.toUpperCase()}</strong>
          </div>
          <div className="rail-instrument-row">
            <span>QUEUE</span>
            <strong>{String(selected.length).padStart(2, '0')}</strong>
          </div>
        </div>
        <div className="rail-foot">
          <div className="rail-foot-state">
            <i className={`signal-dot ${busy || scanning ? 'pulsing' : ''}`} aria-hidden="true" />
            <span>{busy ? tr('写入事务运行中', 'TRANSACTION ACTIVE') : scanning ? tr('目录扫描中', 'SCANNING DISKS') : 'LOCAL / WIN64'}</span>
          </div>
          <div className="rail-foot-rule" />
          <button className="update-link" onClick={checkUpdates} disabled={update.status === 'checking'}>
            <RefreshCw size={12} />
            <span>{updateLabel}</span>
          </button>
          <div className="rail-foot-meta">CyanBird / v{APP_VERSION}</div>
        </div>
      </aside>

      <main className="stage">
        <div className="stage-backdrop" aria-hidden="true">
          <span className="backdrop-sector" />
          <span className="backdrop-diamond" />
          <span className="backdrop-ring" />
          <span className="backdrop-watermark">{page === 'migration' ? source.id.toUpperCase() : page.toUpperCase()}</span>
        </div>

        <header className="stage-header">
          <div className="stage-title">
            <h1>{t(`nav.${page}`)}</h1>
            <p>
              {page === 'migration'
                ? tr('管理上下文，保留每一次对话。', 'Keep every conversation and its context.')
                : page === 'backups'
                  ? tr('查看已完成的备份与源文件校验记录。', 'Review completed snapshots and file checksums.')
                  : tr('管理本地存储、语言与外观。', 'Manage local storage, language and appearance.')}
            </p>
          </div>
          <div className="stage-actions">
            <button className="square-button" aria-label={tr('切换主题', 'Toggle theme')} onClick={() => setDark(!dark)}>
              {dark ? <Sun size={16} /> : <Moon size={16} />}
            </button>
            <button className="language-button" onClick={() => setLocale(locale === 'zh-CN' ? 'en-US' : 'zh-CN')}>
              <Languages size={15} />
              {locale === 'zh-CN' ? '中 / EN' : 'EN / 中'}
            </button>
            {page === 'migration' && (
              <button className="signal-button" disabled={scanning || busy} onClick={scan}>
                <RefreshCw size={15} />
                {scanning ? tr('扫描中…', 'Scanning…') : t('scan')}
              </button>
            )}
          </div>
        </header>

        {error && (
          <div className="message error-message" role="alert">
            <CircleAlert size={16} />
            <span>{error}</span>
            <button aria-label={tr('关闭错误', 'Dismiss error')} onClick={() => setError('')}><X size={15} /></button>
          </div>
        )}
        {notice && (
          <div className="message" role="status">
            <Check size={16} />
            <div>
              {notice}
              {output && <small className="output-path">{output.path}</small>}
            </div>
            {output && (
              <button className="outline-button" onClick={() => openOutputDirectory(output.kind).catch(fail)}>
                <FolderOpen size={14} />
                {tr('打开文件夹', 'Open folder')}
              </button>
            )}
          </div>
        )}
        {update.status === 'available' && update.url && (
          <div className="update-banner">
            <span>{updateLabel}</span>
            <a href={update.url} target="_blank" rel="noreferrer">{t('openRelease')}<ExternalLink size={13} /></a>
          </div>
        )}

        <div className="status-strip">
          <span>
            <i className="state-dot ready" aria-hidden="true" />
            {busy ? tr('正在处理，请勿关闭应用', 'Processing · keep the app open') : tr('本地文件 · 主会话', 'LOCAL FILES · MAIN SESSIONS')}
          </span>
          <span className="status-rule" aria-hidden="true" />
          <span className="status-mark">{source.name} → {target.name}</span>
          <span className="status-spacer" />
          {lastScan && <span>{tr('扫描于', 'Scanned at')} {lastScan.toLocaleTimeString(locale)}</span>}
        </div>

        {page === 'migration' && (
          <section className="workspace" aria-label={tr('迁移工作台', 'Migration workspace')}>
            <section className="source-zone zone">
              <div className="zone-header">
                <div>
                  <h2>{t('sources')}</h2>
                </div>
                <span className="zone-badge">{PROVIDERS.length} AGENTS</span>
              </div>
              <div className="source-list">
                {PROVIDERS.map((provider) => {
                  const item = sources.find((s) => s.provider === provider.id);
                  return (
                    <button
                      key={provider.id}
                      disabled={busy}
                      aria-pressed={state.source === provider.id}
                      className={`source-line ${state.source === provider.id ? 'selected' : ''}`}
                      onClick={() => chooseSource(provider.id)}
                    >
                      <AgentIcon provider={provider.id} />
                      <span className="source-line-copy">
                        <b>{provider.name}</b>
                        <small title={item?.path}>{item?.path || tr('等待扫描', 'Awaiting scan')}</small>
                      </span>
                      <span className="source-line-count">{item?.sessionCount ?? '—'}</span>
                    </button>
                  );
                })}
              </div>
              <p className="zone-help">{tr('子代理不列入主会话，原始文件保持不变。', 'Subagents are excluded from this list. Original files stay untouched.')}</p>
            </section>

            <section className="session-zone zone">
              <div className="zone-header">
                <div>
                  <h2>{source.name}</h2>
                  <span className="zone-caption" title={activeSource?.path}>{activeSource?.path}</span>
                </div>
                <span className="zone-badge">{visible.length} / {sessions.length}</span>
              </div>
              <div className="search-line">
                <Search size={15} />
                <input
                  aria-label={t('search')}
                  placeholder={tr('搜索标题或项目路径…', 'Search title or project…')}
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                />
                {query && <button aria-label={tr('清空搜索', 'Clear search')} onClick={() => setQuery('')}><X size={14} /></button>}
              </div>
              <div className="session-toolbar">
                <span>{loading ? tr('读取中…', 'Reading…') : `${visible.length} ${tr('个会话', 'sessions')}`}</span>
                <span className="selection-count">{tr(`已选 ${selected.length} 项`, `${selected.length} selected`)}</span>
                <button
                  disabled={busy || loading || !visible.length}
                  onClick={() => state.setSelected(allSelected ? state.selected.filter((key) => !visible.some((item) => sessionKey(item) === key)) : Array.from(new Set([...state.selected, ...visible.map(sessionKey)])))}
                >
                  {allSelected ? tr('取消全选', 'Clear selection') : tr('全选', 'Select all')}
                </button>
              </div>
              <div className="session-list" aria-busy={loading}>
                {loading ? (
                  <p className="empty-copy">{tr('正在读取会话元数据…', 'Reading session metadata…')}</p>
                ) : !visible.length ? (
                  <p className="empty-copy">{query ? tr('没有匹配的会话。', 'No matching sessions.') : tr('没有发现可读取的主会话。', 'No readable main sessions found.')}</p>
                ) : (
                  visible.map((item) => (
                    <div className="session-row" key={sessionKey(item)}>
                      <button
                        disabled={busy}
                        role="checkbox"
                        aria-checked={state.selected.includes(sessionKey(item))}
                        className={`session-line ${state.selected.includes(sessionKey(item)) ? 'checked' : ''}`}
                        onClick={() => state.toggle(sessionKey(item))}
                      >
                        <span className="checkbox" aria-hidden="true">{state.selected.includes(sessionKey(item)) && <Check size={12} />}</span>
                        <span className="session-copy">
                          <b title={item.title}>{item.title || item.id}</b>
                          <small title={item.workspace}>{item.workspace || tr('未记录项目路径', 'Project path not recorded')}</small>
                          <span className="session-data">
                            <em>{item.messageCount} {t('messages')}</em>
                            <em>{item.toolCallCount} {t('tools')}</em>
                          </span>
                        </span>
                        <time className="session-date">{new Date(Number(item.updatedAt) || item.updatedAt).toLocaleDateString(locale, { month: '2-digit', day: '2-digit' })}</time>
                      </button>
                      <button
                        className="preview-button"
                        disabled={busy}
                        aria-label={tr(`预览 ${item.title || item.id}`, `Preview ${item.title || item.id}`)}
                        onClick={() => showPreview(item)}
                      >
                        <Eye size={16} />
                      </button>
                    </div>
                  ))
                )}
              </div>
            </section>

            <section className="plan-zone zone">
              <div className="zone-header">
                <h2>{t('plan')}</h2>
                <div className="pipeline-track" aria-hidden="true">
                  <i className={pipelineStep >= 1 ? 'lit' : ''} />
                  <i className={pipelineStep >= 2 ? 'lit' : ''} />
                  <i className={pipelineStep >= 3 ? 'lit' : ''} />
                  <i className={pipelineStep >= 4 ? 'lit' : ''} />
                </div>
              </div>
              {!selected.length ? (
                <div className="plan-empty">
                  <div className="plan-empty-mark"><ArrowRight size={24} /></div>
                  <h3>{t('noSelection')}</h3>
                  <p>{tr('选择会话后，可以创建备份或导出 Markdown。', 'Select sessions to create a backup or export Markdown.')}</p>
                </div>
              ) : (
                <>
                  <div className="route-line">
                    <AgentIcon provider={source.id} />
                    <b>{source.name}</b>
                    <ArrowRight size={14} className="route-arrow" aria-hidden="true" />
                    <AgentIcon provider={target.id} />
                    <b>{target.name}</b>
                    <span className="status-spacer" />
                    <span className="route-pill">{selected.length} {tr('已选', 'selected')}</span>
                  </div>
                  <div className="stat-row">
                    <div><b>{stats.messages}</b><span>{t('messages')}</span></div>
                    <div><b>{stats.tools}</b><span>{t('tools')}</span></div>
                    <div><b>{selected.length}</b><span>{t('sessions')}</span></div>
                  </div>
                  <div className="migration-adapter">
                    <label>{t('target')}</label>
                    <div className="target-grid">
                      {PROVIDERS.filter((p) => p.id !== state.source).map((p) => (
                        <button
                          key={p.id}
                          disabled={busy}
                          title={p.name}
                          className={state.target === p.id ? 'chosen' : ''}
                          onClick={() => state.setTarget(p.id)}
                        >
                          <AgentIcon provider={p.id} />
                          {p.name}
                        </button>
                      ))}
                    </div>
                    <label className="settings-field">
                      {tr('目标项目文件夹', 'Target project directory')}
                      <input disabled={busy} value={workspace} onChange={(e) => setWorkspace(e.target.value)} placeholder="E:/Projects/MyProject" />
                    </label>
                    <p className="zone-help">
                      {tr('生成新会话 ID，保留源会话。迁入 Antigravity 前请完全退出客户端，完成后重新打开（支持 Desktop 2.29 SQLite 格式）。', 'Creates new session IDs and preserves sources. Fully quit Antigravity before importing, then reopen it (Desktop 2.29 SQLite format).')}
                    </p>
                    <button className="outline-button migration-preview-button" disabled={busy || !workspace} onClick={prepareMigration}>
                      {tr('生成迁移预览', 'Prepare migration')}
                    </button>
                    {migrationPlan?.key === migrationKey && (
                      <div className="migration-review">
                        <p>{target.name} · {migrationPlan.messageCount} {tr('条历史消息', 'historical messages')}</p>
                        <p className="output-path">{migrationPlan.targetDirectory}</p>
                        <label>
                          <input type="checkbox" disabled={busy} checked={acknowledged} onChange={(e) => setAcknowledged(e.target.checked)} />
                          {state.target === 'antigravity'
                            ? tr('确认转换差异：仅迁入用户消息和助手正文；工具、结果与推理原始记录保存在备份中；二进制附件不复制。', 'I acknowledge: only user messages and assistant text are imported; raw tools, results and reasoning remain in the backup; binary attachments are not copied.')
                            : tr('确认转换差异：工具、结果与推理以历史文本保留，不重放；二进制附件不复制；迁移前自动备份。', 'I acknowledge: tools, results and reasoning are historical text, not replayed; binary attachments are not copied; a backup is mandatory.')}
                        </label>
                      </div>
                    )}
                  </div>
                  <details open className="output-details">
                    <summary>{tr('输出位置', 'Output locations')}</summary>
                    <p>{tr('备份', 'Backup')}: {prefs.backupDirectory}</p>
                    <p>Markdown: {prefs.exportDirectory}</p>
                    <button className="text-button" onClick={() => setPage('settings')}>{tr('修改路径', 'Change paths')}</button>
                  </details>
                  <div className="plan-commit">
                    <p className="zone-help">
                      {tr('源文件不会被修改。', 'Source files are preserved.')}
                      {state.source === 'zcode' && (
                        <>
                          <br />
                          {tr('Zcode 备份包含整个会话数据库。', 'Zcode backups include the entire session database.')}
                        </>
                      )}
                    </p>
                    <div className="file-actions">
                      <button className="signal-button" disabled={busy || !prefsLoaded} onClick={() => runAction('backup')}>
                        <Archive size={15} />
                        {tr('创建备份', 'Create backup')}
                      </button>
                      <button className="outline-button" disabled={busy || !prefsLoaded} onClick={() => runAction('export')}>
                        <Download size={15} />
                        {tr('导出 Markdown', 'Export Markdown')}
                      </button>
                    </div>
                    <button className="outline-button execute-button" disabled={busy || !acknowledged || migrationPlan?.key !== migrationKey} onClick={migrate}>
                      {t('execute')}
                    </button>
                    {migrationResults.length > 0 && (
                      <ul className="migration-results">
                        {migrationResults.map((item) => (
                          <li key={item.targetId}>
                            <b>{item.status === 'failed' ? tr('失败', 'Failed') : tr('已写入', 'Written')}</b>
                            <small>{item.error || item.path}</small>
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>
                </>
              )}
            </section>
          </section>
        )}

        {page === 'backups' && (
          <section className="utility-page backups-shell" aria-label={t('nav.backups')}>
            <header className="backups-header">
              <div className="backups-heading">
                <h2>{tr('备份记录', 'Backup history')}</h2>
                <div className="backups-dir-bar">
                  <span className="directory-label" title={prefs.backupDirectory}>{prefs.backupDirectory}</span>
                  <button className="outline-button" onClick={() => openOutputDirectory('backup').catch(fail)}>
                    <FolderOpen size={14} />
                    {tr('打开备份目录', 'Open backup directory')}
                  </button>
                </div>
              </div>
              <div className="backups-header-actions">
                <button className="outline-button" onClick={refreshBackups}><RefreshCw size={15} />{t('refresh')}</button>
                <button className="signal-button" onClick={() => setPage('migration')}><Archive size={15} />{tr('选择会话创建备份', 'Select sessions to back up')}</button>
              </div>
            </header>

            <div className="backups-controls">
              <div className="backup-tabs" role="tablist" aria-label={tr('备份类型筛选', 'Backup type filter')}>
                <button type="button" role="tab" aria-selected={backupFilter === 'all'} className={backupFilter === 'all' ? 'active' : ''} onClick={() => setBackupFilter('all')}>
                  {tr('全部', 'All')} <span>{backups.length}</span>
                </button>
                <button type="button" role="tab" aria-selected={backupFilter === 'sessions'} className={backupFilter === 'sessions' ? 'active' : ''} onClick={() => setBackupFilter('sessions')}>
                  {tr('会话备份', 'Session backups')} <span>{sessionBackups.length}</span>
                </button>
                <button type="button" role="tab" aria-selected={backupFilter === 'index'} className={backupFilter === 'index' ? 'active' : ''} onClick={() => setBackupFilter('index')}>
                  {tr('索引快照', 'Index snapshots')} <span>{indexBackups.length}</span>
                </button>
              </div>
              <label className="search-box backups-search">
                <Search size={14} />
                <input
                  value={backupQuery}
                  onChange={(e) => setBackupQuery(e.target.value)}
                  placeholder={tr('搜索备份标题、ID 或 Agent…', 'Search title, ID, or agent…')}
                />
              </label>
            </div>

            {!backups.length ? (
              <div className="empty-state">
                <Archive size={28} />
                <h3>{tr('还没有备份', 'No backups yet')}</h3>
                <p>{tr('在迁移工作台选择会话，点击“创建备份”。', 'Select sessions in the workbench, then choose Create backup.')}</p>
              </div>
            ) : (
              <div className="backups-workspace">
                <div className="backups-list-pane">
                  {!filteredBackups.length ? (
                    <div className="backups-filter-empty">
                      <p>{tr('没有匹配的备份记录。', 'No matching backup records.')}</p>
                    </div>
                  ) : filteredBackups.map((record) => {
                    const isSelected = activeBackup?.id === record.id;
                    const providerInfo = PROVIDERS.find((p) => p.id === record.provider);
                    return (
                      <article
                        key={record.id}
                        className={isSelected ? 'backup-card active' : 'backup-card'}
                        onClick={() => { setSelectedBackupId(record.id); if (backupImport?.id !== record.id) setBackupImport(undefined); }}
                      >
                        <div className="backup-card-top">
                          <span className="backup-card-provider">
                            {providerInfo && <AgentIcon provider={providerInfo.id} />}
                            {providerInfo?.name ?? record.provider}
                          </span>
                          <span className={record.kind === 'target-index' ? 'backup-kind-tag muted' : 'backup-kind-tag'}>
                            {record.kind === 'target-index' ? tr('索引快照', 'Index snapshot') : tr('会话备份', 'Sessions')}
                          </span>
                        </div>
                        <h3>{backupTitle(record)}</h3>
                        <p className="backup-card-meta">
                          {date(record.createdAt)} · {record.kind === 'target-index' ? `${record.files.length} ${tr('个文件', 'files')}` : `${record.sessions.length} ${tr('个会话', 'sessions')}`} · {formatKb(record)} KB
                        </p>
                      </article>
                    );
                  })}
                </div>

                <div className="backups-detail-pane">
                  {backupImport ? (
                    <BackupImportPanel
                      key={backupImport.id}
                      backup={backupImport}
                      onClose={() => setBackupImport(undefined)}
                      onBusy={(value) => { actionLock.current = value; setBusy(value); }}
                      onImported={() => { void refreshBackups(); }}
                    />
                  ) : activeBackup ? (
                    <div className="backup-dossier">
                      <header className="backup-dossier-header">
                        <div className="backup-dossier-title">
                          {activeBackupProvider && <AgentIcon provider={activeBackupProvider.id} />}
                          <div>
                            <strong>{backupTitle(activeBackup)}</strong>
                            <span>
                              {date(activeBackup.createdAt)} · {activeBackupProvider?.name ?? activeBackup.provider}
                            </span>
                          </div>
                        </div>
                        {activeBackup.kind !== 'target-index' && (
                          <button
                            className="signal-button backup-import-trigger"
                            disabled={busy}
                            onClick={() => setBackupImport(activeBackup)}
                          >
                            {tr('导入 Agent', 'Import into agent')}
                          </button>
                        )}
                      </header>

                      <dl className="backup-dossier-facts">
                        <div>
                          <dt>{tr('备份 ID', 'Backup ID')}</dt>
                          <dd>{activeBackup.id}</dd>
                        </div>
                        <div>
                          <dt>{tr('体积与文件', 'Size & files')}</dt>
                          <dd>{formatKb(activeBackup)} KB · {activeBackup.files.length} {tr('个文件', 'files')}</dd>
                        </div>
                        <div className="full-span">
                          <dt>{tr('本地存储目录', 'Backup folder')}</dt>
                          <dd className="output-path">{activeBackup.directory}</dd>
                        </div>
                      </dl>

                      {activeBackup.kind !== 'target-index' && activeBackup.sessions.length > 0 && (
                        <section className="backup-dossier-section">
                          <header className="dossier-section-head">
                            <span>{tr('包含会话', 'Included sessions')}</span>
                            <small>{activeBackup.sessions.length}</small>
                          </header>
                          <ul className="backup-session-titles">
                            {activeBackup.sessions.map((session) => (
                              <li key={session.id}>
                                <b>{session.title || tr('未命名会话', 'Untitled session')}</b>
                                <small>{session.workspace || session.id} · {session.messageCount} {t('messages')}</small>
                              </li>
                            ))}
                          </ul>
                        </section>
                      )}

                      <details className="backup-dossier-files">
                        <summary>{tr('查看文件与校验值', 'Files and checksums')} ({activeBackup.files.length})</summary>
                        <div className="backup-files-list">
                          {activeBackup.files.map((file) => (
                            <div key={file.stored} className="file-integrity">
                              <div className="file-integrity-row">
                                <b>{file.stored}</b>
                                <span>{(file.bytes / 1024).toFixed(1)} KB</span>
                              </div>
                              <small>{file.original}</small>
                              <code>SHA-256: {file.sha256}</code>
                            </div>
                          ))}
                        </div>
                      </details>
                    </div>
                  ) : null}
                </div>
              </div>
            )}
          </section>
        )}

        {page === 'settings' && (
          <section className="utility-page settings-page" aria-label={t('nav.settings')}>
            <form onSubmit={save}>
              <h2>{tr('本地存储', 'Local storage')}</h2>
              <label className="settings-field">
                {tr('备份目录', 'Backup directory')}
                <input required disabled={!prefsLoaded || busy} value={prefs.backupDirectory} onChange={(e) => setPrefs({ ...prefs, backupDirectory: e.target.value })} />
                <small>{tr('使用绝对路径。修改目录不会移动之前的备份。', 'Use an absolute path. Changing this does not move existing backups.')}</small>
              </label>
              <label className="settings-field">
                {tr('Markdown 导出目录', 'Markdown export directory')}
                <input required disabled={!prefsLoaded || busy} value={prefs.exportDirectory} onChange={(e) => setPrefs({ ...prefs, exportDirectory: e.target.value })} />
              </label>
              <button className="signal-button" type="submit" disabled={!prefsLoaded || busy}>{tr('保存设置', 'Save settings')}</button>
            </form>
            <section className="appearance-settings">
              <h2>{tr('外观与语言', 'Appearance and language')}</h2>
              <label>
                {t('theme')}
                <select value={dark ? 'dark' : 'light'} onChange={(e) => setDark(e.target.value === 'dark')}>
                  <option value="light">{t('light')}</option>
                  <option value="dark">{t('dark')}</option>
                </select>
              </label>
              <label>
                {t('language')}
                <select value={locale} onChange={(e) => setLocale(e.target.value as typeof locale)}>
                  <option value="zh-CN">简体中文</option>
                  <option value="en-US">English</option>
                </select>
              </label>
            </section>
            <section className="about-settings">
              <h2>CyanBird {APP_VERSION}</h2>
              <button className="outline-button" onClick={checkUpdates} disabled={update.status === 'checking'}>
                <RefreshCw size={14} />
                {updateLabel}
              </button>
              <a href="https://github.com/MusaLunar/CyanBird" target="_blank" rel="noreferrer">
                GitHub <ExternalLink size={14} />
              </a>
            </section>
          </section>
        )}

        {preview && (
          <aside className="session-preview" aria-label={tr('会话预览', 'Session preview')}>
            <header>
              <h2>{preview.title || preview.id}</h2>
              <button className="square-button" aria-label={tr('关闭预览', 'Close preview')} onClick={() => { ++previewRequest.current; setPreview(undefined); }}>
                <X size={16} />
              </button>
            </header>
            <p>{preview.workspace}</p>
            <pre>{previewText || tr('正在读取会话…', 'Reading session…')}</pre>
          </aside>
        )}

        <footer className="stage-footer">
          <span><ShieldCheck size={14} />{t('safe')}</span>
          <span className="footer-spacer" />
          <span className="footer-telemetry">{source.id.toUpperCase()} → {target.id.toUpperCase()} · v{APP_VERSION}</span>
        </footer>
      </main>
    </div>
  );
}
