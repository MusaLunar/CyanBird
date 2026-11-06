use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation { pub id: String, pub provider: String, pub label: String, pub path: String, pub session_count: u32, pub status: String }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotReceipt { pub snapshot_path: String, pub manifest_path: String, pub bytes: u64 }
#[cfg(test)] mod tests { use super::*; #[test] fn serializes_windows_paths(){ let s=SourceLocation{id:"x".into(),provider:"codex".into(),label:"Codex".into(),path:"C:/Users/A User/.codex".into(),session_count:2,status:"ready".into()}; let json=serde_json::to_string(&s).unwrap(); assert!(json.contains("A User")); } }
