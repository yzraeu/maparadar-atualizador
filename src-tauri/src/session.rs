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
        self.expires_at_unix > now_unix
    }
}

pub fn session_path(config_dir: &Path) -> PathBuf {
    config_dir.join("session.json")
}

pub fn load(config_dir: &Path) -> Result<Option<Session>, AppError> {
    let path = session_path(config_dir);
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path)?;
    let session = serde_json::from_str(&raw).map_err(|e| AppError::Session(e.to_string()))?;
    Ok(Some(session))
}

pub fn save(config_dir: &Path, session: &Session) -> Result<(), AppError> {
    std::fs::create_dir_all(config_dir)?;
    let raw = serde_json::to_string_pretty(session).map_err(|e| AppError::Session(e.to_string()))?;
    std::fs::write(session_path(config_dir), raw)?;
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
        assert!(!loaded.is_valid(1_001));
        clear(dir.path());
        assert!(load(dir.path()).unwrap().is_none());
    }

    #[test]
    fn load_missing_returns_none() {
        let dir = tempdir().unwrap();
        assert!(load(dir.path()).unwrap().is_none());
    }
}
