use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub username: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at_unix: i64,
}

impl Session {
    pub fn is_valid(&self, now_unix: i64) -> bool {
        !self.access_token.is_empty() && self.expires_at_unix > now_unix
    }
}

pub fn session_path(config_dir: &Path) -> PathBuf {
    config_dir.join("session.json")
}

pub fn load(config_dir: &Path) -> Result<Option<Session>, AppError> {
    let path = session_path(config_dir);
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    match serde_json::from_str(&raw) {
        Ok(session) => Ok(Some(session)),
        Err(_) => {
            let _ = std::fs::remove_file(&path);
            Ok(None)
        }
    }
}

pub fn save(config_dir: &Path, session: &Session) -> Result<(), AppError> {
    std::fs::create_dir_all(config_dir)?;
    let raw = serde_json::to_string_pretty(session).map_err(|e| AppError::Session(e.to_string()))?;
    let tmp = session_path(config_dir).with_extension("json.tmp");
    std::fs::write(&tmp, raw)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(&tmp, session_path(config_dir)).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        AppError::from(e)
    })?;
    Ok(())
}

pub fn clear(config_dir: &Path) {
    let _ = std::fs::remove_file(session_path(config_dir));
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_and_clear() {
        let dir = tempdir().unwrap();
        let s = Session {
            username: "izzy".into(),
            access_token: "jwt".into(),
            refresh_token: "r".into(),
            expires_at_unix: 1_000,
        };
        save(dir.path(), &s).unwrap();
        let loaded = load(dir.path()).unwrap().unwrap();
        assert_eq!(loaded.username, "izzy");
        assert!(loaded.is_valid(999));
        assert!(!loaded.is_valid(1_000));
        assert!(!loaded.is_valid(1_001));
        clear(dir.path());
        assert!(load(dir.path()).unwrap().is_none());
    }

    #[test]
    fn load_missing_returns_none() {
        let dir = tempdir().unwrap();
        assert!(load(dir.path()).unwrap().is_none());
    }

    #[test]
    fn is_valid_is_exclusive_boundary() {
        let dir = tempdir().unwrap();
        let s = Session {
            username: "izzy".into(),
            access_token: "jwt".into(),
            refresh_token: "r".into(),
            expires_at_unix: 1_000,
        };
        save(dir.path(), &s).unwrap();
        let loaded = load(dir.path()).unwrap().unwrap();
        assert!(!loaded.is_valid(1_000));
    }

    #[test]
    fn load_corrupt_file_returns_none() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("session.json"), "{ not json").unwrap();
        assert!(load(dir.path()).unwrap().is_none());
        assert!(!session_path(dir.path()).exists());
    }
}
