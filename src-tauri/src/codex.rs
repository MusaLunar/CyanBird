//! Codex desktop writes paginated rollout windows sharing one thread ID.
use crate::{discovery::{home,read_payload,SessionSummary},migration::Message};
use serde_json::Value;
use std::{collections::HashSet,fs,io::{BufRead,BufReader,Read},path::{Path,PathBuf}};

pub fn header(path:&Path)->Option<Value>{
 if matches!(path.extension().and_then(|p|p.to_str()),Some("zst"|"zstd")){return read_payload(path)?.lines().find(|l|!l.trim().is_empty()).and_then(|l|serde_json::from_str(l).ok())}
 let mut line=String::new();BufReader::new(fs::File::open(path).ok()?).take(2*1024*1024).read_line(&mut line).ok()?;serde_json::from_str(&line).ok()
}
pub fn paths(session:&SessionSummary)->Vec<PathBuf>{
 let selected=Path::new(&session.source_path);let native=home().join(".codex");
 if (selected.starts_with(native.join("sessions"))||selected.starts_with(native.join("archived_sessions")))&&selected.file_name().and_then(|n|n.to_str()).is_some_and(|n|n.starts_with("rollout-")){return crate::discovery::codex_history_paths(&session.id)}
 // Backups keep every rollout window as a sibling, so conversion uses snapshots.
 fs::read_dir(selected.parent().unwrap_or(Path::new("."))).into_iter().flatten().flatten().map(|e|e.path()).filter(|p|p.is_file()&&matches!(p.extension().and_then(|s|s.to_str()),Some("jsonl"|"zst"|"zstd"))&&header(p).is_some_and(|m|m["type"]=="session_meta"&&m["payload"]["id"].as_str()==Some(&session.id))).collect()
}
pub fn read(session:&SessionSummary)->Result<Vec<Value>,String>{
 let mut files=paths(session);if !files.iter().any(|p|p==Path::new(&session.source_path)){files.push(PathBuf::from(&session.source_path));}
 let mut fragments=Vec::new();
 for path in files{let raw=read_payload(&path).ok_or_else(||format!("Cannot read Codex history window: {}",path.display()))?;let rows=raw.lines().filter(|l|!l.trim().is_empty()).enumerate().map(|(i,l)|serde_json::from_str::<Value>(l).map_err(|e|format!("Malformed Codex history {}:{}: {e}",path.display(),i+1))).collect::<Result<Vec<_>,_>>()?;if !rows.is_empty(){fragments.push(rows);}}
 fragments.sort_by_key(|rs|rs[0]["payload"]["history_base"]["end_ordinal_exclusive"].as_u64().unwrap_or(0));
 let mut out:Vec<Value>=Vec::new();
 for mut rows in fragments{
  let first=&rows[0];let base=&first["payload"]["history_base"];
  if let Some(boundary)=base["end_ordinal_exclusive"].as_u64(){
   if base["thread_id"].as_str().is_some_and(|id|id!=session.id){return Err("Codex history references another thread; export that thread's history first".into())}
   if boundary>0&&!out.iter().any(|r|r["ordinal"].as_u64()==Some(boundary-1)){return Err(format!("缺少 Codex 早期历史分片（需要序号 {} 之前的记录），已停止迁移以避免导出残缺对话。",boundary))}
   out.retain(|r|r["ordinal"].as_u64().is_some_and(|n|n<boundary));
  }else if !out.is_empty(){out.clear();} // Duplicate legacy/root copies: keep one complete root.
  out.append(&mut rows);
 }
 Ok(out)
}
fn content(value:&Value)->String{crate::discovery::text_content(value)}
fn user_content(value:&Value)->String{
 match value{
  Value::String(s)=>if crate::metadata::is_injected(s){String::new()}else{s.clone()},
  Value::Array(items)=>items.iter().filter_map(|item|item.get("text").and_then(Value::as_str)).filter(|t|!t.trim().is_empty()&&!crate::metadata::is_injected(t)).collect::<Vec<_>>().join("\n"),
  _=>String::new(),
 }
}
fn push(out:&mut Vec<Message>,role:&str,text:String){if !text.trim().is_empty(){out.push(Message{role:role.into(),text});}}
pub fn messages(records:&[Value])->Vec<Message>{
 collect_messages(records,true)
}
pub fn dialogue(records:&[Value])->Vec<Message>{
 collect_messages(records,false)
}
fn collect_messages(records:&[Value],include_archives:bool)->Vec<Message>{
 let response_ids:HashSet<&str>=records.iter().filter(|r|r["type"]=="response_item"&&r["payload"]["type"]=="message").filter_map(|r|r["payload"]["id"].as_str()).collect();
 let mut out:Vec<Message>=Vec::new();let mut last_event_user=false;
 for r in records{
 let p=&r["payload"];
 if r["type"]=="response_item" {
  match p["type"].as_str().unwrap_or(""){
   "message"=>{let role=p["role"].as_str().unwrap_or("");if !["user","assistant"].contains(&role){continue}let text=if role=="user"{user_content(&p["content"])}else{content(&p["content"])};if role=="user"&&(text.trim().is_empty()||crate::metadata::is_injected(&text)){continue}
    if role=="user"&&last_event_user&&out.last().is_some_and(|m|m.role=="user"&&m.text.trim()==text.trim()){last_event_user=false;continue}push(&mut out,role,text);last_event_user=false;
   },
   "reasoning" if include_archives=>{let text=content(&p["summary"]);if !text.is_empty(){push(&mut out,"assistant",format!("### Reasoning summary\n\n{text}"));}},
   "function_call"|"custom_tool_call"|"function_call_output"|"custom_tool_call_output" if include_archives=>push(&mut out,"assistant",format!("### Historical tool event\n\n{}",crate::storage::fence(&serde_json::to_string_pretty(p).unwrap_or_default(),"json"))),
   _=>{},
  }
 }else if r["type"]=="event_msg"&&p["type"]=="item_completed" {
  let item=&p["item"];let text=content(&item["content"]);
  match item["type"].as_str().unwrap_or(""){
   "AgentMessage" if !item["id"].as_str().is_some_and(|id|response_ids.contains(id))=>push(&mut out,"assistant",text),
   "UserMessage"=>{if !out.last().is_some_and(|m|m.role=="user"&&m.text.trim()==text.trim())&&!crate::metadata::is_injected(&text){push(&mut out,"user",text);last_event_user=true;}},
   _=>{},
  }
 }else if r["type"]=="event_msg"&&p["type"]=="user_message" {
  let text=p["message"].as_str().unwrap_or("").to_string();
  if !text.trim().is_empty()&&!crate::metadata::is_injected(&text)&&!out.last().is_some_and(|m|m.role=="user"&&m.text.trim()==text.trim())&&!records.iter().any(|rec|rec["type"]=="response_item"&&rec["payload"]["role"]=="user"&&user_content(&rec["payload"]["content"]).trim()==text.trim()){
   push(&mut out,"user",text);last_event_user=true;
  }
 }else if r["type"]=="event_msg"&&p["type"]=="agent_message" {
  // Legacy event-only text has no response-item ID; skip its mirrored response.
  let text=p["message"].as_str().unwrap_or("");if !records.iter().any(|r|r["type"]=="response_item"&&r["payload"]["role"]=="assistant"&&content(&r["payload"]["content"])==text){push(&mut out,"assistant",text.into());}
 }
 }
 out
}

