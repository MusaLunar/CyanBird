use crate::{discovery::{home,SessionSummary}, storage::{chosen,read_export_records,markdown_records,preferences,backup_to}};
use serde::{Serialize,Deserialize};
use serde_json::{json,Value};
use std::{fs,io::Write,path::{Path,PathBuf}};
use uuid::Uuid;

#[derive(Clone,Serialize,Deserialize)]
pub struct Message { pub role:String, pub text:String }
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Plan { pub target_directory:String, pub session_count:usize, pub message_count:usize, pub warnings:Vec<String> }
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Item { pub source_id:String, pub target_id:String, pub path:String, pub status:String, pub error:Option<String> }
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Receipt { pub backup_directory:String, pub items:Vec<Item> }
fn validate(target:&str,workspace:&str)->Result<(),String>{
 if !["claude-code","codex","dsh","zcode"].contains(&target){return Err("This target has no verified native writer (Antigravity is source-only)".into())}
 if !Path::new(workspace).is_absolute()||!Path::new(workspace).is_dir(){return Err("Choose an existing absolute project directory".into())}Ok(())
}
fn slug(workspace:&str)->String{workspace.chars().map(|c|if c.is_ascii_alphanumeric()||c=='_'||c=='-' {c}else{'-'}).collect()}
fn target_directory(base:&Path,target:&str,workspace:&str)->PathBuf{
 match target{
 "claude-code"=>base.join(".claude/projects").join(slug(workspace)),
 "codex"=>base.join(".codex/sessions"),
 "dsh"=>std::env::var_os("DSH_HOME").map(PathBuf::from).unwrap_or_else(||base.join(".dsh")).join("sessions").join(format!("--{}--",slug(workspace))),
 "zcode"=>base.join(".zcode/cli/db"),_=>base.to_path_buf()
 }
}
pub fn normalize(summary:&SessionSummary,records:&[Value])->Vec<Message>{
 records.iter().filter_map(|r|{
 let t=r["type"].as_str().unwrap_or("");
 let role=match summary.source.as_str(){
 "claude-code" if t=="user"||t=="assistant"=>t,
 "codex" if t=="response_item"=>r["payload"]["role"].as_str().unwrap_or("user"),
 "antigravity" if t=="USER_INPUT"=>"user",
 "antigravity" if ["PLANNER_RESPONSE","GENERIC","ERROR_MESSAGE"].contains(&t)=>"assistant",
 "dsh" if t=="user/message"=>"user",
 "dsh" if t=="assistant/message"||t.contains("tool")=>"assistant",
 "zcode"=>r["role"].as_str().unwrap_or("user"),
 _=>return None,
 };
 let mut markdown=markdown_records(summary,&[r.clone()]);
 // DSH assistant payloads nest their message content, unlike user/message.
 if summary.source=="dsh" && t=="assistant/message" {markdown=crate::discovery::text_content(&r["data"]["message"]["content"]);}
 let text=if let Some((_,body))=markdown.split_once("\n\n## "){body.split_once("\n\n").map(|(_,body)|body.trim().to_string()).unwrap_or_default()}else if summary.source=="dsh" && t=="assistant/message" {markdown}else{String::new()};
 if text.is_empty(){return None}Some(Message{role:if role=="assistant"{"assistant"}else{"user"}.into(),text})
 }).collect()
}
fn push_event(events:&mut Vec<Value>,kind:&str,data:Value,surface:bool,now:i128){
 let mut event=json!({"type":kind,"seq":events.len(),"time":now/1_000_000,"data":data});if surface{event["surfaceOp"]=json!("append");}events.push(event);
}
pub fn serialize(target:&str,id:&str,title:&str,workspace:&str,messages:&[Message])->Result<String,String>{
 let now=time::OffsetDateTime::now_utc();let stamp=now.format(&time::format_description::well_known::Rfc3339).map_err(|e|e.to_string())?;let millis=now.unix_timestamp_nanos()/1_000_000;
 let mut records=Vec::new();
 match target {
 "claude-code"=>{
 let mut parent:Option<String>=None;
 for (i,m) in messages.iter().enumerate(){let uuid=Uuid::new_v4().to_string();let message=if m.role=="assistant"{json!({"role":"assistant","type":"message","id":format!("msg_{}",Uuid::new_v4().simple()),"model":"cyanbird-import","content":[{"type":"text","text":m.text}],"stop_reason":"end_turn","usage":{"input_tokens":0,"output_tokens":0}})}else{json!({"role":"user","content":m.text})};records.push(json!({"type":m.role,"uuid":uuid,"parentUuid":parent,"sessionId":id,"timestamp":stamp,"cwd":workspace,"isSidechain":false,"message":message}));parent=Some(uuid);if i==0{records.push(json!({"type":"custom-title","sessionId":id,"customTitle":title}));}}
 },
 "codex"=>{records.push(json!({"type":"session_meta","timestamp":stamp,"payload":{"id":id,"timestamp":stamp,"cwd":workspace,"originator":"cyanbird","source":"cli"}}));for m in messages {records.push(json!({"type":"response_item","timestamp":stamp,"payload":{"type":"message","role":m.role,"content":[{"type":if m.role=="assistant"{"output_text"}else{"input_text"},"text":m.text}]}}));}},
 "dsh"=>{
 records.push(json!({"type":"session","id":id,"version":4,"cwd":workspace,"createdAt":millis,"isSeeded":false}));
 let mut events=Vec::new();
 for (i,m) in messages.iter().enumerate(){let turn=i+1;
 push_event(&mut events,"turn/start",json!({"turn":turn}),false,now.unix_timestamp_nanos());
 push_event(&mut events,"step/start",json!({"turn":turn,"step":1}),false,now.unix_timestamp_nanos());
 if i==0{push_event(&mut events,"system/message",json!({"turn":turn,"step":1,"message":{"id":format!("{id}:system"),"role":"system","content":[],"source":{"kind":"system-prompt"}}}),true,now.unix_timestamp_nanos());}
 if m.role=="assistant"{push_event(&mut events,"assistant/message",json!({"turn":turn,"step":1,"stream":[],"message":{"id":format!("{id}:{i}"),"role":"assistant","content":[{"type":"text","text":m.text}],"source":{"kind":"model","provider":"cyanbird","model":"imported-history"}}}),true,now.unix_timestamp_nanos());}
 else{push_event(&mut events,"user/message",json!({"id":format!("{id}:{i}"),"role":"user","source":{"kind":"user"},"content":[{"type":"text","text":m.text}]}),true,now.unix_timestamp_nanos());}
 push_event(&mut events,"step/end",json!({"turn":turn,"step":1}),false,now.unix_timestamp_nanos());push_event(&mut events,"turn/end",json!({"turn":turn,"reason":{"kind":"completed"}}),false,now.unix_timestamp_nanos());
 }
 push_event(&mut events,"session/title",json!({"title":title,"messageSeqs":[],"source":{"kind":"user"}}),false,now.unix_timestamp_nanos());records.extend(events);
 },_=>return Err("No JSONL writer".into())
 }
 Ok(records.iter().map(|r|serde_json::to_string(r).unwrap()).collect::<Vec<_>>().join("\n")+"\n")
}
fn write_sqlite(path:&Path,id:&str,title:&str,workspace:&str,messages:&[Message])->Result<(),String>{
 let mut db=rusqlite::Connection::open_with_flags(path,rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(|e|e.to_string())?;
 let tx=db.transaction().map_err(|e|e.to_string())?;let now=(time::OffsetDateTime::now_utc().unix_timestamp_nanos()/1_000_000) as i64;
 let version:String=tx.query_row("SELECT version FROM session ORDER BY time_updated DESC LIMIT 1",[],|r|r.get(0)).map_err(|e|e.to_string())?;
 let project=tx.query_row("SELECT project_id FROM session WHERE directory=?1 LIMIT 1",[workspace],|r|r.get::<_,String>(0)).unwrap_or_else(|_|format!("proj_{}",slug(workspace).to_lowercase()));
 tx.execute("INSERT INTO session (id,project_id,slug,directory,path,title,version,time_created,time_updated) VALUES (?1,?2,?1,?3,?3,?4,?5,?6,?6)",rusqlite::params![id,project,workspace,title,version,now]).map_err(|e|e.to_string())?;
 for (i,m) in messages.iter().enumerate(){let mid=format!("msg_{}",Uuid::new_v4().simple());let pid=format!("part_{}",Uuid::new_v4().simple());let mut data=json!({"role":m.role,"time":{"created":now+i as i64},"agent":"zcode-agent","path":{"cwd":workspace,"root":workspace}}); if m.role=="assistant" { if let Ok(raw)=tx.query_row("SELECT data FROM message WHERE json_extract(data, '$.role')='assistant' ORDER BY time_updated DESC LIMIT 1",[],|r|r.get::<_,String>(0)) {if let Ok(previous)=serde_json::from_str::<Value>(&raw){for key in ["modelID","providerID","mode","agent"]{if let Some(value)=previous.get(key){data[key]=value.clone();}}}}data["finish"]=json!("stop");}
 tx.execute("INSERT INTO message(id,session_id,time_created,time_updated,data) VALUES(?1,?2,?3,?3,?4)",rusqlite::params![mid,id,now+i as i64,data.to_string()]).map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO part(id,message_id,session_id,time_created,time_updated,data) VALUES(?1,?2,?3,?4,?4,?5)",rusqlite::params![pid,mid,id,now+i as i64,json!({"type":"text","text":m.text}).to_string()]).map_err(|e|e.to_string())?;
 }
 let count:i64=tx.query_row("SELECT count(*) FROM message WHERE session_id=?1",[id],|r|r.get(0)).map_err(|e|e.to_string())?;if count!=messages.len() as i64{return Err("Target message count mismatch".into())}tx.commit().map_err(|e|e.to_string())
}
fn write_jsonl(path:&Path,text:&str)->Result<(),String>{
 let parent=path.parent().ok_or("Invalid target path")?;fs::create_dir_all(parent).map_err(|e|e.to_string())?;
 let staging=parent.join(format!(".cyanbird-{}.tmp",Uuid::new_v4()));
 let mut f=fs::OpenOptions::new().write(true).create_new(true).open(&staging).map_err(|e|e.to_string())?;f.write_all(text.as_bytes()).map_err(|e|e.to_string())?;f.sync_all().map_err(|e|e.to_string())?;drop(f);
 let read=fs::read_to_string(&staging).map_err(|e|e.to_string())?;if read!=text{return Err("Target byte verification failed".into())}
 for line in read.lines(){serde_json::from_str::<Value>(line).map_err(|e|e.to_string())?;}
 // A hard link publishes without replacing an existing destination; staging is on the same volume.
 fs::hard_link(&staging,path).map_err(|e|e.to_string())?;fs::remove_file(staging).map_err(|e|e.to_string())?;Ok(())
}
#[tauri::command]
pub async fn preview_migration(provider:String,ids:Vec<String>,target:String,workspace:String)->Result<Plan,String>{
 tauri::async_runtime::spawn_blocking(move||{validate(&target,&workspace)?;let sessions=chosen(provider,ids)?;let mut count=0;for s in &sessions{let records=read_export_records(s)?;let messages=normalize(s,&records);if messages.is_empty(){return Err("A selected session has no transferable messages".into())}count+=messages.len();}
 Ok(Plan{target_directory:target_directory(&home(),&target,&workspace).to_string_lossy().into(),session_count:sessions.len(),message_count:count,warnings:vec!["Tool calls, results and reasoning are archived as historical text; they are not replayed. Binary attachments are not copied. Source files are preserved in a mandatory backup.".into()]})}).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn execute_migration(provider:String,ids:Vec<String>,target:String,workspace:String,acknowledged:bool)->Result<Receipt,String>{
 tauri::async_runtime::spawn_blocking(move||{validate(&target,&workspace)?;if !acknowledged{return Err("Review and acknowledge conversion differences first".into())}let sessions=chosen(provider.clone(),ids)?;let prefs=preferences()?;
 let backup=backup_to(Path::new(&prefs.backup_directory),&provider,sessions.clone())?;
 let directory=target_directory(&home(),&target,&workspace);
 if target=="zcode" {let db=directory.join("db.sqlite");if !db.exists(){return Err("Zcode database not found; initialize Zcode first".into())}let mut marker=sessions[0].clone();marker.source_path=db.to_string_lossy().into();backup_to(Path::new(&prefs.backup_directory),"zcode",vec![marker])?;}
 let mut items=Vec::new();
 for source in sessions{let uuid=Uuid::new_v4().to_string();let id=if target=="zcode"{format!("sess_{uuid}")}else{uuid};let now=time::OffsetDateTime::now_utc();let path=match target.as_str(){"claude-code"=>directory.join(format!("{id}.jsonl")),"codex"=>directory.join(format!("{:04}/{:02}/{:02}/rollout-{}-{id}.jsonl",now.year(),u8::from(now.month()),now.day(),format!("{:04}-{:02}-{:02}T{:02}-{:02}-{:02}",now.year(),u8::from(now.month()),now.day(),now.hour(),now.minute(),now.second()))),"dsh"=>directory.join(&id).join("session.v4.jsonl"),_=>directory.join("db.sqlite")};
 let result=(||{let mut snapshot=source.clone(); let stored=backup.files.iter().find(|f|f.original==source.source_path).ok_or("Missing source snapshot")?; snapshot.source_path=Path::new(&backup.directory).join(&stored.stored).to_string_lossy().into(); let records=read_export_records(&snapshot)?;let messages=normalize(&source,&records);if messages.is_empty(){return Err("No transferable messages".into())}if target=="zcode"{write_sqlite(&path,&id,&source.title,&workspace,&messages)}else{let text=serialize(&target,&id,&source.title,&workspace,&messages)?;write_jsonl(&path,&text)}})();
 items.push(Item{source_id:source.id,target_id:id,path:path.to_string_lossy().into(),status:if result.is_ok(){"written"}else{"failed"}.into(),error:result.err()});
 }
 let receipt=Receipt{backup_directory:backup.directory.clone(),items};fs::write(Path::new(&backup.directory).join("migration-report.json"),serde_json::to_vec_pretty(&receipt).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;Ok(receipt)
 }).await.map_err(|e|e.to_string())?
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn writes_native_envelopes_and_preserves_roles(){let messages=vec![Message{role:"user".into(),text:"question".into()},Message{role:"assistant".into(),text:"answer".into()}];for target in ["claude-code","codex","dsh"]{let text=serialize(target,"fixture-id","Fixture","C:/Project",&messages).unwrap();let records:Vec<Value>=text.lines().map(|l|serde_json::from_str(l).unwrap()).collect();assert!(text.contains("question")&&text.contains("answer"));assert!(!records.is_empty());if target=="dsh"{for (i,e) in records.iter().skip(1).enumerate(){assert_eq!(e["seq"],i);}assert_eq!(records[3]["type"],"system/message");}}}
 #[test]fn migration_fixture_roundtrip(){
 let messages=vec![Message{role:"user".into(),text:"Please inspect the project".into()},Message{role:"assistant".into(),text:"Historical tool result: all checks passed".into()}];
 if let Ok(root)=std::env::var("CYANBIRD_MIGRATION_QA"){fs::create_dir_all(&root).unwrap();for target in ["claude-code","codex","dsh"]{fs::write(Path::new(&root).join(format!("{target}.jsonl")),serialize(target,"e4095dd8-520c-47f6-8c6f-0441c208c1af","CyanBird fixture","E:/Programming/CyanBird",&messages).unwrap()).unwrap();}}
 }
 #[test]fn sqlite_writer_commits_messages_and_rolls_back_on_failure(){
 let dir=std::env::temp_dir().join(format!("cyanbird-sqlite-migration-{}",Uuid::new_v4()));fs::create_dir(&dir).unwrap();let path=dir.join("db.sqlite");let db=rusqlite::Connection::open(&path).unwrap();
 db.execute_batch("CREATE TABLE session(id TEXT PRIMARY KEY, project_id TEXT NOT NULL, slug TEXT NOT NULL,directory TEXT NOT NULL,path TEXT,title TEXT NOT NULL,version TEXT NOT NULL,time_created INTEGER NOT NULL,time_updated INTEGER NOT NULL);CREATE TABLE message(id TEXT PRIMARY KEY,session_id TEXT NOT NULL,time_created INTEGER NOT NULL,time_updated INTEGER NOT NULL,data TEXT NOT NULL);CREATE TABLE part(id TEXT PRIMARY KEY,message_id TEXT NOT NULL,session_id TEXT NOT NULL,time_created INTEGER NOT NULL,time_updated INTEGER NOT NULL,data TEXT NOT NULL);INSERT INTO session VALUES('existing','proj','slug','C:/Old','C:/Old','Existing','0.15.0',1,1);").unwrap();
 let messages=vec![Message{role:"user".into(),text:"question".into()},Message{role:"assistant".into(),text:"answer".into()}];
 write_sqlite(&path,"new","Imported","C:/Project",&messages).unwrap();let count:i64=db.query_row("SELECT count(*) FROM message WHERE session_id='new'",[],|r|r.get(0)).unwrap();assert_eq!(count,2);
 db.execute_batch("DROP TABLE part;").unwrap();assert!(write_sqlite(&path,"failed","Failed","C:/Project",&messages).is_err());let count:i64=db.query_row("SELECT count(*) FROM session WHERE id='failed'",[],|r|r.get(0)).unwrap();assert_eq!(count,0);drop(db);fs::remove_dir_all(dir).unwrap();
 }
 #[test]fn never_overwrites_existing_target(){let dir=std::env::temp_dir().join(format!("cyanbird-migration-{}",Uuid::new_v4()));fs::create_dir(&dir).unwrap();let path=dir.join("session.jsonl");write_jsonl(&path,"{\"first\":true}\n").unwrap();assert!(write_jsonl(&path,"{\"second\":true}\n").is_err());assert_eq!(fs::read_to_string(path).unwrap(),"{\"first\":true}\n");fs::remove_dir_all(dir).unwrap();}
}
