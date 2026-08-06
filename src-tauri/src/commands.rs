use serde::Serialize;
use tauri::State;

use crate::alert_types::{self, AlertType};
use crate::api::{now_unix, AuthResponse};
use crate::device::{self, DeviceKind};
use crate::error::AppError;
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
pub async fn login(
    state: State<'_, AppState>,
    username: String,
    password: String,
) -> Result<SessionInfo, AppError> {
    let auth = state.api.login(&username, &password).await?;
    persist_session(&state, username.clone(), &auth)?;
    Ok(SessionInfo {
        username,
        expires_at_unix: now_unix() + auth.expires_in,
    })
}

#[tauri::command]
pub fn logout(state: State<'_, AppState>) {
    session::clear(&state.config_dir);
}

#[tauri::command]
pub async fn session_status(
    state: State<'_, AppState>,
) -> Result<Option<SessionInfo>, AppError> {
    let Some(s) = session::load(&state.config_dir).ok().flatten() else {
        return Ok(None);
    };
    if s.is_valid(now_unix()) {
        return Ok(Some(SessionInfo {
            username: s.username,
            expires_at_unix: s.expires_at_unix,
        }));
    }
    // Token expired: try transparent refresh before forcing re-login.
    match state.api.refresh(&s.refresh_token).await {
        Ok(auth) => {
            let username = s.username.clone();
            if persist_session(&state, username.clone(), &auth).is_ok() {
                Ok(Some(SessionInfo {
                    username,
                    expires_at_unix: now_unix() + auth.expires_in,
                }))
            } else {
                Ok(None)
            }
        }
        Err(e) => {
            if matches!(e, AppError::Unauthorized) {
                session::clear(&state.config_dir);
            }
            Ok(None)
        }
    }
}

#[tauri::command]
pub fn detect_device() -> Vec<DeviceInfo> {
    device::detect()
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
    state.api.preview_count(&radar_types).await
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

#[tauri::command]
pub async fn update_device(
    state: State<'_, AppState>,
    kind: String,
    radar_types: String,
) -> Result<UpdateSummary, AppError> {
    let kind_enum = match kind.as_str() {
        "igo8" => DeviceKind::Igo8,
        "ndrive" => DeviceKind::NDrive,
        _ => return Err(AppError::Api("Tipo de dispositivo inválido.".into())),
    };

    let token = ensure_access_token(&state).await?;

    let mut folders = Vec::new();
    for d in device::detect() {
        if d.kind == kind_enum {
            folders.extend(d.folders);
        }
    }
    if folders.is_empty() {
        return Err(AppError::DeviceNotFound);
    }

    let bytes = state.api.export_updater(&token, &kind, &radar_types).await?;

    let summary = match kind_enum {
        DeviceKind::Igo8 => {
            let mut acc = WriteSummary::default();
            let mut failed = Vec::new();
            for folder in &folders {
                match writer::write_igo8(folder, &bytes) {
                    Ok(s) => {
                        acc.files_written.extend(s.files_written);
                        acc.files_deleted.extend(s.files_deleted);
                    }
                    Err(e) => failed.push((folder.clone(), e)),
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
            acc
        }
        DeviceKind::NDrive => writer::write_ndrive(&folders, &bytes)?,
    };

    Ok(UpdateSummary {
        files_written: summary.files_written.iter().map(|p| p.to_string_lossy().to_string()).collect(),
        files_deleted: summary.files_deleted.iter().map(|p| p.to_string_lossy().to_string()).collect(),
    })
}
