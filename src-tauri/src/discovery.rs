use crate::metadata::{antigravity_id, annotation_title, clean_prompt, dominant_cwd, is_codex_child, is_injected, workspace_uri};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fs, io::Read, path::{Path, PathBuf}, time::UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceLocation { pub id: String, pub provider: String, pub label: String, pub path: String, pub session_count: usize, pub status: String, pub last_scanned: Option<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary { pub id: String, pub title: String, pub source: String, pub workspace: Option<String>, pub updated_at: String, pub message_count: usize, pub tool_call_count: usize, pub status: String, pub source_path: String }

pub(crate) fn home() -> PathBuf { std::env::var_os("USERPROFILE").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(PathBuf::from)).unwrap_or_else(|| PathBuf::from(".")) }
fn now_string() -> String { std::time::SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis().to_string() }
fn path_string(path: &Path) -> String { path.to_string_lossy().replace('\\', "/") }
fn mtime(path: &Path) -> String { fs::metadata(path).ok().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_millis().to_string()).unwrap_or_default() }
fn walk(root: &Path, depth: usize, out: &mut Vec<PathBuf>) { if depth > 12 { return; } let Ok(entries)=fs::read_dir(root) else { return; }; for e in entries.flatten() { let p=e.path(); if e.file_type().map(|t|t.is_symlink()).unwrap_or(true) { continue; } if p.is_dir() { let n=p.file_name().and_then(|x|x.to_str()).unwrap_or_default(); if matches!(n,"node_modules"|".git"|"target"|"dist"|"build"|".cache") { continue; } walk(&p,depth+1,out); } else { out.push(p); } } }
pub(crate) fn read_payload(path: &Path) -> Option<String> { let mut bytes=Vec::new(); fs::File::open(path).ok()?.read_to_end(&mut bytes).ok()?; if path.extension().and_then(|x|x.to_str()).unwrap_or_default().eq_ignore_ascii_case("zst") || path.extension().and_then(|x|x.to_str()).unwrap_or_default().eq_ignore_ascii_case("zstd") { zstd::stream::decode_all(bytes.as_slice()).ok().and_then(|b| String::from_utf8(b).ok()) } else { String::from_utf8(bytes).ok() } }
pub(crate) fn records(path: &Path) -> Vec<Value> { let Some(text)=read_payload(path) else { return vec![] }; text.lines().filter_map(|line| serde_json::from_str::<Value>(line).ok()).collect() }
pub(crate) fn text_content(v: &Value) -> String { match v { Value::String(s)=>s.clone(), Value::Array(items)=>items.iter().filter_map(|item| item.get("text").and_then(Value::as_str)).collect::<Vec<_>>().join("\n"), _=>String::new() } }
fn first_user(records: &[Value], kind: &str) -> String { for r in records { let is_user = if kind=="claude" { r.get("type").and_then(Value::as_str)==Some("user") } else if kind=="codex" { r.get("type").and_then(Value::as_str)==Some("response_item") && r.get("payload").and_then(|p|p.get("role")).and_then(Value::as_str)==Some("user") } else if kind=="dsh" { r.get("type").and_then(Value::as_str)==Some("user/message") } else { false }; if !is_user { continue; } let candidate = if kind=="claude" { r.get("message").and_then(|m|m.get("content")).map(text_content).unwrap_or_default() } else if kind=="codex" { r.get("payload").and_then(|p|p.get("content")).map(text_content).unwrap_or_default() } else { r.get("data").and_then(|d|d.get("content")).map(text_content).unwrap_or_default() }; let value=candidate.trim(); if !value.is_empty() && !is_injected(value) { return value.chars().take(80).collect(); } } String::new() }
fn count_tool_calls(records: &[Value]) -> usize {
 records.iter().map(|r| {
  let t=r["type"].as_str().unwrap_or_default();
  let payload=r["payload"]["type"].as_str().unwrap_or_default();
  let blocks=r["message"]["content"].as_array().map(|b| b.iter().filter(|x|x["type"]=="tool_use").count()).unwrap_or(0);
  blocks + r["tool_calls"].as_array().map(Vec::len).unwrap_or(0) + usize::from(t=="tool/call" || t=="tool/start" || t=="response_item" && matches!(payload,"function_call"|"custom_tool_call"))
 }).sum()
}
fn count_messages(records: &[Value]) -> usize {
 records.iter().filter(|r| {
  let t=r["type"].as_str().unwrap_or_default();
  matches!(t,"user"|"assistant"|"USER_INPUT"|"MODEL_RESPONSE"|"PLANNER_RESPONSE"|"user/message"|"assistant/message") || t=="response_item" && r["payload"]["type"]=="message"
 }).count()
}
fn summary_from_json(path: &Path, provider: &str, id: String, title: String, workspace: Option<String>, records: Vec<Value>) -> SessionSummary { SessionSummary { id, title, source: provider.to_string(), workspace, updated_at: mtime(path), message_count: count_messages(&records), tool_call_count: count_tool_calls(&records), status: if records.is_empty(){"warning"}else{"ready"}.to_string(), source_path: path_string(path) } }
fn claude_sessions() -> Vec<SessionSummary> { let root=home().join(".claude/projects"); let mut files=Vec::new(); walk(&root,0,&mut files); files.into_iter().filter(|p|p.extension().and_then(|x|x.to_str())==Some("jsonl")).filter_map(|p| { let rs=records(&p); if rs.is_empty(){return None;} let id=rs.iter().rev().find_map(|r|r.get("sessionId").and_then(Value::as_str)).or_else(||p.file_stem().and_then(|x|x.to_str()))?.to_string(); if p.file_stem().and_then(|x|x.to_str()) != Some(id.as_str()) { return None; } let title=rs.iter().rev().find_map(|r|r.get("customTitle").and_then(Value::as_str)).map(str::to_string).unwrap_or_else(||first_user(&rs,"claude")); let cwd=rs.iter().find_map(|r|r.get("cwd").and_then(Value::as_str)).map(str::to_string); Some(summary_from_json(&p,"claude-code",id,title,cwd,rs)) }).collect() }
fn codex_sessions() -> Vec<SessionSummary> { let mut files=Vec::new(); for root in [home().join(".codex/sessions"),home().join(".codex/archived_sessions")] { walk(&root,0,&mut files); } files.into_iter().filter(|p|p.file_name().and_then(|x|x.to_str()).unwrap_or_default().starts_with("rollout-")).filter_map(|p| { let rs=records(&p); if rs.is_empty(){return None;} let meta=rs.iter().find(|r|r["type"]=="session_meta")?; if is_codex_child(&meta["payload"]) { return None; } let id=meta["payload"]["id"].as_str()?.to_string(); let mut title=first_user(&rs,"codex"); if let Ok(index)=fs::read_to_string(home().join(".codex/session_index.jsonl")) { for row in index.lines().filter_map(|line|serde_json::from_str::<Value>(line).ok()) { if row["id"].as_str()==Some(id.as_str()) { if let Some(name)=row["thread_name"].as_str(){title=name.to_string();} } } } let cwd=meta["payload"]["cwd"].as_str().map(str::to_string); Some(summary_from_json(&p,"codex",id,title,cwd,rs)) }).collect() }
fn antigravity_sessions() -> Vec<SessionSummary> {
 let mut out=Vec::new();
 for name in ["antigravity","antigravity-cli","antigravity-ide"] {
  let root=home().join(".gemini").join(name);
  let mut summaries=std::collections::HashMap::<String,(String,Option<String>,bool)>::new();
  if let Ok(db)=rusqlite::Connection::open_with_flags(root.join("conversation_summaries.db"),rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
   if let Ok(mut statement)=db.prepare("SELECT conversation_id,title,workspace_uris,parent_conversation_id FROM conversation_summaries") {
    if let Ok(rows)=statement.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?))) {
     for (id,title,uris,parent) in rows.flatten() {
      let workspace=serde_json::from_str::<Vec<String>>(&uris).ok().and_then(|u|u.first().map(|p|workspace_uri(p)));
      summaries.insert(id,(title,workspace,!parent.trim().is_empty()));
     }
    }
   }
  }
  let mut files=Vec::new(); walk(&root.join("brain"),0,&mut files);
  for path in files.into_iter().filter(|p|p.file_name().is_some_and(|n|n=="transcript.jsonl")) {
   let Some(id)=antigravity_id(&path) else {continue};
   let summary=summaries.get(&id);
   if summary.is_some_and(|x|x.2) {continue;}
   let rs=records(&path); if rs.is_empty(){continue;}
   let title=summary.map(|x|x.0.clone()).filter(|s|!s.trim().is_empty())
    .or_else(||fs::read_to_string(root.join("annotations").join(format!("{id}.pbtxt"))).ok().and_then(|s|annotation_title(&s)))
    .unwrap_or_else(||rs.iter().find(|r|r["type"]=="USER_INPUT").map(|r|clean_prompt(&text_content(&r["content"]))).unwrap_or_else(||id.clone()));
   let workspace=summary.and_then(|x|x.1.clone()).or_else(||dominant_cwd(&rs));
   out.push(summary_from_json(&path,"antigravity",id,title,workspace,rs));
  }
 }
 out
}
fn dsh_root() -> PathBuf { std::env::var_os("DSH_HOME").map(PathBuf::from).unwrap_or_else(||home().join(".dsh")).join("sessions") }
fn dsh_sessions() -> Vec<SessionSummary> { let root=dsh_root(); let mut files=Vec::new(); walk(&root,0,&mut files); files.into_iter().filter(|p|p.file_name().and_then(|x|x.to_str()).map(|n| n.starts_with("session") && (n.ends_with(".jsonl")||n.ends_with(".zstd"))).unwrap_or(false)).filter_map(|p| { let rs=records(&p); let parent=p.parent()?; let id=rs.iter().find_map(|r|r.get("type").and_then(Value::as_str).filter(|t|*t=="session").and_then(|_|r.get("id")).and_then(Value::as_str)).or_else(||parent.file_name().and_then(|x|x.to_str()))?.to_string(); let title=rs.iter().rev().find_map(|r| if r.get("type").and_then(Value::as_str)==Some("session/title"){r.get("data").and_then(|d|d.get("title")).and_then(Value::as_str).map(str::to_string)}else{None}).unwrap_or_else(||first_user(&rs,"dsh")); let workspace=parent.parent().and_then(|p|p.file_name()).and_then(|x|x.to_str()).map(str::to_string); Some(summary_from_json(&p,"dsh",id,title,workspace,rs)) }).collect() }
fn zcode_sessions() -> Vec<SessionSummary> { let path=home().join(".zcode/cli/db/db.sqlite"); let Ok(conn)=rusqlite::Connection::open_with_flags(&path,rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) else { return vec![] }; let Ok(mut stmt)=conn.prepare("SELECT id, title, directory, time_updated, time_created FROM session WHERE parent_id IS NULL ORDER BY time_updated DESC") else {return vec![]}; let rows=stmt.query_map([],|row| Ok(SessionSummary{id:row.get(0)?,title:row.get::<_,String>(1).unwrap_or_default(),source:"zcode".to_string(),workspace:row.get::<_,Option<String>>(2)?,updated_at:row.get::<_,i64>(3).unwrap_or_else(|_|row.get::<_,i64>(4).unwrap_or_default()).to_string(),message_count:0,tool_call_count:0,status:"ready".to_string(),source_path:path_string(&path)})); rows.map(|iter|iter.filter_map(Result::ok).collect()).unwrap_or_default() }
fn source(id:&str,label:&str,path:PathBuf,count:usize)->SourceLocation { SourceLocation{id:id.to_string(),provider:id.to_string(),label:label.to_string(),path:path_string(&path),session_count:count,status:if count>0{"ready"}else{"unavailable"}.to_string(),last_scanned:Some(now_string())} }
fn scan_sources()->Vec<SourceLocation>{ let c=scan_sessions("claude-code".into()); let x=scan_sessions("codex".into()); let a=scan_sessions("antigravity".into()); let d=scan_sessions("dsh".into()); let z=scan_sessions("zcode".into()); vec![source("claude-code","Claude Code",home().join(".claude/projects"),c.len()),source("antigravity","Antigravity",home().join(".gemini/antigravity"),a.len()),source("codex","Codex",home().join(".codex/sessions"),x.len()),source("zcode","Zcode",home().join(".zcode/cli/db/db.sqlite"),z.len()),source("dsh","DeepSeek Harness",dsh_root(),d.len())] }
fn raw_sessions(provider:String)->Vec<SessionSummary>{ match provider.as_str(){"claude-code"=>claude_sessions(),"antigravity"=>antigravity_sessions(),"codex"=>codex_sessions(),"zcode"=>zcode_sessions(),"dsh"=>dsh_sessions(),_=>vec![]} }

