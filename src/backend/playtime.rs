use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlaySession {
    pub instance_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_seconds: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PlaytimeManager {
    /// Map of instance ID to total playtime in seconds.
    pub instance_playtime: HashMap<String, u64>,
    /// History of play sessions.
    #[serde(default)]
    pub sessions: Vec<PlaySession>,
}

impl PlaytimeManager {
    fn file_path() -> PathBuf {
        let mut path = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()));
        path.push(".config");
        path.push("obelisk-launcher");
        path.push("playtime.json");
        path
    }

    pub fn load() -> Self {
        let path = Self::file_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(manager) = serde_json::from_str(&content) {
                    return manager;
                }
            }
        }
        // If file doesn't exist, we start fresh. 
        // We'll rely on the app to populate instance_playtime from existing data if needed,
        // but the user said "don't edit instance playtime in each instance", 
        // so we'll just track new data here.
        Self::default()
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::file_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn add_session(&mut self, session: PlaySession) {
        let entry = self.instance_playtime.entry(session.instance_id.clone()).or_insert(0);
        *entry += session.duration_seconds;
        self.sessions.push(session);
        let _ = self.save();
    }

    pub fn get_total_playtime(&self) -> u64 {
        self.instance_playtime.values().sum()
    }

    pub fn get_instance_playtime(&self, instance_id: &str) -> u64 {
        self.instance_playtime.get(instance_id).cloned().unwrap_or(0)
    }

    pub fn ensure_initialized(&mut self, instances: &[crate::backend::instance::manager::Instance]) {
        if self.instance_playtime.is_empty() {
            let mut changed = false;
            for inst in instances {
                if inst.total_time_played > 0 {
                    self.instance_playtime.insert(inst.id.clone(), inst.total_time_played);
                    changed = true;
                }
            }
            if changed {
                let _ = self.save();
            }
        }
    }
}
