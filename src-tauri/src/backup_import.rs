use crate::{discovery::SessionSummary,storage::BackupRecord};
use std::{collections::HashSet,fs,path::{Path,PathBuf}};

pub fn index_only(record:&BackupRecord)->bool{
 record.sessions.iter().any(|s|s.source!=record.provider||Path::new(&s.source_path).file_name().is_some_and(|n|n=="conversation_summaries.db"||n=="agyhub_summaries_proto.pb"))
}
pub fn load(root:&Path,id:&str)->Result<BackupRecord,String>{
 if !id.starts_with("backup-")||!id.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-'){return Err("Invalid backup ID".into())}
 let canonical_root=fs::canonicalize(root).map_err(|e|e.to_string())?;
 let directory=fs::canonicalize(canonical_root.join(id)).map_err(|e|e.to_string())?;
 if !directory.starts_with(&canonical_root){return Err("Backup directory escapes the configured backup root".into())}
 let manifest=directory.join("manifest.json");let manifest=fs::canonicalize(manifest).map_err(|e|e.to_string())?;
 if !manifest.starts_with(&directory){return Err("Backup manifest escapes its directory".into())}
 let mut record:BackupRecord=serde_json::from_slice(&fs::read(manifest).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
 if record.id!=id{return Err("Backup ID does not match its manifest".into())}
 record.directory=directory.to_string_lossy().into();
 if record.kind=="target-index"||index_only(&record){return Err("这是目标索引快照，不是可导入的会话备份。".into())}
 Ok(record)
}
fn stored_path(directory:&Path,stored:&str)->Result<PathBuf,String>{
 let path=Path::new(stored);if stored.contains(['/', '\\', ':'])||path.components().count()!=1||!matches!(path.components().next(),Some(std::path::Component::Normal(_))){return Err("Invalid stored backup filename".into())}
 let path=fs::canonicalize(directory.join(path)).map_err(|e|e.to_string())?;
 if !path.starts_with(directory){return Err("Backup file escapes its directory".into())}Ok(path)
}
pub fn sessions(record:&BackupRecord,ids:&[String])->Result<Vec<SessionSummary>,String>{
 let ids:HashSet<_>=ids.iter().collect();if ids.is_empty(){return Err("Select at least one backed-up session".into())}
 let selected:Vec<_>=record.sessions.iter().filter(|s|ids.contains(&s.id)).collect();if selected.len()!=ids.len(){return Err("Selected session is not in this backup".into())}
 let directory=fs::canonicalize(&record.directory).map_err(|e|e.to_string())?;
 // Verify all files, including previous Codex rollout windows needed by a session.
 for file in &record.files{let path=stored_path(&directory,&file.stored)?;
 if fs::metadata(&path).map_err(|e|e.to_string())?.len()!=file.bytes||crate::storage::hash_file(&path)?!=file.sha256{return Err(format!("备份文件校验失败，已停止导入：{}",file.stored))}}
 selected.into_iter().map(|source|{let file=record.files.iter().find(|f|f.original==source.source_path).ok_or("Missing session snapshot in backup")?;let mut source=source.clone();source.source_path=stored_path(&directory,&file.stored)?.to_string_lossy().into();Ok(source)}).collect()
}

#[cfg(test)]mod tests{
 use super::*;use crate::storage::{backup_to,read_export_records};
 #[test]fn imports_deleted_original_from_verified_snapshot_and_rejects_tampering(){
 let root=std::env::temp_dir().join(format!("cyanbird-backup-import-{}",uuid::Uuid::new_v4()));fs::create_dir_all(&root).unwrap();let source=root.join("source.jsonl");
 fs::write(&source,"{\"type\":\"user\",\"message\":{\"content\":\"BACKUP_QUESTION\"}}\n{\"type\":\"assistant\",\"message\":{\"content\":\"BACKUP_ANSWER\"}}\n").unwrap();
 let summary=SessionSummary{id:"fixture".into(),title:"Named backup session".into(),source:"claude-code".into(),workspace:None,updated_at:"0".into(),message_count:2,tool_call_count:0,status:"ready".into(),source_path:source.to_string_lossy().into()};
 let backup=backup_to(&root.join("backups"),"claude-code",vec![summary]).unwrap();fs::remove_file(source).unwrap();let loaded=load(&root.join("backups"),&backup.id).unwrap();let selected=sessions(&loaded,&["fixture".into()]).unwrap();assert_eq!(selected[0].title,"Named backup session");let records=read_export_records(&selected[0]).unwrap();assert_eq!(records.len(),2);
 let manifest_before=fs::read(Path::new(&backup.directory).join("manifest.json")).unwrap();
 for target in ["claude-code","codex","dsh"]{let messages=crate::migration::for_target(&selected[0],&records,target);let written=crate::migration::write_target(&root.join("target").join(target),target,&uuid::Uuid::new_v4().to_string(),&selected[0].title,"C:/Project",&messages).unwrap();let output=fs::read_to_string(written).unwrap();assert!(output.contains("BACKUP_QUESTION")&&output.contains("BACKUP_ANSWER"));}assert_eq!(fs::read(Path::new(&backup.directory).join("manifest.json")).unwrap(),manifest_before);
 let index=crate::storage::backup_index_to(&root.join("backups"),"claude-code",vec![selected[0].clone()]).unwrap();assert!(load(&root.join("backups"),&index.id).unwrap_err().contains("目标索引"));
 assert!(load(&root.join("backups"),"../outside").is_err());assert!(sessions(&loaded,&["not-in-backup".into()]).is_err());
 let mut escaped=loaded.clone();escaped.files[0].stored="../outside".into();assert!(sessions(&escaped,&["fixture".into()]).is_err());
 fs::write(Path::new(&loaded.directory).join(&loaded.files[0].stored),"tampered").unwrap();assert!(sessions(&loaded,&["fixture".into()]).unwrap_err().contains("校验失败"));fs::remove_dir_all(root).unwrap();
 }
}
