pub mod ahledger;
pub mod blizzard;
pub mod collector;
pub mod commands;
pub mod credentials;
pub mod db;
pub mod domain;
pub mod processing;
pub mod queries;
pub mod settings;
pub mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
