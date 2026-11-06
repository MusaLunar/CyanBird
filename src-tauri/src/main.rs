#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod discovery;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![discovery::discover_sources, discovery::discover_sessions])
        .run(tauri::generate_context!())
        .expect("error while running CyanBird");
}
