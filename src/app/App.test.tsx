import { fireEvent, render, screen, waitFor, cleanup } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { App } from './App';
vi.mock('@tauri-apps/api/core',()=>({invoke:vi.fn()}));
afterEach(cleanup);
beforeEach(()=>{localStorage.clear();vi.mocked(invoke).mockImplementation(async(command:string)=>{
 if(command==='discover_sources') return [];
 if(command==='discover_sessions') return ['one','two'].map(id=>({id,title:id,source:'claude-code',workspace:'C:/Project',updatedAt:'1794000000000',messageCount:2,toolCallCount:1,status:'ready'}));
 if(command==='get_preferences'||command==='save_preferences')return {backupDirectory:'C:/Backups',exportDirectory:'C:/Exports'};
 if(command==='list_backups')return [{id:'backup-fixture',createdAt:'1794000000000',provider:'claude-code',kind:'sessions',directory:'C:/Backups/backup-fixture',sessions:[{id:'saved',title:'已删除源文件的会话',source:'claude-code',workspace:'C:/Project'}],files:[]}];
 if(command==='preview_backup_import')return {targetDirectory:'D:/Restored/sessions',messageCount:2};
 if(command==='import_backup')return {items:[{sourceId:'saved',targetId:'restored',path:'D:/Restored/new.jsonl',status:'written'}]};
 if(command==='preview_migration')return {targetDirectory:'D:/Target/sessions',messageCount:2};
 if(command==='execute_migration')return {backupDirectory:'C:/Backups/before',items:[{targetId:'new',path:'D:/Target/sessions/new.jsonl',status:'written'}]};
 if(command==='create_backup')return {directory:'C:/Backups/snapshot'};
 if(command==='export_markdown')return {directory:'C:/Exports/session',files:['session.md']};
 return null;
});});
it('selects one session without selecting the other and exports real IDs',async()=>{
 render(<App/>);const rows=await screen.findAllByRole('checkbox');fireEvent.click(rows[0]);expect(rows[0]).toHaveAttribute('aria-checked','true');expect(rows[1]).toHaveAttribute('aria-checked','false');expect(screen.queryByText('metadata.unknownField')).toBeNull();
 fireEvent.click(screen.getByRole('button',{name:'导出 Markdown'}));await waitFor(()=>expect(invoke).toHaveBeenCalledWith('export_markdown',{provider:'claude-code',ids:['one']}));await screen.findByText('Markdown 已导出。');
});
it('opens backup and settings routes and saves paths',async()=>{
 render(<App/>);await screen.findAllByRole('checkbox');expect(screen.queryByText(/\bark\b|\bcomplex\b/i)).toBeNull();fireEvent.click(screen.getByRole('button',{name:'备份中心'}));await screen.findByText('备份记录');expect(screen.getByRole('tab',{name:/全部/})).toBeInTheDocument();fireEvent.click(screen.getByRole('button',{name:'设置'}));fireEvent.change(screen.getByLabelText('Markdown 导出目录'),{target:{value:'D:/Exports'}});fireEvent.click(screen.getByRole('button',{name:'保存设置'}));await waitFor(()=>expect(invoke).toHaveBeenCalledWith('save_preferences',{value:{backupDirectory:'C:/Backups',exportDirectory:'D:/Exports'}}));
});
it('creates a backup for only the selected session',async()=>{
 render(<App/>);fireEvent.click((await screen.findAllByRole('checkbox'))[1]);fireEvent.click(screen.getByRole('button',{name:'创建备份'}));await waitFor(()=>expect(invoke).toHaveBeenCalledWith('create_backup',{provider:'claude-code',ids:['two']}));await screen.findByText('备份已创建并校验。');
});
it('orders configuration before actions and keeps row previews separate',async()=>{
 render(<App/>);fireEvent.click((await screen.findAllByRole('checkbox'))[0]);
 expect(screen.getByText('已选 1 项')).toBeInTheDocument();expect(screen.getByRole('button',{name:'全选'})).toBeInTheDocument();
 expect(screen.queryByText('Zcode 备份包含整个会话数据库。')).toBeNull();
 const output=screen.getByText('输出位置'), backup=screen.getByRole('button',{name:'创建备份'});
 expect(output.compareDocumentPosition(backup)&Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
 fireEvent.click(screen.getByRole('button',{name:'预览 two'}));await waitFor(()=>expect(invoke).toHaveBeenCalledWith('preview_session',{provider:'claude-code',id:'two'}));
 expect(screen.getAllByRole('checkbox')[1]).toHaveAttribute('aria-checked','false');
});

it('requires a current preview and acknowledgment before native migration',async()=>{
 render(<App/>);fireEvent.click((await screen.findAllByRole('checkbox'))[0]);
 const execute=screen.getByRole('button',{name:'执行迁移'});expect(execute).toBeDisabled();
 fireEvent.click(screen.getByRole('button',{name:'生成迁移预览'}));
 const acknowledgment=await screen.findByLabelText(/确认转换差异/);fireEvent.click(acknowledgment);expect(execute).toBeEnabled();
 fireEvent.change(screen.getByLabelText('目标项目文件夹'),{target:{value:'D:/Different'}});expect(execute).toBeDisabled();
 fireEvent.click(screen.getByRole('button',{name:'生成迁移预览'}));fireEvent.click(await screen.findByLabelText(/确认转换差异/));fireEvent.click(execute);
 await waitFor(()=>expect(invoke).toHaveBeenCalledWith('execute_migration',{provider:'claude-code',ids:['one'],target:'dsh',workspace:'D:/Different',acknowledged:true}));
 await screen.findByText('已写入');
});

it('allows Antigravity as a migration destination',async()=>{
 render(<App/>);fireEvent.click((await screen.findAllByRole('checkbox'))[0]);
 const destination=screen.getByRole('button',{name:'Antigravity'});
 expect(destination).toBeEnabled();fireEvent.click(destination);
 fireEvent.click(screen.getByRole('button',{name:'生成迁移预览'}));
 await waitFor(()=>expect(invoke).toHaveBeenCalledWith('preview_migration',{provider:'claude-code',ids:['one'],target:'antigravity',workspace:'C:/Project'}));
 expect(await screen.findByLabelText(/仅迁入用户消息和助手正文/)).toBeInTheDocument();
 fireEvent.click(await screen.findByLabelText(/确认转换差异/));fireEvent.click(screen.getByRole('button',{name:'执行迁移'}));
 await waitFor(()=>expect(invoke).toHaveBeenCalledWith('execute_migration',{provider:'claude-code',ids:['one'],target:'antigravity',workspace:'C:/Project',acknowledged:true}));
});

it('shows backed-up titles and imports snapshot IDs with a current preview',async()=>{
 render(<App/>);await screen.findAllByRole('checkbox');fireEvent.click(screen.getByRole('button',{name:'备份中心'}));
 await screen.findByRole('heading',{name:'已删除源文件的会话'});fireEvent.click(screen.getByRole('button',{name:'导入 Agent'}));
 fireEvent.change(screen.getByLabelText('导入目标 Agent'),{target:{value:'antigravity'}});
 fireEvent.change(screen.getByLabelText('导入项目文件夹'),{target:{value:'D:/Restored'}});
 const execute=screen.getByRole('button',{name:'确认导入'});expect(execute).toBeDisabled();
 fireEvent.click(screen.getByRole('button',{name:'预览导入'}));
 await waitFor(()=>expect(invoke).toHaveBeenCalledWith('preview_backup_import',{backupId:'backup-fixture',ids:['saved'],target:'antigravity',workspace:'D:/Restored'}));
 fireEvent.click(await screen.findByLabelText(/确认导入为新会话/));expect(execute).toBeEnabled();
 fireEvent.change(screen.getByLabelText('导入项目文件夹'),{target:{value:'D:/Other'}});expect(execute).toBeDisabled();
 fireEvent.click(screen.getByRole('button',{name:'预览导入'}));fireEvent.click(await screen.findByLabelText(/确认导入为新会话/));fireEvent.click(execute);
 await waitFor(()=>expect(invoke).toHaveBeenCalledWith('import_backup',{backupId:'backup-fixture',ids:['saved'],target:'antigravity',workspace:'D:/Other',acknowledged:true}));
 await screen.findByText(/已删除源文件的会话 · 已导入/);
});
