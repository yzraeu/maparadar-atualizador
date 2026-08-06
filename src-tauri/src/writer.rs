use std::path::{Path, PathBuf};

use crate::error::AppError;

#[derive(Debug, Clone, Default)]
pub struct WriteSummary {
    pub files_written: Vec<PathBuf>,
    pub files_deleted: Vec<PathBuf>,
}

pub fn write_igo8(target: &Path, data: &[u8]) -> Result<WriteSummary, AppError> {
    let mut summary = WriteSummary::default();

    std::fs::create_dir_all(target)?;
    let dest = target.join("speedcam.txt");
    let tmp = target.join("speedcam.txt.tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, &dest).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        AppError::from(e)
    })?;
    summary.files_written.push(dest);

    if target.is_dir() {
        for entry in std::fs::read_dir(target)? {
            let path = entry?.path();
            if !path.is_file() {
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let should_delete = ext.eq_ignore_ascii_case("spdb");
            if should_delete {
                if std::fs::remove_file(&path).is_ok() {
                    summary.files_deleted.push(path);
                }
            }
        }
    }

    Ok(summary)
}

pub fn write_ndrive(target: &Path, data: &[u8]) -> Result<WriteSummary, AppError> {
    let mut summary = WriteSummary::default();
    std::fs::create_dir_all(target)?;
    let dest = target.join("maparadar.kml");
    let tmp = target.join("maparadar.kml.tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, &dest).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        AppError::from(e)
    })?;
    summary.files_written.push(dest);
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn igo8_cleans_spdb_and_writes_speedcam_atomically() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("old.spdb"), b"x").unwrap();
        std::fs::write(dir.path().join("old2.SPDB"), b"x").unwrap();
        std::fs::write(dir.path().join("speedcam.txt"), b"old").unwrap();
        std::fs::write(dir.path().join("keep.log"), b"x").unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"x").unwrap();

        let s = write_igo8(dir.path(), b"NEW").unwrap();

        // two .spdb files deleted; speedcam.txt replaced, not counted as deleted
        assert_eq!(s.files_deleted.len(), 2);
        assert!(!dir.path().join("old.spdb").exists());
        assert!(!dir.path().join("old2.SPDB").exists());
        assert!(dir.path().join("keep.log").exists());
        assert!(dir.path().join("notes.txt").exists());
        assert_eq!(std::fs::read_to_string(dir.path().join("speedcam.txt")).unwrap(), "NEW");
        // no stray tmp file left behind
        assert!(!dir.path().join("speedcam.txt.tmp").exists());
    }

    #[test]
    fn igo8_writes_into_missing_folder() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("content/speedcam");
        let s = write_igo8(&target, b"DATA").unwrap();
        assert_eq!(s.files_written.len(), 1);
        assert_eq!(std::fs::read_to_string(target.join("speedcam.txt")).unwrap(), "DATA");
    }

    #[test]
    fn ndrive_writes_kml_to_folder() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("speedcams");
        let s = write_ndrive(&target, b"KML").unwrap();
        assert_eq!(s.files_written.len(), 1);
        assert_eq!(std::fs::read_to_string(target.join("maparadar.kml")).unwrap(), "KML");
    }
}
