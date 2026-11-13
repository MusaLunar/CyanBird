//! Minimal historical-text trajectories for Antigravity Desktop 2.29.
//! Wire field observations: xhluca/session-migrate (MIT), see docs/antigravity-import.md.
use crate::migration::Message;
use rusqlite::{Connection, OpenFlags, params};
use std::{fs,io::{Read,Write,Seek,SeekFrom},path::{Path,PathBuf},time::Duration};
use uuid::Uuid;

fn varint(mut n:u64)->Vec<u8>{let mut out=Vec::new();while n>127{out.push((n as u8&127)|128);n>>=7;}out.push(n as u8);out}
fn number(field:u64,n:u64)->Vec<u8>{let mut out=varint(field<<3);out.extend(varint(n));out}
fn bytes(field:u64,data:&[u8])->Vec<u8>{let mut out=varint((field<<3)|2);out.extend(varint(data.len() as u64));out.extend(data);out}
fn text(field:u64,value:&str)->Vec<u8>{bytes(field,value.as_bytes())}

pub fn root(base:&Path)->PathBuf{base.join(".gemini/antigravity")}
pub(crate) fn project_workspace(root:&Path,id:&str)->Option<String>{
 if Uuid::parse_str(id).is_err(){return None}
 let raw=fs::read_to_string(root.parent()?.join("config/projects").join(format!("{id}.json"))).ok()?;
 let value:serde_json::Value=serde_json::from_str(&raw).ok()?;
 value["projectResources"]["resources"].as_array()?.iter().find_map(|r|r["folderUri"].as_str().or_else(||r["gitFolder"]["folderUri"].as_str()).map(crate::metadata::workspace_uri))
}
fn find_project(root:&Path,workspace:&str)->Option<String>{
 let directory=root.parent()?.join("config/projects");
 for entry in fs::read_dir(directory).ok()?.flatten(){let id=entry.path().file_stem()?.to_str()?.to_string();
 if project_workspace(root,&id).is_some_and(|p|p.replace('\\',"/").trim_end_matches('/').eq_ignore_ascii_case(workspace.replace('\\',"/").trim_end_matches('/'))){return Some(id)}
 }None
}
pub fn preflight(root:&Path)->Result<(),String>{
 let db=Connection::open_with_flags(root.join("conversation_summaries.db"),OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|_|"请先启动 Antigravity Desktop 初始化会话存储（当前支持 2.29 SQLite 格式）".to_string())?;
 db.prepare("SELECT conversation_id,title,preview,step_count,last_modified_time,workspace_uris,last_user_input_time,last_user_input_step_index,app_data_dir,raw_summary FROM conversation_summaries LIMIT 0").map_err(|e|format!("不支持此 Antigravity 索引格式: {e}"))?;
 Ok(())
}
pub fn require_closed()->Result<(),String>{
 #[cfg(target_os="windows")]{
 use std::os::windows::process::CommandExt;
 let output=std::process::Command::new("tasklist.exe").args(["/FI","IMAGENAME eq Antigravity.exe","/FO","CSV","/NH"]).creation_flags(0x08000000).output().map_err(|e|e.to_string())?;
 if !output.status.success(){return Err("Unable to check Antigravity process state".into())}
 if String::from_utf8_lossy(&output.stdout).to_lowercase().contains("\"antigravity.exe\""){return Err("请先完全退出 Antigravity，再执行迁入，以免客户端缓存覆盖会话索引。".into())}
 }Ok(())
}
fn take_varint(data:&[u8],pos:&mut usize)->Result<u64,String>{let mut value=0u64;for shift in (0..70).step_by(7){let b=*data.get(*pos).ok_or("Truncated Antigravity index")?;*pos+=1;if shift==63&&b>1{return Err("Invalid index varint".into())}value|=((b&127) as u64)<<shift;if b<128{return Ok(value)}}Err("Invalid index varint".into())}
fn fields(data:&[u8])->Result<Vec<(u64,&[u8])>,String>{
 let mut pos=0;let mut out=Vec::new();while pos<data.len(){if out.len()>100_000{return Err("Antigravity index field limit".into())}let tag=take_varint(data,&mut pos)?;if tag>>3==0{return Err("Invalid index field".into())}let start=pos;let len=match tag&7{0=>{take_varint(data,&mut pos)?;0},1=>8,2=>usize::try_from(take_varint(data,&mut pos)?).map_err(|e|e.to_string())?,5=>4,_=>return Err("Unsupported index wire type".into())};let content=pos;pos=pos.checked_add(len).filter(|p|*p<=data.len()).ok_or("Truncated index field")?;out.push((tag,if tag&7==2{&data[content..pos]}else{&data[start..pos]}));}Ok(out)
}
fn check_index(data:&[u8],id:&str)->Result<(),String>{for(tag,map)in fields(data)?{if tag==10{for (field,value) in fields(map)?{if field==10&&value==id.as_bytes(){return Err("Antigravity conversation already indexed".into())}}}}Ok(())}
fn workspace_uri(workspace:&str)->String{
 let normalized=workspace.replace('\\',"/");let prefix=if normalized.starts_with('/') {"file://"}else{"file:///"};
 const URI:&percent_encoding::AsciiSet=&percent_encoding::NON_ALPHANUMERIC.remove(b'/').remove(b':').remove(b'-').remove(b'_').remove(b'.').remove(b'~');
 let encoded=percent_encoding::utf8_percent_encode(&normalized, URI).to_string();format!("{prefix}{encoded}")
}
const SCHEMA:&str=r#"
-- Version zero makes Desktop reinitialize trajectory IDs and metadata.
PRAGMA user_version=1;
CREATE TABLE trajectory_meta(trajectory_id text PRIMARY KEY,cascade_id text,trajectory_type integer,source integer);
CREATE TABLE steps(idx integer PRIMARY KEY,step_type integer NOT NULL DEFAULT 0,status integer NOT NULL DEFAULT 0,has_subtrajectory numeric NOT NULL DEFAULT false,metadata blob,error_details blob,permissions blob,task_details blob,render_info blob,step_payload blob,step_format integer NOT NULL DEFAULT 0);
CREATE INDEX idx_steps_status ON steps(status);CREATE INDEX idx_steps_step_type ON steps(step_type);
CREATE TABLE gen_metadata(idx integer PRIMARY KEY,data blob,size integer NOT NULL DEFAULT 0);
CREATE TABLE executor_metadata(idx integer PRIMARY KEY,data blob);
CREATE TABLE parent_references(idx integer PRIMARY KEY,data blob);
CREATE TABLE trajectory_metadata_blob(id text PRIMARY KEY DEFAULT 'main',data blob);
CREATE TABLE battle_mode_infos(idx integer PRIMARY KEY,data blob);
"#;
fn build(path:&Path,id:&str,project:&str,uri:&str,messages:&[Message],seconds:u64)->Result<(String,Vec<u8>),String>{
 let mut db=Connection::open(path).map_err(|e|e.to_string())?;
 db.execute_batch(SCHEMA).map_err(|e|e.to_string())?;
 let tx=db.transaction().map_err(|e|e.to_string())?;
 let trajectory=Uuid::new_v4().to_string();
 tx.execute("INSERT INTO trajectory_meta VALUES(?1,?2,4,1)",params![trajectory,id]).map_err(|e|e.to_string())?;
 for (idx,m) in messages.iter().enumerate(){
 let assistant=m.role=="assistant";let kind=if assistant{15}else{14};
 let mut payload=text(if assistant{1}else{2},&m.text);if assistant{payload.extend(text(6,&Uuid::new_v4().to_string()));}else{payload.extend(bytes(3,&text(1,&m.text)));}
 let mut step_metadata=bytes(1,&number(1,seconds));step_metadata.extend(number(3,if assistant{2}else{4}));
 let mut step=number(1,kind);step.extend(number(4,3));step.extend(bytes(5,&step_metadata));step.extend(bytes(if assistant{20}else{19},&payload));
 tx.execute("INSERT INTO steps(idx,step_type,status,metadata,step_payload) VALUES(?1,?2,3,?3,?4)",params![idx as i64,kind as i64,step_metadata,step]).map_err(|e|e.to_string())?;
 }
 let workspace_record=text(1,uri);
 let mut metadata=bytes(1,&workspace_record);metadata.extend(bytes(2,&number(1,seconds)));metadata.extend(text(3,&Uuid::new_v4().to_string()));metadata.extend(text(6,id));metadata.extend(text(7,uri));metadata.extend(text(18,project));
 tx.execute("INSERT INTO trajectory_metadata_blob VALUES('main',?1)",[&metadata]).map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())?;
 let check:String=db.query_row("PRAGMA quick_check",[],|r|r.get(0)).map_err(|e|e.to_string())?;if check!="ok"{return Err(check)}drop(db);
 fs::OpenOptions::new().read(true).write(true).open(path).and_then(|f|f.sync_all()).map_err(|e|e.to_string())?;Ok((trajectory,metadata))
}

