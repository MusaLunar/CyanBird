use serde_json::Value;
use std::{collections::HashMap, path::Path};

pub fn is_codex_child(meta: &Value) -> bool {
    meta["thread_source"] == "subagent" || meta["source"] == "subagent"
        || meta["source"].get("subagent").is_some_and(|v| !v.is_null() && v != false)
}
pub fn clean_prompt(raw: &str) -> String {
    let request = regex::Regex::new(r"(?is)<USER_REQUEST>(.*?)</USER_REQUEST>").unwrap();
    let text = request.captures(raw).map(|m|m[1].to_string()).unwrap_or_else(||raw.to_string());
    regex::Regex::new(r"(?is)<ADDITIONAL_METADATA>.*?</ADDITIONAL_METADATA>").unwrap().replace_all(&text, "").trim().to_string()
}
pub fn is_injected(text: &str) -> bool {
    let text = text.trim();
    ["# AGENTS.md", "<environment_context>", "<permissions instructions>", "<system", "<developer", "<external_codex_apps", "<INSTRUCTIONS>"].iter().any(|prefix|text.starts_with(prefix))
}
pub fn annotation_title(raw: &str) -> Option<String> {
    let pattern=regex::Regex::new(r#"title\s*:\s*("(?:[^"\\]|\\.)*")"#).unwrap();
    pattern.captures(raw).and_then(|m|serde_json::from_str::<String>(&m[1]).ok()).filter(|s|!s.trim().is_empty())
}
pub fn antigravity_id(path: &Path) -> Option<String> {
    let parts=path.to_str()?.replace('\\', "/");
    let parts:Vec<_>=parts.split('/').collect();
    let brain=parts.iter().rposition(|p|*p=="brain")?;
    parts.get(brain+1).filter(|p|!p.is_empty()).map(|s|s.to_string())
}
pub fn workspace_uri(uri: &str) -> String {
    let decoded=percent_encoding::percent_decode_str(uri).decode_utf8_lossy().to_string();
    if let Some(path)=decoded.strip_prefix("file:///") {
        if path.as_bytes().get(1)==Some(&b':') {path.to_string()} else {format!("/{path}")}
    } else if let Some(path)=decoded.strip_prefix("file://") {format!("//{path}")} else {decoded}
}
pub fn dominant_cwd(records: &[Value]) -> Option<String> {
    let mut counts: HashMap<String,usize>=HashMap::new();
    for record in records {
        if let Some(calls)=record["tool_calls"].as_array() {
            for call in calls {
                if let Some(raw)=call["args"]["Cwd"].as_str() {
                    let cwd=serde_json::from_str::<String>(raw).unwrap_or_else(|_|raw.to_string());
                    if cwd.starts_with('/') || cwd.starts_with("\\\\") || cwd.as_bytes().get(1)==Some(&b':') { *counts.entry(cwd).or_default()+=1; }
                }
            }
        }
    }
    counts.into_iter().max_by(|a,b|a.1.cmp(&b.1).then_with(||b.0.cmp(&a.0))).map(|x|x.0)
}
#[cfg(test)] mod tests {
 use super::*;
 #[test] fn antigravity_ids_are_unique_and_not_brain() {
  assert_eq!(antigravity_id(Path::new(r"C:\u\.gemini\antigravity\brain\one\.system_generated\logs\transcript.jsonl")).as_deref(),Some("one"));
  assert_eq!(antigravity_id(Path::new("C:/u/brain/two/.system_generated/logs/transcript.jsonl")).as_deref(),Some("two"));
 }
 #[test] fn filters_subagents_without_hiding_user_forks() {
  assert!(is_codex_child(&serde_json::json!({"source":{"subagent":{"thread_spawn":{"parent_thread_id":"p"}}}})));
  assert!(!is_codex_child(&serde_json::json!({"source":"cli","forked_from_id":"p","parent_thread_id":"p"})));
 }
 #[test] fn unwraps_prompts_and_decodes_project_uri() {
  assert_eq!(clean_prompt("<USER_REQUEST>build app</USER_REQUEST><ADDITIONAL_METADATA>hidden</ADDITIONAL_METADATA>"),"build app");
  assert_eq!(workspace_uri("file:///e%3A/Programming/My%20Project"),"e:/Programming/My Project");
  assert_eq!(annotation_title(r#"title: "Renamed session""#).as_deref(),Some("Renamed session"));
  assert!(is_injected("# AGENTS.md instructions for project"));
 }
}
