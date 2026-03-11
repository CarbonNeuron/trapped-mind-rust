use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Ai,
    User,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub role: Role,
    pub text: String,
    pub timestamp: String,
}

impl HistoryEntry {
    pub fn new(role: Role, text: String) -> Self {
        Self { role, text, timestamp: Utc::now().to_rfc3339() }
    }
}

pub struct HistoryManager {
    path: PathBuf,
    max_entries: usize,
    entries: Vec<HistoryEntry>,
}

impl HistoryManager {
    pub fn new(path: PathBuf, max_entries: usize) -> Self {
        let entries = Self::load_from_file(&path, max_entries);
        Self { path, max_entries, entries }
    }

    fn load_from_file(path: &Path, max_entries: usize) -> Vec<HistoryEntry> {
        let file = match fs::File::open(path) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };
        let reader = BufReader::new(file);
        let mut entries: Vec<HistoryEntry> = reader
            .lines()
            .filter_map(|line| line.ok())
            .filter_map(|line| serde_json::from_str(&line).ok())
            .collect();
        if entries.len() > max_entries {
            entries = entries.split_off(entries.len() - max_entries);
        }
        entries
    }

    pub fn append(&mut self, entry: HistoryEntry) {
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
        self.save();
    }

    pub fn last_n(&self, n: usize) -> &[HistoryEntry] {
        let start = self.entries.len().saturating_sub(n);
        &self.entries[start..]
    }

    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        let _ = fs::remove_file(&self.path);
    }

    fn save(&self) {
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut file = match fs::File::create(&self.path) {
            Ok(f) => f,
            Err(_) => return,
        };
        for entry in &self.entries {
            if let Ok(json) = serde_json::to_string(entry) {
                let _ = writeln!(file, "{}", json);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_path() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join("trapped-mind-test");
        fs::create_dir_all(&dir).unwrap();
        dir.join(format!(
            "history_{}_{}.jsonl",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ))
    }

    #[test]
    fn test_new_empty() {
        let path = temp_path();
        let _ = fs::remove_file(&path);
        let mgr = HistoryManager::new(path.clone(), 50);
        assert!(mgr.entries().is_empty());
    }

    #[test]
    fn test_append_and_read() {
        let path = temp_path();
        let _ = fs::remove_file(&path);
        let mut mgr = HistoryManager::new(path.clone(), 50);
        mgr.append(HistoryEntry::new(Role::User, "hello".to_string()));
        mgr.append(HistoryEntry::new(Role::Ai, "hi there".to_string()));
        assert_eq!(mgr.entries().len(), 2);
        assert_eq!(mgr.entries()[0].text, "hello");
        assert_eq!(mgr.entries()[1].text, "hi there");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_last_n() {
        let path = temp_path();
        let _ = fs::remove_file(&path);
        let mut mgr = HistoryManager::new(path.clone(), 50);
        for i in 0..10 {
            mgr.append(HistoryEntry::new(Role::Ai, format!("thought {}", i)));
        }
        let last3 = mgr.last_n(3);
        assert_eq!(last3.len(), 3);
        assert_eq!(last3[0].text, "thought 7");
        assert_eq!(last3[2].text, "thought 9");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_max_entries_trim() {
        let path = temp_path();
        let _ = fs::remove_file(&path);
        let mut mgr = HistoryManager::new(path.clone(), 5);
        for i in 0..10 {
            mgr.append(HistoryEntry::new(Role::Ai, format!("thought {}", i)));
        }
        assert_eq!(mgr.entries().len(), 5);
        assert_eq!(mgr.entries()[0].text, "thought 5");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_persistence() {
        let path = temp_path();
        let _ = fs::remove_file(&path);
        {
            let mut mgr = HistoryManager::new(path.clone(), 50);
            mgr.append(HistoryEntry::new(Role::User, "persisted".to_string()));
        }
        let mgr = HistoryManager::new(path.clone(), 50);
        assert_eq!(mgr.entries().len(), 1);
        assert_eq!(mgr.entries()[0].text, "persisted");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_clear() {
        let path = temp_path();
        let _ = fs::remove_file(&path);
        let mut mgr = HistoryManager::new(path.clone(), 50);
        mgr.append(HistoryEntry::new(Role::Ai, "gone".to_string()));
        mgr.clear();
        assert!(mgr.entries().is_empty());
        assert!(!path.exists());
    }

    #[test]
    fn test_jsonl_format() {
        let path = temp_path();
        let _ = fs::remove_file(&path);
        let mut mgr = HistoryManager::new(path.clone(), 50);
        mgr.append(HistoryEntry::new(Role::User, "test line".to_string()));
        drop(mgr);
        let content = fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(parsed["role"], "user");
        assert_eq!(parsed["text"], "test line");
        assert!(parsed["timestamp"].is_string());
        let _ = fs::remove_file(&path);
    }
}
