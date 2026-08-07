use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tauri::{Emitter, State};

use crate::alert_types::{self, AlertType};
use crate::api::{now_unix, AuthResponse};
use crate::device::{self, DeviceKind};
use crate::error::AppError;
use crate::log::LogEntry;
use crate::session;
use crate::writer::{self, WriteSummary};
use crate::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub username: String,
    pub expires_at_unix: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub kind: String,
    pub display: String,
    pub drive: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSummary {
    pub files_written: Vec<String>,
    pub files_deleted: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub arch: String,
    pub tauri_version: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    pub stage: String,
    pub percent: u32,
}

fn persist_session(state: &AppState, username: String, auth: &AuthResponse) -> Result<(), AppError> {
    let s = session::Session {
        username: username.clone(),
        access_token: auth.access_token.clone(),
        refresh_token: auth.refresh_token.clone(),
        expires_at_unix: now_unix() + auth.expires_in,
    };
    session::save(&state.config_dir, &s)
}

#[tauri::command]
pub fn get_alert_types() -> Vec<AlertType> {
    alert_types::ALERT_TYPES.to_vec()
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        name: "Atualizador MapaRadar".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        tauri_version: tauri::VERSION.to_string(),
    }
}

#[tauri::command]
pub fn get_logs(state: State<'_, AppState>) -> Vec<LogEntry> {
    state.logs.list()
}

#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    username: String,
    password: String,
) -> Result<SessionInfo, AppError> {
    let start = Instant::now();
    state.logs.info(format!("Tentativa de login para usuário {}", username));

    let auth = match state.api.login(&username, &password).await {
        Ok(auth) => auth,
        Err(e) => {
            state.logs.error(format!(
                "Falha no login para usuário {} após {} ms: {}",
                username,
                start.elapsed().as_millis(),
                e
            ));
            return Err(e);
        }
    };

    let now = now_unix();
    if let Err(e) = persist_session(&state, username.clone(), &auth) {
        state.logs.error(format!("Falha ao persistir sessão para {}: {}", username, e));
        return Err(e);
    }

    state.logs.info(format!(
        "Login concluído para usuário {} em {} ms",
        username,
        start.elapsed().as_millis()
    ));
    Ok(SessionInfo {
        username,
        expires_at_unix: now + auth.expires_in,
    })
}

#[tauri::command]
pub fn logout(state: State<'_, AppState>) {
    if let Ok(Some(s)) = session::load(&state.config_dir) {
        state.logs.info(format!("Logout solicitado para usuário {}", s.username));
    } else {
        state.logs.info("Logout solicitado".to_string());
    }
    session::clear(&state.config_dir);
}

#[tauri::command]
pub async fn session_status(
    state: State<'_, AppState>,
) -> Result<Option<SessionInfo>, AppError> {
    let Some(s) = session::load(&state.config_dir).ok().flatten() else {
        state.logs.warn("Sessão inexistente ao verificar status".to_string());
        return Ok(None);
    };
    if s.is_valid(now_unix()) {
        state.logs.info(format!("Sessão ativa para usuário {}", s.username));
        return Ok(Some(SessionInfo {
            username: s.username,
            expires_at_unix: s.expires_at_unix,
        }));
    }
    // Token expired: try transparent refresh before forcing re-login.
    match state.api.refresh(&s.refresh_token).await {
        Ok(auth) => {
            let username = s.username.clone();
            let now_after_refresh = now_unix();
            if persist_session(&state, username.clone(), &auth).is_ok() {
                state.logs.info(format!("Sessão renovada para usuário {}", username));
                Ok(Some(SessionInfo {
                    username,
                    expires_at_unix: now_after_refresh + auth.expires_in,
                }))
            } else {
                state.logs.error(format!("Falha ao salvar sessão renovada para usuário {}", username));
                Ok(None)
            }
        }
        Err(e) => {
            if matches!(e, AppError::Unauthorized) {
                session::clear(&state.config_dir);
                state.logs.warn("Refresh inválido; sessão limpa".to_string());
            } else {
                state.logs.error(format!("Erro ao renovar sessão: {}", e));
            }
            Ok(None)
        }
    }
}