#[cfg(test)]mod tests{
 use super::*;use serde_json::json;
 #[test]fn dialogue_keeps_assistant_code_but_omits_tool_and_reasoning_archives(){
 let records=vec![json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"text":"QUESTION"}]}}),json!({"type":"response_item","payload":{"type":"function_call","name":"sleep","arguments":"{}"}}),json!({"type":"response_item","payload":{"type":"reasoning","summary":[{"text":"PRIVATE_ARCHIVE"}]}}),json!({"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"text":"Example:\n```json\n{\"name\":\"sleep\"}\n```"}]}})];
 let out=dialogue(&records);assert_eq!(out.len(),2);assert!(out[1].text.contains("```json"));assert!(!out.iter().any(|m|m.text.contains("Historical tool event")||m.text.contains("PRIVATE_ARCHIVE")));
 }
 #[test]fn historical_tool_json_is_literal_even_with_math_and_markdown_delimiters(){
 let input="text(await tools.exec_command({cmd:\"$taskNewBitmap.GetPixel(0,0); [pscustomobject]@{IconCornerR=$taskCorner.R}\"}));\n```markdown\n$$ x_{broken\n```";
 let payload=json!({"type":"custom_tool_call","name":"exec","call_id":"call-fixture","input":input});
 let output=messages(&[json!({"type":"response_item","payload":payload})]);assert_eq!(output.len(),1);
 let (_,block)=output[0].text.split_once("\n\n").unwrap();let mut lines=block.lines();let opening=lines.next().unwrap();assert!(opening.ends_with("json"),"tool JSON must be inside a code fence");let fence=opening.strip_suffix("json").unwrap();assert!(fence.len()>=4&&fence.chars().all(|c|c=='`'));
 let closing=block.rfind(&format!("\n{fence}")).unwrap();let body=&block[opening.len()+1..closing];assert_eq!(serde_json::from_str::<Value>(body).unwrap(),payload);
 }
 #[test]fn completion_mirrors_are_deduplicated_and_encrypted_reasoning_is_not_chat(){
 let records=vec![json!({"type":"response_item","payload":{"type":"message","id":"u1","role":"user","content":[{"text":"QUESTION"}]}}),json!({"type":"event_msg","payload":{"type":"item_completed","item":{"type":"UserMessage","id":"other-id","content":[{"text":"QUESTION"}]}}}),json!({"type":"event_msg","payload":{"type":"item_completed","item":{"type":"AgentMessage","id":"a1","content":[{"text":"ANSWER"}]}}}),json!({"type":"response_item","payload":{"type":"message","id":"a1","role":"assistant","content":[{"text":"ANSWER"}]}}),json!({"type":"response_item","payload":{"type":"reasoning","summary":[{"text":"PUBLIC_SUMMARY"}],"encrypted_content":"OPAQUE_CIPHERTEXT"}})];
 let out=messages(&records);assert_eq!(out.iter().filter(|m|m.role=="user").count(),1);assert_eq!(out.iter().filter(|m|m.text=="ANSWER").count(),1);assert!(out.iter().any(|m|m.text.contains("PUBLIC_SUMMARY")));assert!(!out.iter().any(|m|m.text.contains("OPAQUE_CIPHERTEXT")));
 }
 #[test]fn user_message_fallback_and_mixed_injected_blocks_are_preserved(){
  let records=vec![
   json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"<environment_context>\n  <cwd>E:/Repo</cwd>\n</environment_context>"},{"type":"input_text","text":"FIRST_MIXED_PROMPT"}]}}),
   json!({"type":"event_msg","payload":{"type":"user_message","message":"SECOND_EVENT_PROMPT"}}),
   json!({"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"text":"REPLY"}]}}),
  ];
  let out=dialogue(&records);
  assert_eq!(out.len(),3);
  assert_eq!(out[0].role,"user");
  assert_eq!(out[0].text,"FIRST_MIXED_PROMPT");
  assert_eq!(out[1].role,"user");
  assert_eq!(out[1].text,"SECOND_EVENT_PROMPT");
  assert_eq!(out[2].role,"assistant");
 }
 #[test]fn local_history_diagnostics(){
 let Ok(source)=std::env::var("CYANBIRD_CODEX_SAMPLE")else{return};let meta=header(Path::new(&source)).unwrap();let summary=SessionSummary{id:meta["payload"]["id"].as_str().unwrap().into(),title:"Diagnostic".into(),source:"codex".into(),workspace:None,updated_at:"0".into(),message_count:0,tool_call_count:0,status:"ready".into(),source_path:source};let records=read(&summary).unwrap();let output=messages(&records);
 let expected:Vec<_>=records.iter().filter(|r|r["type"]=="response_item"&&r["payload"]["type"]=="message"&&r["payload"]["role"]=="assistant").map(|r|content(&r["payload"]["content"])).filter(|s|!s.is_empty()).collect();assert!(expected.iter().all(|text|output.iter().any(|m|&m.text==text)));println!("Codex history: {} windows, {} canonical records, {} assistant messages retained, {} actual user messages, {} historical assistant/tool records",paths(&summary).len(),records.len(),expected.len(),output.iter().filter(|m|m.role=="user").count(),output.iter().filter(|m|m.role=="assistant").count());
 }
}
