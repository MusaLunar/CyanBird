use crate::discovery::{home, read_payload, scan_sessions, text_content, SessionSummary};
use crate::metadata::clean_prompt;
use rusqlite::{Connection, DatabaseName, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fs, io::Read, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Preferences { pub backup_directory:String, pub export_directory:String }
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct StoredFile { pub original:String, pub stored:String, pub bytes:u64, pub sha256:String }
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct BackupRecord { pub id:String, pub created_at:String, pub provider:String, pub sessions:Vec<SessionSummary>, pub files:Vec<StoredFile>, pub directory:String }
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct ExportReceipt { pub directory:String, pub files:Vec<String> }
fn storage_root() -> PathBuf { std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(||home().join(".local/share")).join("CyanBird") }
fn defaults() -> Preferences { Preferences {backup_directory:storage_root().join("backups").to_string_lossy().into(),export_directory:home().join("Documents/CyanBird/exports").to_string_lossy().into()} }
pub fn preferences() -> Result<Preferences,String> {
 let path=storage_root().join("settings.json");
 if !path.exists(){return Ok(defaults())}
 serde_json::from_slice(&fs::read(path).map_err(|e|e.to_string())?).map_err(|e|format!("Settings: {e}"))
}
fn validate_preferences(value:&Preferences) -> Result<(),String> {
 for path in [&value.backup_directory,&value.export_directory] { if !Path::new(path).is_absolute(){return Err("Choose an absolute output directory".into())} }
 Ok(())
}
fn fresh_directory(root:&Path,prefix:&str) -> Result<PathBuf,String> {
 fs::create_dir_all(root).map_err(|e|e.to_string())?;
 let stamp=SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos();
 let path=root.join(format!("{prefix}-{stamp}"));fs::create_dir(&path).map_err(|e|e.to_string())?;Ok(path)
}
fn chosen(provider:String,ids:Vec<String>) -> Result<Vec<SessionSummary>,String> {
 if ids.is_empty(){return Err("Select at least one session".into())}
 let unique:HashSet<_>=ids.iter().collect();
 let result:Vec<_>=scan_sessions(provider).into_iter().filter(|s|unique.contains(&s.id)).collect();
 if result.len()!=unique.len(){return Err("Some selected sessions no longer exist; scan again".into())} Ok(result)
}
fn hash_file(path:&Path)->Result<String,String>{
 let mut f=fs::File::open(path).map_err(|e|e.to_string())?;let mut hasher=Sha256::new();let mut buffer=[0u8;65536];
 loop {let n=f.read(&mut buffer).map_err(|e|e.to_string())?;if n==0{break}hasher.update(&buffer[..n]);}Ok(format!("{:x}",hasher.finalize()))
}
fn snapshot_file(source:&Path,target:&Path)->Result<StoredFile,String>{
 if source.extension().is_some_and(|e|e=="sqlite"||e=="db") {
  let conn=Connection::open_with_flags(source,OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e|e.to_string())?;
  conn.backup(DatabaseName::Main,target,None).map_err(|e|e.to_string())?;
  let check=Connection::open_with_flags(target,OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e|e.to_string())?;
  let health:String=check.query_row("PRAGMA quick_check",[],|r|r.get(0)).map_err(|e|e.to_string())?;
  if health!="ok"{return Err(format!("SQLite snapshot check: {health}"))}
 } else {
  fs::copy(source,target).map_err(|e|e.to_string())?;
  if hash_file(source)?!=hash_file(target)?{return Err("Source changed while backing up; retry after the agent stops writing".into())}
 }
 Ok(StoredFile{original:source.to_string_lossy().into(),stored:target.file_name().unwrap().to_string_lossy().into(),bytes:fs::metadata(target).map_err(|e|e.to_string())?.len(),sha256:hash_file(target)?})
}
pub fn backup_to(root:&Path,provider:&str,sessions:Vec<SessionSummary>)->Result<BackupRecord,String>{
 let stage=fresh_directory(root,".partial")?; let mut files=Vec::new();let mut seen=HashSet::new();
 for session in &sessions {
  let source=PathBuf::from(&session.source_path);
  if seen.insert(source.clone()) {
   let name=format!("{:04}-{}",files.len(),source.file_name().ok_or("Invalid source filename")?.to_string_lossy());
   files.push(snapshot_file(&source,&stage.join(name))?);
  }
  if provider=="antigravity" {
   if let Some(root)=source.ancestors().nth(5) {
    let annotation=root.join("annotations").join(format!("{}.pbtxt",session.id));
    if annotation.exists() && seen.insert(annotation.clone()) {files.push(snapshot_file(&annotation,&stage.join(format!("{:04}-annotation.pbtxt",files.len())))?);}
   }
  }
 }
 let id=stage.file_name().unwrap().to_string_lossy().replace(".partial-","backup-");let directory=root.join(&id);
 let record=BackupRecord{id,created_at:SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis().to_string(),provider:provider.into(),sessions,files,directory:directory.to_string_lossy().into()};
 fs::write(stage.join("manifest.json"),serde_json::to_vec_pretty(&record).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
 fs::rename(&stage,&directory).map_err(|e|e.to_string())?;Ok(record)
}
fn fence(content:&str,language:&str)->String {
 let longest=content.split(|c|c!='`').map(str::len).max().unwrap_or(0);let marker="`".repeat(3.max(longest+1));format!("{marker}{language}\n{content}\n{marker}\n")
}
fn body(value:&Value)->String {
 if let Some(parts)=value.as_array(){parts.iter().filter_map(|part|{
  if let Some(text)=part["text"].as_str(){Some(text.to_string())}
  else if part["type"]=="thinking"{Some(format!("### Reasoning\n\n{}",part["thinking"].as_str().unwrap_or_default()))}
  else {Some(fence(&serde_json::to_string_pretty(part).unwrap_or_default(),"json"))}
 }).collect::<Vec<_>>().join("\n\n")}else{text_content(value)}
}
fn markdown_records(session:&SessionSummary,records:&[Value])->String{
 let mut out=format!("# {}\n\nAgent: {}\n\nSession: {}\n\nWorkspace: {}\n\n",session.title.replace('\n'," "),session.source,session.id,session.workspace.as_deref().unwrap_or("Not recorded"));
 for r in records {
  let t=r["type"].as_str().unwrap_or_default();
  let (role,content)=match session.source.as_str(){
   "claude-code" if matches!(t,"user"|"assistant")=>(t,body(&r["message"]["content"])),
   "codex" if t=="response_item" => {
    let p=&r["payload"];
    if p["type"]=="message" {(p["role"].as_str().unwrap_or("message"),body(&p["content"]))}
    else {("event",fence(&serde_json::to_string_pretty(p).unwrap_or_default(),"json"))}
   },
   "antigravity" if matches!(t,"USER_INPUT"|"PLANNER_RESPONSE"|"GENERIC"|"ERROR_MESSAGE") => {
    let mut content=if t=="USER_INPUT" {clean_prompt(&text_content(&r["content"]))}else{body(&r["content"])};
    if let Some(thinking)=r["thinking"].as_str(){content.push_str(&format!("\n\n### Reasoning\n\n{thinking}"));}
    if let Some(calls)=r["tool_calls"].as_array(){for call in calls{content.push_str(&format!("\n\n### Tool call\n\n{}",fence(&serde_json::to_string_pretty(call).unwrap_or_default(),"json")));}}
    (t,content)
   },
   "dsh" if t.ends_with("/message") =>(t,body(&r["data"]["content"])),
   "dsh" if t.contains("tool") || t.contains("reasoning") =>(t,fence(&serde_json::to_string_pretty(&r["data"]).unwrap_or_default(),"json")),
   "zcode" =>(r["role"].as_str().unwrap_or("message"),body(&r["content"])),
   _=>continue,
  };
  if !content.trim().is_empty(){out.push_str(&format!("## {role}\n\n{content}\n\n"));}
 }
 out
}
fn read_export_records(session:&SessionSummary)->Result<Vec<Value>,String>{
 if session.source=="zcode" {
  let db=Connection::open_with_flags(&session.source_path,OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e|e.to_string())?;
  let mut messages=db.prepare("SELECT id,data FROM message WHERE session_id=?1 ORDER BY time_created,id").map_err(|e|e.to_string())?;
  let rows=messages.query_map([&session.id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(|e|e.to_string())?;
  let mut out=Vec::new();
  for row in rows {let (id,data)=row.map_err(|e|e.to_string())?;let mut value:Value=serde_json::from_str(&data).map_err(|e|e.to_string())?;
   let mut parts=db.prepare("SELECT data FROM part WHERE message_id=?1 ORDER BY time_created,id").map_err(|e|e.to_string())?;
   let content=parts.query_map([id],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?.map(|s|s.map_err(|e|e.to_string()).and_then(|s|serde_json::from_str::<Value>(&s).map_err(|e|e.to_string()))).collect::<Result<Vec<_>,_>>()?;
   value["content"]=Value::Array(content);out.push(value);
  }return Ok(out)
 }
 let raw=read_payload(Path::new(&session.source_path)).ok_or("Cannot read or decode transcript")?;
 raw.lines().filter(|line|!line.trim().is_empty()).enumerate().map(|(i,line)|serde_json::from_str(line).map_err(|e|format!("Malformed transcript at line {}: {e}",i+1))).collect()
}
fn export_to(root:&Path,sessions:&[SessionSummary])->Result<ExportReceipt,String>{
 let stage=fresh_directory(root,".partial")?;let mut names=Vec::new();
 for (i,session) in sessions.iter().enumerate(){
  let records=read_export_records(session)?;
  let name=format!("session-{:04}.md",i+1);fs::write(stage.join(&name),markdown_records(session,&records)).map_err(|e|e.to_string())?;names.push(name);
 }
 let directory=root.join(stage.file_name().unwrap().to_string_lossy().replace(".partial-","markdown-"));
 fs::rename(stage,&directory).map_err(|e|e.to_string())?;
 Ok(ExportReceipt{files:names.iter().map(|n|directory.join(n).to_string_lossy().into()).collect(),directory:directory.to_string_lossy().into()})
}
#[tauri::command] pub fn get_preferences()->Result<Preferences,String>{preferences()}
#[tauri::command] pub fn save_preferences(value:Preferences)->Result<Preferences,String>{
 validate_preferences(&value)?;fs::create_dir_all(storage_root()).map_err(|e|e.to_string())?;
 fs::write(storage_root().join("settings.json"),serde_json::to_vec_pretty(&value).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;Ok(value)
}
#[tauri::command] pub async fn create_backup(provider:String,ids:Vec<String>)->Result<BackupRecord,String>{
 tauri::async_runtime::spawn_blocking(move||{let prefs=preferences()?;let sessions=chosen(provider.clone(),ids)?;backup_to(Path::new(&prefs.backup_directory),&provider,sessions)}).await.map_err(|e|e.to_string())?
}
#[tauri::command] pub async fn export_markdown(provider:String,ids:Vec<String>)->Result<ExportReceipt,String>{
 tauri::async_runtime::spawn_blocking(move||{let prefs=preferences()?;let sessions=chosen(provider,ids)?;export_to(Path::new(&prefs.export_directory),&sessions)}).await.map_err(|e|e.to_string())?
}
#[tauri::command] pub async fn list_backups()->Result<Vec<BackupRecord>,String>{
 tauri::async_runtime::spawn_blocking(||{let root=PathBuf::from(preferences()?.backup_directory);if !root.exists(){return Ok(Vec::new())}let mut records=Vec::new();
  for item in fs::read_dir(root).map_err(|e|e.to_string())?{let p=item.map_err(|e|e.to_string())?.path();if p.file_name().is_some_and(|n|n.to_string_lossy().starts_with("backup-")){let raw=fs::read(p.join("manifest.json")).map_err(|e|e.to_string())?;let record:BackupRecord=serde_json::from_slice(&raw).map_err(|e|e.to_string())?;records.push(record);}}
  records.sort_by(|a,b|b.created_at.cmp(&a.created_at));Ok(records)
 }).await.map_err(|e|e.to_string())?
}
#[tauri::command] pub fn open_output_directory(kind:String)->Result<(),String>{
 let prefs=preferences()?;let path=match kind.as_str(){"backup"=>prefs.backup_directory,"export"=>prefs.export_directory,_=>return Err("Unknown directory".into())};
 fs::create_dir_all(&path).map_err(|e|e.to_string())?;
 std::process::Command::new("explorer.exe").arg(path).spawn().map_err(|e|e.to_string())?;Ok(())
}
#[cfg(test)] mod tests {
 use super::*;
 fn session(path:&Path)->SessionSummary{SessionSummary{id:"fixture".into(),title:"Fixture session".into(),source:"claude-code".into(),workspace:Some("C:/Project".into()),updated_at:"0".into(),message_count:1,tool_call_count:0,status:"ready".into(),source_path:path.to_string_lossy().into()}}
 #[test] fn backup_manifest_matches_original_bytes_and_preserves_source(){
  let root=fresh_directory(&std::env::temp_dir(),"cyanbird-backup-test").unwrap();let source=root.join("session.jsonl");fs::write(&source,b"original transcript\n").unwrap();
  let backup=backup_to(&root.join("backups"),"claude-code",vec![session(&source)]).unwrap();
  assert_eq!(backup.files[0].sha256,hash_file(&source).unwrap());assert_eq!(fs::read_to_string(&source).unwrap(),"original transcript\n");assert!(Path::new(&backup.directory).join("manifest.json").exists());fs::remove_dir_all(root).unwrap();
 }
 #[test] fn markdown_keeps_tools_and_nested_code_fences(){
  let session=session(Path::new("fixture.jsonl"));let records=vec![serde_json::json!({"type":"assistant","message":{"content":[{"type":"text","text":"answer"},{"type":"tool_use","name":"Read","input":{"code":"```"}}]}})];let md=markdown_records(&session,&records);
  assert!(md.contains("answer"));assert!(md.contains("Read"));assert!(md.contains("````json"));
 }
 #[test] fn sqlite_backup_contains_uncheckpointed_wal(){
  let root=fresh_directory(&std::env::temp_dir(),"cyanbird-sqlite-test").unwrap();let source=root.join("db.sqlite");let db=Connection::open(&source).unwrap();db.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE session(id TEXT); INSERT INTO session VALUES('wal-session');").unwrap();
  let target=root.join("copy.sqlite");snapshot_file(&source,&target).unwrap();let backup=Connection::open(&target).unwrap();let id:String=backup.query_row("SELECT id FROM session",[],|r|r.get(0)).unwrap();assert_eq!(id,"wal-session");drop(backup);drop(db);fs::remove_dir_all(root).unwrap();
 }
}
