#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod discovery;
mod metadata;
mod storage;
mod migration;
mod antigravity;
mod codex;
mod backup_import;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![discovery::discover_sources, discovery::discover_sessions, storage::get_preferences, storage::save_preferences, storage::create_backup, storage::list_backups, storage::export_markdown, storage::open_output_directory, storage::preview_session, migration::preview_migration, migration::execute_migration, migration::preview_backup_import, migration::import_backup])
        .run(tauri::generate_context!())
        .expect("error while running CyanBird");
}
