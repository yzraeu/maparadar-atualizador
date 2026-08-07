mod alert_types;
mod api;
mod commands;
mod device;
mod error;
mod log;
mod session;
mod writer;

use std::path::PathBuf;
use tauri::Manager;

pub struct AppState {
    api: api::ApiClient,
    config_dir: PathBuf,
    logs: log::LogStore,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("maparadar-atualizador");

    let logs = log::LogStore::new(1000);
    logs.info(format!(
        "App iniciado v{} em {} {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    ));

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
            logs,
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_alert_types,
            commands::get_app_info,
            commands::get_logs,
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