pub(crate) fn scan_sessions(provider:String) -> Vec<SessionSummary> {
 let mut unique=std::collections::HashMap::<String,SessionSummary>::new();
 for session in raw_sessions(provider) {
  match unique.get(&session.id) {
   Some(previous) if previous.updated_at >= session.updated_at => {},
   _ => {unique.insert(session.id.clone(),session);}
  }
 }
 let mut result:Vec<_>=unique.into_values().collect();
 result.sort_by(|a,b|b.updated_at.cmp(&a.updated_at));result
}


#[cfg(test)]
mod tests {
 use super::*;
 #[test] fn windows_transcript_path_is_normalized() {
  assert!(path_string(Path::new(r"C:\Users\test\.gemini\antigravity\brain\id\.system_generated\logs\transcript.jsonl")).contains(".system_generated/logs"));
 }
 #[test] fn message_count_excludes_metadata_and_counts_tool_use() {
  let records=vec![serde_json::json!({"type":"session_meta"}),serde_json::json!({"type":"user","message":{"content":"question"}}),serde_json::json!({"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read"}]}})];
  assert_eq!(count_messages(&records),2); assert_eq!(count_tool_calls(&records),1);
 }
 #[test] fn both_zstd_extensions_are_decoded() {
  for extension in ["zst","zstd"] {
   let path=std::env::temp_dir().join(format!("cyanbird-test-{}.{}",std::process::id(),extension));
   let payload=b"{\"type\":\"session\",\"id\":\"fixture\"}\n";
   fs::write(&path,zstd::stream::encode_all(&payload[..],1).unwrap()).unwrap();
   assert_eq!(records(&path)[0]["id"],"fixture"); fs::remove_file(path).unwrap();
  }
 }
 #[test] fn local_scan_diagnostics() {
  if std::env::var_os("CYANBIRD_SCAN_DIAGNOSTICS").is_none() { return; }
  for source in scan_sources() { println!("{}: {}",source.provider,source.session_count); }
 }
}

#[tauri::command]
pub async fn discover_sources() -> Result<Vec<SourceLocation>,String> {
 tauri::async_runtime::spawn_blocking(scan_sources).await.map_err(|e|e.to_string())
}
#[tauri::command]
pub async fn discover_sessions(provider:String) -> Result<Vec<SessionSummary>,String> {
 tauri::async_runtime::spawn_blocking(move || scan_sessions(provider)).await.map_err(|e|e.to_string())
}