#[tauri::command]
pub async fn detect_device() -> Vec<DeviceInfo> {
    tauri::async_runtime::spawn_blocking(device::detect)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|d| DeviceInfo {
            kind: d.kind.export_type().to_string(),
            display: d.kind.display().to_string(),
            drive: d.drive.to_string_lossy().to_string(),
        })
        .collect()
}

#[tauri::command]
pub async fn preview_count(
    state: State<'_, AppState>,
    radar_types: String,
) -> Result<u32, AppError> {
    let start = Instant::now();
    let result = state.api.preview_count(&radar_types).await;
    match &result {
        Ok(count) => state.logs.info(format!(
            "Preview concluído: {} pontos para tipos [{}] em {} ms",
            count,
            radar_types,
            start.elapsed().as_millis()
        )),
        Err(e) => state.logs.error(format!(
            "Falha no preview para tipos [{}] após {} ms: {}",
            radar_types,
            start.elapsed().as_millis(),
            e
        )),
    }
    result
}

async fn ensure_access_token(state: &AppState) -> Result<String, AppError> {
    let s = session::load(&state.config_dir)?.ok_or(AppError::Unauthorized)?;
    if s.is_valid(now_unix()) {
        return Ok(s.access_token);
    }
    let auth = state.api.refresh(&s.refresh_token).await?;
    let username = s.username.clone();
    persist_session(state, username, &auth)?;
    Ok(auth.access_token)
}

fn parse_kind(kind: &str) -> Result<DeviceKind, AppError> {
    match kind {
        "igo8" => Ok(DeviceKind::Igo8),
        "ndrive" => Ok(DeviceKind::NDrive),
        _ => Err(AppError::Api("Tipo de dispositivo inválido.".into())),
    }
}

fn aggregate_writes<F, P>(folders: &[PathBuf], data: &[u8], write_fn: F, on_progress: P) -> Result<WriteSummary, AppError>
where
    F: Fn(&Path, &[u8]) -> Result<WriteSummary, AppError>,
    P: Fn(u32),
{
    let total = folders.len() as u32;
    let mut acc = WriteSummary::default();
    let mut failed = Vec::new();
    for (i, folder) in folders.iter().enumerate() {
        match write_fn(folder, data) {
            Ok(s) => {
                acc.files_written.extend(s.files_written);
                acc.files_deleted.extend(s.files_deleted);
            }
            Err(e) => failed.push((folder.clone(), e)),
        }
        if total > 0 {
            on_progress(40 + ((i as u32 + 1) * 60 / total));
        }
    }
    if !failed.is_empty() {
        let details: Vec<String> = failed
            .iter()
            .map(|(p, _)| p.to_string_lossy().to_string())
            .collect();
        let ok_count = folders.len() - failed.len();
        return Err(AppError::Io(format!(
            "{ok_count} de {} pastas atualizadas; falha em: {}",
            folders.len(),
            details.join(", ")
        )));
    }
    Ok(acc)
}

