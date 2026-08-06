use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Igo8,
    NDrive,
}

impl DeviceKind {
    pub fn export_type(self) -> &'static str {
        match self {
            DeviceKind::Igo8 => "igo8",
            DeviceKind::NDrive => "ndrive",
        }
    }

    pub fn display(self) -> &'static str {
        match self {
            DeviceKind::Igo8 => "iGO",
            DeviceKind::NDrive => "NDrive",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DetectedDevice {
    pub kind: DeviceKind,
    pub drive: PathBuf,
    pub folders: Vec<PathBuf>,
}

const MAX_DEPTH: usize = 6;

pub fn removable_mount_points() -> Vec<PathBuf> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    disks
        .iter()
        .filter(|d| d.is_removable())
        .map(|d| d.mount_point().to_path_buf())
        .collect()
}

pub fn detect() -> Vec<DetectedDevice> {
    removable_mount_points()
        .iter()
        .filter_map(|m| detect_in_drive(m))
        .collect()
}

fn normalize(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/").to_lowercase()
}

pub fn detect_in_drive(drive: &Path) -> Option<DetectedDevice> {
    let mut igo8 = Vec::new();
    let mut ndrive = Vec::new();

    scan(drive, 0, &mut |dir| {
        let n = normalize(dir);
        if n.ends_with("/content/speedcam") {
            igo8.push(dir.to_path_buf());
        }
        if n.ends_with("/speedcams") {
            ndrive.push(dir.to_path_buf());
        }
    });

    if igo8.is_empty() && ndrive.is_empty() {
        return None;
    }
    let (kind, folders) = if !igo8.is_empty() {
        (DeviceKind::Igo8, igo8)
    } else {
        (DeviceKind::NDrive, ndrive)
    };
    Some(DetectedDevice {
        kind,
        drive: drive.to_path_buf(),
        folders,
    })
}

fn scan<F: FnMut(&Path)>(dir: &Path, depth: usize, on_dir: &mut F) {
    if depth > MAX_DEPTH {
        return;
    }
    on_dir(dir);
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if name.starts_with('.')
            || name == "$recycle.bin"
            || name == "system volume information"
        {
            continue;
        }
        scan(&path, depth + 1, on_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn detects_igo8() {
        let dir = tempdir().unwrap();
        let cam = dir.path().join("content/speedcam");
        std::fs::create_dir_all(&cam).unwrap();
        let d = detect_in_drive(dir.path()).unwrap();
        assert_eq!(d.kind, DeviceKind::Igo8);
        assert_eq!(d.folders, vec![cam]);
    }

    #[test]
    fn detects_ndrive_case_insensitive() {
        let dir = tempdir().unwrap();
        let cam = dir.path().join("NDrive/SpeedCams");
        std::fs::create_dir_all(&cam).unwrap();
        let d = detect_in_drive(dir.path()).unwrap();
        assert_eq!(d.kind, DeviceKind::NDrive);
        assert_eq!(d.folders, vec![cam]);
    }

    #[test]
    fn detects_both_but_igo8_takes_precedence() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("content/speedcam")).unwrap();
        std::fs::create_dir_all(dir.path().join("speedcams")).unwrap();
        let d = detect_in_drive(dir.path()).unwrap();
        assert_eq!(d.kind, DeviceKind::Igo8);
    }

    #[test]
    fn no_match_returns_none() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("some/folder")).unwrap();
        assert!(detect_in_drive(dir.path()).is_none());
    }

    #[test]
    fn skips_hidden_directories() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".hidden/speedcams")).unwrap();
        assert!(detect_in_drive(dir.path()).is_none());
    }
}
