mod alert_types;
mod api;
mod commands;
mod device;
mod error;
mod session;
mod writer;

use std::path::PathBuf;
use tauri::Manager;

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
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
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