/// Reserves the index ID under an immediate transaction, publishes only new files,
/// then commits. A failed commit removes only this import's newly published files.
pub fn install(root:&Path,id:&str,title:&str,workspace:&str,messages:&[Message])->Result<(),String>{
 // Desktop presents a turn's final planner response. Keep all progress messages
 // and historical tool text in that same response rather than replaceable steps.
 let mut turns:Vec<Message>=Vec::new();
 for message in messages{if message.role=="assistant"&&turns.last().is_some_and(|m|m.role=="assistant"){let previous=turns.last_mut().unwrap();previous.text.push_str("\n\n---\n\n");previous.text.push_str(&message.text);}else{turns.push(message.clone());}}
 let messages=turns.as_slice();
 if Uuid::parse_str(id).is_err()||messages.is_empty(){return Err("Invalid Antigravity conversation".into())}
 preflight(root)?;
 let mut index=Connection::open_with_flags(root.join("conversation_summaries.db"),OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(|e|e.to_string())?;
 index.busy_timeout(Duration::from_secs(5)).map_err(|e|e.to_string())?;
 let tx=index.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
 // Desktop uses this protobuf map as the authoritative trajectory-ID lookup,
 // in addition to the SQLite picker index. Hold an exclusive Windows handle.
 let proto_path=root.join("agyhub_summaries_proto.pb");let mut options=fs::OpenOptions::new();options.read(true).write(true).create(true);
 #[cfg(target_os="windows")]{use std::os::windows::fs::OpenOptionsExt;options.share_mode(0);}
 let mut proto=options.open(&proto_path).map_err(|e|e.to_string())?;let mut original=Vec::new();if proto.metadata().map_err(|e|e.to_string())?.len()>64*1024*1024{return Err("Antigravity index is too large".into())}proto.read_to_end(&mut original).map_err(|e|e.to_string())?;check_index(&original,id)?;
 let uri=workspace_uri(workspace);
 let existing_project=find_project(root,workspace);let project=existing_project.clone().unwrap_or_else(||Uuid::new_v4().to_string());
 let project_file=root.parent().ok_or("Invalid Antigravity root")?.join("config/projects").join(format!("{project}.json"));
 let now=time::OffsetDateTime::now_utc();let stamp=now.format(&time::format_description::well_known::Rfc3339).map_err(|e|e.to_string())?;
 let preview=messages.iter().find(|m|m.role=="user").unwrap_or(&messages[0]).text.chars().take(1024).collect::<String>();
 let last_user=messages.iter().rposition(|m|m.role=="user").map(|i|i as i64).unwrap_or(-1);
 tx.execute("INSERT INTO conversation_summaries(conversation_id,title,preview,step_count,last_modified_time,workspace_uris,status,project_id,last_user_input_time,last_user_input_step_index,app_data_dir) VALUES(?1,?2,?3,?4,?5,?6,'CASCADE_RUN_STATUS_IDLE',?7,?5,?8,'antigravity')",params![id,title,preview,messages.len() as i64,stamp,serde_json::to_string(&vec![&uri]).unwrap(),project,last_user]).map_err(|e|e.to_string())?;
 let conversations=root.join("conversations");fs::create_dir_all(&conversations).map_err(|e|e.to_string())?;
 let final_path=conversations.join(format!("{id}.db"));let staged=conversations.join(format!(".cyanbird-{}.tmp",Uuid::new_v4()));
 let brain=root.join("brain").join(id);let annotation=root.join("annotations").join(format!("{id}.pbtxt"));let mut published=false;let mut brain_created=false;let mut annotation_created=false;let mut project_created=false;let mut proto_changed=false;
 let result=(||{
 let seconds=now.unix_timestamp() as u64;let (trajectory,metadata)=build(&staged,id,&project,&uri,messages,seconds)?;
 // Keep the desktop SQLite and protobuf index entries aligned with the exact
 // generated trajectory ID, instead of relying on client-side reconstruction.
 let mut summary=text(1,title);summary.extend(number(2,messages.len() as u64));summary.extend(bytes(3,&number(1,seconds)));summary.extend(text(4,&trajectory));summary.extend(number(5,1));summary.extend(bytes(7,&number(1,seconds)));summary.extend(bytes(10,&number(1,seconds)));if last_user>=0{summary.extend(number(16,last_user as u64));}summary.extend(bytes(17,&metadata));
 tx.execute("UPDATE conversation_summaries SET raw_summary=?1 WHERE conversation_id=?2",params![&summary,id]).map_err(|e|e.to_string())?;
 if existing_project.is_none(){
 fs::create_dir_all(project_file.parent().unwrap()).map_err(|e|e.to_string())?;
 let mut f=fs::OpenOptions::new().write(true).create_new(true).open(&project_file).map_err(|e|e.to_string())?;project_created=true;
 let data=serde_json::json!({"id":project,"name":workspace.rsplit(['/', '\\']).next().unwrap_or("Imported project"),"projectResources":{"resources":[{"folderUri":uri}]},"settings":{},"isWorkspaceOnly":true});f.write_all(data.to_string().as_bytes()).and_then(|_|f.sync_all()).map_err(|e|e.to_string())?;
 }
 fs::create_dir_all(root.join("brain")).map_err(|e|e.to_string())?;
 fs::create_dir(&brain).map_err(|e|e.to_string())?;brain_created=true;
 let logs=brain.join(".system_generated/logs");fs::create_dir_all(&logs).map_err(|e|e.to_string())?;
 let transcript=messages.iter().enumerate().map(|(i,m)|serde_json::json!({"step_index":i,"source":if m.role=="assistant"{"MODEL"}else{"USER_EXPLICIT"},"type":if m.role=="assistant"{"PLANNER_RESPONSE"}else{"USER_INPUT"},"status":"DONE","created_at":stamp,"content":m.text}).to_string()).collect::<Vec<_>>().join("\n")+"\n";
 fs::write(logs.join("transcript.jsonl"),transcript).map_err(|e|e.to_string())?;
 fs::create_dir_all(root.join("annotations")).map_err(|e|e.to_string())?;
 let mut f=fs::OpenOptions::new().write(true).create_new(true).open(&annotation).map_err(|e|e.to_string())?;annotation_created=true;
 f.write_all(format!("title: {}\n",serde_json::to_string(title).map_err(|e|e.to_string())?).as_bytes()).and_then(|_|f.sync_all()).map_err(|e|e.to_string())?;drop(f);
 fs::hard_link(&staged,&final_path).map_err(|e|e.to_string())?;published=true;
 let mut entry=text(1,id);entry.extend(bytes(2,&summary));let addition=bytes(1,&entry);proto_changed=true;proto.seek(SeekFrom::End(0)).and_then(|_|proto.write_all(&addition)).and_then(|_|proto.sync_all()).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(())
 })();
 let _=fs::remove_file(&staged);
 if result.is_err(){if proto_changed{proto.set_len(original.len() as u64).and_then(|_|proto.sync_all()).map_err(|e|format!("Index rollback failed: {e}"))?;}if published{let _=fs::remove_file(&final_path);}if brain_created{let _=fs::remove_dir_all(&brain);}if annotation_created{let _=fs::remove_file(&annotation);}if project_created{let _=fs::remove_file(&project_file);}}
 result
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn index_validation_rejects_corruption_and_duplicate_ids(){
 let mut entry=text(1,"existing");entry.extend(bytes(2,&text(1,"Title")));let index=bytes(1,&entry);
 assert!(check_index(&index,"new").is_ok());assert!(check_index(&index,"existing").is_err());
 for invalid in [vec![0],vec![10,255],vec![15],vec![128;11]]{assert!(check_index(&invalid,"new").is_err());}
 }
 #[test]fn native_fixture_and_collision_rollback(){
 let fixture=std::env::temp_dir().join(format!("cyanbird-ag-{}",Uuid::new_v4()));let temp=std::env::var_os("CYANBIRD_ANTIGRAVITY_QA").map(PathBuf::from).unwrap_or_else(||fixture.join("antigravity"));fs::create_dir_all(&temp).unwrap();
 let db=Connection::open(temp.join("conversation_summaries.db")).unwrap();
 db.execute_batch(include_str!("../tests/antigravity-summary.sql")).unwrap();
 let id="e4095dd8-520c-47f6-8c6f-0441c208c1af";let messages=vec![Message{role:"user".into(),text:"CYANBIRD_USER_ALPHA".into()},Message{role:"assistant".into(),text:"CYANBIRD_PROGRESS_FIRST".into()},Message{role:"assistant".into(),text:"CYANBIRD_PROGRESS_MIDDLE".into()},Message{role:"assistant".into(),text:"CYANBIRD_ASSISTANT_BETA\n\n```rust\nfn main() {}\n```".into()}];
 let mut records:Vec<serde_json::Value>=messages.iter().map(|m|serde_json::json!({"type":"response_item","payload":{"type":"message","role":m.role,"content":[{"type":"text","text":m.text}]}})).collect();records.insert(2,serde_json::json!({"type":"response_item","payload":{"type":"function_call","name":"sleep","arguments":"TOOL_ARCHIVE_OMIT"}}));records.insert(3,serde_json::json!({"type":"response_item","payload":{"type":"reasoning","summary":[{"text":"THINKING_ARCHIVE_OMIT"}]}}));
 let source=crate::discovery::SessionSummary{id:"source-fixture".into(),title:"Fixture".into(),source:"codex".into(),workspace:None,updated_at:"0".into(),message_count:4,tool_call_count:1,status:"ready".into(),source_path:"fixture.jsonl".into()};let messages=crate::migration::for_target(&source,&records,"antigravity");
 let workspace=temp.parent().unwrap().join("Project 中文");fs::create_dir_all(&workspace).unwrap();let workspace=workspace.to_string_lossy();
 crate::migration::write_target(&temp,"antigravity",id,"CyanBird fixture",&workspace,&messages).unwrap();
 assert!(find_project(&temp,&workspace).is_some());
 let original_index=fs::read(temp.join("agyhub_summaries_proto.pb")).unwrap();
 let native=Connection::open(temp.join("conversations").join(format!("{id}.db"))).unwrap();assert_eq!(native.query_row("PRAGMA user_version",[],|r|r.get::<_,i64>(0)).unwrap(),1);assert_eq!(native.query_row("SELECT count(*) FROM steps",[],|r|r.get::<_,i64>(0)).unwrap(),2);
 let user_payload:Vec<u8>=native.query_row("SELECT step_payload FROM steps WHERE idx=0",[],|r|r.get(0)).unwrap();let expected_item=bytes(3,&text(1,"CYANBIRD_USER_ALPHA"));assert!(user_payload.windows(expected_item.len()).any(|w|w==expected_item.as_slice()));drop(native);
 let path=temp.join("conversations").join(format!("{id}.db"));let original=fs::read(&path).unwrap();assert!(install(&temp,id,"Collision","E:/Project",&messages).is_err());assert_eq!(fs::read(&path).unwrap(),original);
 let blocked=Uuid::new_v4().to_string();fs::write(temp.join("conversations").join(format!("{blocked}.db")),b"existing").unwrap();assert!(install(&temp,&blocked,"Blocked","E:/Project",&messages).is_err());let count:i64=db.query_row("SELECT count(*) FROM conversation_summaries WHERE conversation_id=?1",[&blocked],|r|r.get(0)).unwrap();assert_eq!(count,0);assert!(!temp.join("brain").join(&blocked).exists());
 assert_eq!(workspace_uri("E:\\Project 中文"),"file:///E:/Project%20%E4%B8%AD%E6%96%87");
 fs::remove_file(temp.join("conversations").join(format!("{blocked}.db"))).unwrap();
 assert_eq!(fs::read(temp.join("agyhub_summaries_proto.pb")).unwrap(),original_index);
 drop(db);if std::env::var_os("CYANBIRD_ANTIGRAVITY_QA").is_none(){fs::remove_dir_all(fixture).unwrap();}
 }
}