#[tauri::command]
pub async fn update_device(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    kind: String,
    radar_types: String,
) -> Result<UpdateSummary, AppError> {
    let start = Instant::now();
    let kind_enum = parse_kind(&kind)?;

    let _ = app_handle.emit("update-progress", UpdateProgress {
        stage: "download".into(),
        percent: 5,
    });

    state.logs.info(format!(
        "Iniciando exportação para {} com tipos [{}]",
        kind, radar_types
    ));

    let token = match ensure_access_token(&state).await {
        Ok(token) => token,
        Err(e) => {
            state.logs.error(format!("Falha ao obter token para exportação: {}", e));
            return Err(e);
        }
    };

    let _ = app_handle.emit("update-progress", UpdateProgress {
        stage: "download".into(),
        percent: 15,
    });

    let mut folders = Vec::new();
    for d in device::detect() {
        if d.kind == kind_enum {
            folders.extend(d.folders);
        }
    }
    if folders.is_empty() {
        state.logs.warn(format!("Exportação cancelada: destino {} não encontrado", kind));
        return Err(AppError::DeviceNotFound);
    }

    let destination_list = folders
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    state
        .logs
        .info(format!("Destino(s) da exportação: {}", destination_list));

    let export_type = kind_enum.export_type();
    let bytes = match state.api.export_updater(&token, export_type, &radar_types).await {
        Ok(bytes) => bytes,
        Err(e) => {
            state.logs.error(format!("Falha ao baixar exportação: {}", e));
            return Err(e);
        }
    };

    let _ = app_handle.emit("update-progress", UpdateProgress {
        stage: "write".into(),
        percent: 40,
    });

    let ah = app_handle.clone();
    let summary = match kind_enum {
        DeviceKind::Igo8 => aggregate_writes(&folders, &bytes, writer::write_igo8, move |p| {
            let _ = ah.emit("update-progress", UpdateProgress { stage: "write".into(), percent: p });
        }),
        DeviceKind::NDrive => aggregate_writes(&folders, &bytes, writer::write_ndrive, move |p| {
            let _ = ah.emit("update-progress", UpdateProgress { stage: "write".into(), percent: p });
        }),
    };

    let summary = match summary {
        Ok(summary) => summary,
        Err(e) => {
            state.logs.error(format!("Falha ao gravar exportação no destino: {}", e));
            return Err(e);
        }
    };

    state.logs.info(format!(
        "Exportação concluída para {} em {} ms ({} arquivos gravados, {} removidos)",
        kind,
        start.elapsed().as_millis(),
        summary.files_written.len(),
        summary.files_deleted.len()
    ));

    Ok(UpdateSummary {
        files_written: summary.files_written.iter().map(|p| p.to_string_lossy().to_string()).collect(),
        files_deleted: summary.files_deleted.iter().map(|p| p.to_string_lossy().to_string()).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parse_kind_igo8() {
        assert!(matches!(parse_kind("igo8").unwrap(), DeviceKind::Igo8));
    }

    #[test]
    fn parse_kind_ndrive() {
        assert!(matches!(parse_kind("ndrive").unwrap(), DeviceKind::NDrive));
    }

    #[test]
    fn parse_kind_invalid() {
        let err = parse_kind("garmin").unwrap_err();
        assert!(matches!(err, AppError::Api(_)));
        assert!(err.to_string().contains("inválido"));
    }

    #[test]
    fn aggregate_writes_all_success() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        let folders = vec![a.clone(), b.clone()];
        let s = aggregate_writes(&folders, b"DATA", |target, data| {
            std::fs::create_dir_all(target).unwrap();
            std::fs::write(target.join("out.txt"), data).unwrap();
            Ok(WriteSummary {
                files_written: vec![target.join("out.txt")],
                files_deleted: vec![],
            })
        }, |_| {})
        .unwrap();
        assert_eq!(s.files_written.len(), 2);
    }

    #[test]
    fn aggregate_writes_partial_failure() {
        let dir = tempdir().unwrap();
        let ok = dir.path().join("ok");
        let fail_dir = dir.path().join("fail");
        let folders = vec![ok.clone(), fail_dir.clone()];
        let err = aggregate_writes(&folders, b"DATA", |target, _data| {
            if target.ends_with("fail") {
                Err(AppError::Io("disconnected".into()))
            } else {
                Ok(WriteSummary {
                    files_written: vec![target.join("out.txt")],
                    files_deleted: vec![],
                })
            }
        }, |_| {})
        .unwrap_err();
        assert!(matches!(err, AppError::Io(_)));
        assert!(err.to_string().contains("1 de 2"));
        assert!(err.to_string().contains("fail"));
    }

    #[test]
    fn aggregate_writes_all_fail() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        let folders = vec![a, b];
        let err = aggregate_writes(&folders, b"DATA", |_, _| {
            Err(AppError::Io("dead".into()))
        }, |_| {})
        .unwrap_err();
        assert!(matches!(err, AppError::Io(_)));
        assert!(err.to_string().contains("0 de 2"));
    }
}
