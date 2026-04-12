use crate::backend::auth::microsoft::Account;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub instances_path: Option<PathBuf>,
    #[serde(alias = "fallback_data_path")]
    pub shared_data_path: Option<PathBuf>,
    pub minecraft_data_path: PathBuf,
    pub java_path: Option<PathBuf>,
    pub max_memory: u32,
    pub min_memory: u32,
    pub microsoft_client_id: Option<String>,
    #[serde(default)]
    pub accounts: Vec<Account>,
    pub active_account_uuid: Option<String>,
    /// Path to the global default instance icon image.
    #[serde(default)]
    pub default_instance_icon: Option<PathBuf>,
    /// Recently used instance icon paths (most recent first, max 12).
    #[serde(default)]
    pub recent_instance_icons: Vec<PathBuf>,
    /// Total playtime across all instances (persistent even if instances are deleted).
    #[serde(default)]
    pub total_playtime: u64,
}

impl Default for Config {
    fn default() -> Self {
        let home = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()));

        let mut mc_data = home.clone();
        mc_data.push(".local/share/minecraft-manager");

        Self {
            instances_path: None,
            shared_data_path: None,
            minecraft_data_path: mc_data,
            java_path: Some(PathBuf::from("java")),
            max_memory: 4096,
            min_memory: 512,
            microsoft_client_id: None,
            accounts: Vec::new(),
            active_account_uuid: None,
            default_instance_icon: None,
            recent_instance_icons: Vec::new(),
            total_playtime: 0,
        }
    }
}

impl Config {
    fn config_path() -> PathBuf {
        let mut path = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()));
        path.push(".config");
        path.push("minecraft-manager");
        path.push("config.json");
        path
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        Config::default()
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}
