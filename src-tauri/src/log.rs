use serde::Serialize;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub timestamp_unix_ms: i64,
    pub level: String,
    pub message: String,
}

pub struct LogStore {
    capacity: usize,
    entries: Mutex<VecDeque<LogEntry>>,
}

impl LogStore {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: Mutex::new(VecDeque::with_capacity(capacity.min(1024))),
        }
    }

    pub fn info(&self, message: impl Into<String>) {
        self.push("info", message.into());
    }

    pub fn warn(&self, message: impl Into<String>) {
        self.push("warn", message.into());
    }

    pub fn error(&self, message: impl Into<String>) {
        self.push("error", message.into());
    }

    pub fn list(&self) -> Vec<LogEntry> {
        let guard = self.entries.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.iter().cloned().collect()
    }

    fn push(&self, level: &str, message: String) {
        let mut guard = self.entries.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.len() >= self.capacity {
            guard.pop_front();
        }
        guard.push_back(LogEntry {
            timestamp_unix_ms: now_unix_ms(),
            level: level.to_string(),
            message,
        });
    }
}

fn now_unix_ms() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis() as i64,
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_latest_entries_with_capacity_limit() {
        let store = LogStore::new(3);

        store.info("a");
        store.info("b");
        store.info("c");
        store.info("d");

        let entries = store.list();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].message, "b");
        assert_eq!(entries[1].message, "c");
        assert_eq!(entries[2].message, "d");
    }

    #[test]
    fn stores_levels() {
        let store = LogStore::new(10);

        store.info("ok");
        store.warn("warn");
        store.error("err");

        let entries = store.list();
        assert_eq!(entries[0].level, "info");
        assert_eq!(entries[1].level, "warn");
        assert_eq!(entries[2].level, "error");
    }
}
