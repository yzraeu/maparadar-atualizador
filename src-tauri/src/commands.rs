use serde::Serialize;
use std::path::{Path, PathBuf};
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
    let now = now_unix();
    persist_session(&state, username.clone(), &auth)?;
    Ok(SessionInfo {
        username,
        expires_at_unix: now + auth.expires_in,
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
            let now_after_refresh = now_unix();
            if persist_session(&state, username.clone(), &auth).is_ok() {
                Ok(Some(SessionInfo {
                    username,
                    expires_at_unix: now_after_refresh + auth.expires_in,
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

fn parse_kind(kind: &str) -> Result<DeviceKind, AppError> {
    match kind {
        "igo8" => Ok(DeviceKind::Igo8),
        "ndrive" => Ok(DeviceKind::NDrive),
        _ => Err(AppError::Api("Tipo de dispositivo inválido.".into())),
    }
}

fn aggregate_writes<F>(folders: &[PathBuf], data: &[u8], write_fn: F) -> Result<WriteSummary, AppError>
where
    F: Fn(&Path, &[u8]) -> Result<WriteSummary, AppError>,
{
    let mut acc = WriteSummary::default();
    let mut failed = Vec::new();
    for folder in folders {
        match write_fn(folder, data) {
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
    Ok(acc)
}

#[tauri::command]
pub async fn update_device(
    state: State<'_, AppState>,
    kind: String,
    radar_types: String,
) -> Result<UpdateSummary, AppError> {
    let kind_enum = parse_kind(&kind)?;

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

    let export_type = kind_enum.export_type();
    let bytes = state.api.export_updater(&token, export_type, &radar_types).await?;

    let summary = match kind_enum {
        DeviceKind::Igo8 => aggregate_writes(&folders, &bytes, writer::write_igo8)?,
        DeviceKind::NDrive => aggregate_writes(&folders, &bytes, writer::write_ndrive)?,
    };

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
        })
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
        })
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
        })
        .unwrap_err();
        assert!(matches!(err, AppError::Io(_)));
        assert!(err.to_string().contains("0 de 2"));
    }
}
