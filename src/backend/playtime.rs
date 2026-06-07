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
pub struct InstancePlaytimeData {
    pub name: String,
    pub playtime: u64,
    pub session_count: usize,
    #[serde(default)]
    pub last_played: Option<DateTime<Utc>>,
    #[serde(default)]
    pub first_played: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlaytimeManager {
    #[serde(default = "default_version")]
    pub version: String,
    pub instances: HashMap<String, InstancePlaytimeData>,
    #[serde(default)]
    pub sessions: Vec<PlaySession>,
}

fn default_version() -> String {
    "2.0".to_string()
}

impl Default for PlaytimeManager {
    fn default() -> Self {
        Self {
            version: default_version(),
            instances: HashMap::new(),
            sessions: Vec::new(),
        }
    }
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
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                    let version = value.get("version").and_then(|v| v.as_str());
                    if version == Some("2.0") {
                        if let Ok(manager) = serde_json::from_value::<PlaytimeManager>(value) {
                            return manager;
                        }
                    } else {
                        // Migrate legacy format
                        let mut instances = HashMap::new();
                        
                        let legacy_playtime = value.get("instance_playtime").and_then(|v| v.as_object());
                        let legacy_names = value.get("instance_names").and_then(|v| v.as_object());
                        let legacy_sessions_val = value.get("sessions").and_then(|v| v.as_array());

                        let mut sessions = Vec::new();
                        if let Some(arr) = legacy_sessions_val {
                            for item in arr {
                                if let Ok(sess) = serde_json::from_value::<PlaySession>(item.clone()) {
                                    sessions.push(sess);
                                }
                            }
                        }

                        if let Some(playtimes) = legacy_playtime {
                            for (id, playtime_val) in playtimes {
                                if let Some(playtime) = playtime_val.as_u64() {
                                    let name = legacy_names
                                        .and_then(|names| names.get(id))
                                        .and_then(|n| n.as_str())
                                        .unwrap_or(id)
                                        .to_string();

                                    let instance_sessions: Vec<&PlaySession> = sessions
                                        .iter()
                                        .filter(|s| &s.instance_id == id)
                                        .collect();

                                    let last_played = instance_sessions.iter().map(|s| s.end_time).max();
                                    let first_played = instance_sessions.iter().map(|s| s.start_time).min();
                                    let session_count = instance_sessions.len();

                                    instances.insert(
                                        id.clone(),
                                        InstancePlaytimeData {
                                            name,
                                            playtime,
                                            session_count,
                                            last_played,
                                            first_played,
                                        },
                                    );
                                }
                            }
                        }

                        // Cap global sessions to previous 5
                        sessions.sort_by(|a, b| b.end_time.cmp(&a.end_time));
                        sessions.truncate(5);

                        let manager = PlaytimeManager {
                            version: "2.0".to_string(),
                            instances,
                            sessions,
                        };
                        let _ = manager.save();
                        return manager;
                    }
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

    pub fn add_session(&mut self, session: PlaySession) {
        let entry = self
            .instances
            .entry(session.instance_id.clone())
            .or_insert_with(|| InstancePlaytimeData {
                name: "Unknown Instance".to_string(),
                playtime: 0,
                session_count: 0,
                last_played: None,
                first_played: None,
            });

        entry.playtime += session.duration_seconds;
        entry.session_count += 1;
        entry.last_played = Some(
            entry
                .last_played
                .map(|lp| lp.max(session.end_time))
                .unwrap_or(session.end_time),
        );
        if entry.first_played.is_none() {
            entry.first_played = Some(session.start_time);
        }

        self.sessions.push(session);
        // Cap global sessions to previous 5
        self.sessions.sort_by(|a, b| b.end_time.cmp(&a.end_time));
        self.sessions.truncate(5);

        let _ = self.save();
    }

    pub fn get_total_playtime(&self) -> u64 {
        self.instances.values().map(|i| i.playtime).sum()
    }

    pub fn get_instance_playtime(&self, instance_id: &str) -> u64 {
        self.instances
            .get(instance_id)
            .map(|i| i.playtime)
            .unwrap_or(0)
    }

    pub fn ensure_initialized(
        &mut self,
        instances: &[crate::backend::instance::manager::Instance],
    ) {
        let mut changed = false;

        for inst in instances {
            let entry = self.instances.entry(inst.id.clone()).or_insert_with(|| {
                changed = true;
                InstancePlaytimeData {
                    name: inst.name.clone(),
                    playtime: inst.total_time_played,
                    session_count: 0,
                    last_played: None,
                    first_played: None,
                }
            });

            if entry.name != inst.name {
                entry.name = inst.name.clone();
                changed = true;
            }

            if entry.playtime == 0 && inst.total_time_played > 0 {
                entry.playtime = inst.total_time_played;
                changed = true;
            }
        }

        if changed {
            let _ = self.save();
        }
    }
}
