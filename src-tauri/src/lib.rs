mod alert_types;
mod api;
mod commands;
mod device;
mod error;
mod session;
mod writer;

use std::path::PathBuf;

pub struct AppState {
    api: api::ApiClient,
    config_dir: PathBuf,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("maparadar-atualizador");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            api: api::ApiClient::new("https://api.maparadar.com"),
            config_dir,
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_alert_types,
            commands::login,
            commands::logout,
            commands::session_status,
            commands::detect_device,
            commands::preview_count,
            commands::update_device,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
