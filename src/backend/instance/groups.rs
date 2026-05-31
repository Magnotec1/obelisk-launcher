use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Persistent data for a single group.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GroupInfo {
    /// Prism uses `hidden` to indicate collapsed state; we default to false.
    #[serde(default)]
    pub hidden: bool,
    /// The set of instance *folder names* (not display names) that belong to this group.
    #[serde(default)]
    pub instances: HashSet<String>,
}

/// The full group state for the instances directory.
/// Serialises to / from `instgroups.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstanceGroups {
    #[serde(
        default = "default_format_version",
        deserialize_with = "deserialize_format_version"
    )]
    pub format_version: u32,
    #[serde(default)]
    pub groups: HashMap<String, GroupInfo>,
}

fn default_format_version() -> u32 {
    1
}

fn deserialize_format_version<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let v = serde_json::Value::deserialize(deserializer)?;
    match v {
        serde_json::Value::Number(n) => n
            .as_u64()
            .map(|x| x as u32)
            .ok_or_else(|| D::Error::custom("Invalid number")),
        serde_json::Value::String(s) => s.parse::<u32>().map_err(D::Error::custom),
        _ => Err(D::Error::custom(
            "Expected string or number for formatVersion",
        )),
    }
}

impl InstanceGroups {
    /// Load `instgroups.json` from `instances_path`.
    /// Returns an empty `InstanceGroups` if the file doesn't exist or is malformed.
    pub fn load(instances_path: &Path) -> Self {
        let path = instances_path.join("instgroups.json");
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(parsed) = serde_json::from_str::<InstanceGroups>(&content) {
                return parsed;
            }
        }
        InstanceGroups::default()
    }

    /// Save `instgroups.json` back to `instances_path`.
    pub fn save(&self, instances_path: &Path) -> Result<(), String> {
        let path = instances_path.join("instgroups.json");
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialise instgroups.json: {}", e))?;
        std::fs::write(&path, content)
            .map_err(|e| format!("Failed to write instgroups.json: {}", e))?;
        Ok(())
    }

    // ── Group mutations ───────────────────────────────────────────────────────

    /// Create a new empty group. No-op if it already exists.
    pub fn create_group(&mut self, name: &str) {
        self.groups.entry(name.to_string()).or_default();
    }

    /// Rename a group. No-op if `old_name` doesn't exist.
    /// If `new_name` already exists the instances are merged.
    pub fn rename_group(&mut self, old_name: &str, new_name: &str) {
        if let Some(info) = self.groups.remove(old_name) {
            let entry = self.groups.entry(new_name.to_string()).or_default();
            entry.instances.extend(info.instances);
        }
    }

    /// Delete a group entirely. Instances it contained become ungrouped.
    pub fn delete_group(&mut self, name: &str) {
        self.groups.remove(name);
    }

    // ── Instance ↔ group mutations ────────────────────────────────────────────

    /// Move `folder_name` to `group_name`, removing it from any previous group first.
    /// Creates the group if it doesn't exist.
    pub fn set_instance_group(&mut self, folder_name: &str, group_name: &str) {
        // Remove from any existing group
        for info in self.groups.values_mut() {
            info.instances.remove(folder_name);
        }
        // Add to target group (create if needed)
        self.groups
            .entry(group_name.to_string())
            .or_default()
            .instances
            .insert(folder_name.to_string());
    }

    /// Remove `folder_name` from whatever group it currently belongs to.
    pub fn remove_instance_from_groups(&mut self, folder_name: &str) {
        for info in self.groups.values_mut() {
            info.instances.remove(folder_name);
        }
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// Returns the name of the group that `folder_name` belongs to, if any.
    pub fn get_instance_group(&self, folder_name: &str) -> Option<&str> {
        for (name, info) in &self.groups {
            if info.instances.contains(folder_name) {
                return Some(name.as_str());
            }
        }
        None
    }

    /// Returns an alphabetically sorted list of group names.
    pub fn sorted_group_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.groups.keys().map(|s| s.as_str()).collect();
        names.sort_unstable();
        names
    }
}
