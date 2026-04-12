use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PlaytimeManager {
    /// Map of instance ID to total playtime in seconds.
    pub instance_playtime: HashMap<String, u64>,
}

impl PlaytimeManager {
    fn file_path() -> PathBuf {
        let mut path = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()));
        path.push(".config");
        path.push("minecraft-manager");
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

    pub fn add_playtime(&mut self, instance_id: &str, seconds: u64) {
        let entry = self.instance_playtime.entry(instance_id.to_string()).or_insert(0);
        *entry += seconds;
        let _ = self.save();
    }

    pub fn get_total_playtime(&self) -> u64 {
        self.instance_playtime.values().sum()
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
