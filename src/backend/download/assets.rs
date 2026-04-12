use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A single deletable entry in the asset tree.
#[derive(Debug, Clone)]
pub struct AssetEntry {
    pub name: String,
    pub size: u64,
    pub path: PathBuf,
}

/// A group of related entries (e.g. "Sounds", "Textures", a single instance).
#[derive(Debug, Clone)]
pub struct AssetGroup {
    pub name: String,
    pub entries: Vec<AssetEntry>,
    pub total_size: u64,
}

/// Top-level category in the tree view.
#[derive(Debug, Clone)]
pub struct AssetCategory {
    pub name: String,
    pub icon: &'static str,
    pub groups: Vec<AssetGroup>,
    pub total_size: u64,
}

/// Full scan result.
#[derive(Debug, Clone)]
pub struct AssetScanResult {
    pub categories: Vec<AssetCategory>,
    pub total_size: u64,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Recursively compute directory size.
fn dir_size(path: &Path) -> u64 {
    if path.is_file() {
        return path.metadata().map(|m| m.len()).unwrap_or(0);
    }
    let mut total: u64 = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else {
                total += p.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

// ---------------------------------------------------------------------------
// Scanning
// ---------------------------------------------------------------------------

/// Scans the data directory, optional shared directory, and instances to build a full asset tree.
pub fn scan_assets(
    data_path: &Path,
    shared_path: Option<&Path>,
    instances_path: Option<&Path>,
) -> AssetScanResult {
    let paths: Vec<&Path> = if let Some(p) = shared_path {
        vec![data_path, p]
    } else {
        vec![data_path]
    };

    // Initialize categories
    let mut game_assets = AssetCategory {
        name: "Game Assets".to_string(),
        icon: "folder-music-symbolic",
        groups: Vec::new(),
        total_size: 0,
    };
    let mut client_jars = AssetCategory {
        name: "Client JARs".to_string(),
        icon: "application-x-executable-symbolic",
        groups: Vec::new(),
        total_size: 0,
    };
    let mut libraries = AssetCategory {
        name: "Libraries".to_string(),
        icon: "system-file-manager-symbolic",
        groups: Vec::new(),
        total_size: 0,
    };
    let mut metadata = AssetCategory {
        name: "Version Metadata".to_string(),
        icon: "text-x-generic-symbolic",
        groups: Vec::new(),
        total_size: 0,
    };

    for path in &paths {
        // 1. Asset Objects
        let assets_dir = path.join("assets");
        let indexes_dir = assets_dir.join("indexes");
        let objects_dir = assets_dir.join("objects");

        if indexes_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&indexes_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() && p.extension().map(|e| e == "json").unwrap_or(false) {
                        let index_name = p
                            .file_stem()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        if let Ok(content) = fs::read_to_string(&p) {
                            if let Ok(index) = serde_json::from_str::<
                                crate::backend::download::manager::AssetObjects,
                            >(&content)
                            {
                                let mut type_sizes: HashMap<String, u64> = HashMap::new();
                                for (key, obj) in &index.objects {
                                    let prefix = &obj.hash[0..2];
                                    let obj_path = objects_dir.join(prefix).join(&obj.hash);
                                    let actual_size = if obj_path.exists() {
                                        obj_path.metadata().map(|m| m.len()).unwrap_or(obj.size)
                                    } else {
                                        0
                                    };

                                    let type_name = if key.starts_with("minecraft/sounds/") {
                                        "Sounds"
                                    } else if key.starts_with("minecraft/textures/") {
                                        "Textures"
                                    } else if key.starts_with("minecraft/models/") {
                                        "Models"
                                    } else if key.starts_with("minecraft/blockstates/") {
                                        "Block States"
                                    } else if key.starts_with("minecraft/lang/") {
                                        "Languages"
                                    } else if key.starts_with("minecraft/font/") {
                                        "Fonts"
                                    } else if key.starts_with("minecraft/shaders/") {
                                        "Shaders"
                                    } else if key.starts_with("realms/") {
                                        "Realms"
                                    } else if key.starts_with("icons/") {
                                        "Icons"
                                    } else {
                                        "Other"
                                    };

                                    *type_sizes.entry(type_name.to_string()).or_insert(0) +=
                                        actual_size;
                                }

                                let mut sub_entries: Vec<AssetEntry> = type_sizes
                                    .into_iter()
                                    .map(|(name, size)| AssetEntry {
                                        name,
                                        size,
                                        path: PathBuf::new(), // Virtual entry, not directly deletable
                                    })
                                    .collect();
                                sub_entries.sort_by(|a, b| b.size.cmp(&a.size));
                                let total: u64 = sub_entries.iter().map(|e| e.size).sum();
                                let index_size = p.metadata().map(|m| m.len()).unwrap_or(0);

                                // Add the actual index file as a deletable entry
                                sub_entries.push(AssetEntry {
                                    name: "Index JSON File".to_string(),
                                    size: index_size,
                                    path: p.clone(),
                                });

                                game_assets.groups.push(AssetGroup {
                                    name: format!(
                                        "{} (in {})",
                                        index_name,
                                        path.file_name().and_then(|n| n.to_str()).unwrap_or("data")
                                    ),
                                    entries: sub_entries,
                                    total_size: total + index_size,
                                });
                                game_assets.total_size += total + index_size;
                            }
                        }
                    }
                }
            }
        }

        // 2. Client JARs
        let client_jar_dir = path
            .join("libraries")
            .join("com")
            .join("mojang")
            .join("minecraft");
        if client_jar_dir.is_dir() {
            let mut entries = Vec::new();
            if let Ok(versions) = fs::read_dir(&client_jar_dir) {
                for v_entry in versions.flatten() {
                    let v_path = v_entry.path();
                    if v_path.is_dir() {
                        let v_name = v_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let size = dir_size(&v_path);
                        entries.push(AssetEntry {
                            name: format!("Minecraft {} Client", v_name),
                            size,
                            path: v_path,
                        });
                    }
                }
            }
            if !entries.is_empty() {
                let total: u64 = entries.iter().map(|e| e.size).sum();
                client_jars.groups.push(AssetGroup {
                    name: format!(
                        "Versions in {}",
                        path.file_name().and_then(|n| n.to_str()).unwrap_or("data")
                    ),
                    total_size: total,
                    entries,
                });
                client_jars.total_size += total;
            }
        }

        // 3. Libraries
        let libraries_dir = path.join("libraries");
        if libraries_dir.is_dir() {
            let mut lib_groups: HashMap<String, (u64, Vec<AssetEntry>)> = HashMap::new();
            if let Ok(top_entries) = fs::read_dir(&libraries_dir) {
                for top_entry in top_entries.flatten() {
                    let top_path = top_entry.path();
                    if top_path.is_dir() {
                        let top_name = top_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        if top_name == "com" {
                            if let Ok(com_entries) = fs::read_dir(&top_path) {
                                for com_entry in com_entries.flatten() {
                                    let com_path = com_entry.path();
                                    let com_name = com_path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("unknown")
                                        .to_string();
                                    if com_name == "mojang" {
                                        if let Ok(mj_entries) = fs::read_dir(&com_path) {
                                            for mj_entry in mj_entries.flatten() {
                                                let mj_path = mj_entry.path();
                                                let mj_name = mj_path
                                                    .file_name()
                                                    .and_then(|n| n.to_str())
                                                    .unwrap_or("")
                                                    .to_string();
                                                if mj_name != "minecraft" && mj_path.is_dir() {
                                                    let size = dir_size(&mj_path);
                                                    let group = lib_groups
                                                        .entry("com.mojang".to_string())
                                                        .or_insert_with(|| (0, Vec::new()));
                                                    group.0 += size;
                                                    group.1.push(AssetEntry {
                                                        name: mj_name,
                                                        size,
                                                        path: mj_path,
                                                    });
                                                }
                                            }
                                        }
                                    } else {
                                        let size = dir_size(&com_path);
                                        let group = lib_groups
                                            .entry(format!("com.{}", com_name))
                                            .or_insert_with(|| (0, Vec::new()));
                                        group.0 += size;
                                        group.1.push(AssetEntry {
                                            name: com_name,
                                            size,
                                            path: com_path,
                                        });
                                    }
                                }
                            }
                        } else {
                            let size = dir_size(&top_path);
                            let group = lib_groups
                                .entry(top_name.clone())
                                .or_insert_with(|| (0, Vec::new()));
                            group.0 += size;
                            group.1.push(AssetEntry {
                                name: top_name,
                                size,
                                path: top_path,
                            });
                        }
                    }
                }
            }
            for (name, (total, mut entries)) in lib_groups {
                entries.sort_by(|a, b| b.size.cmp(&a.size));
                libraries.groups.push(AssetGroup {
                    name: format!(
                        "{} (in {})",
                        name,
                        path.file_name().and_then(|n| n.to_str()).unwrap_or("data")
                    ),
                    entries,
                    total_size: total,
                });
                libraries.total_size += total;
            }
        }

        // 4. Metadata
        let meta_dir = path.join("meta");
        if meta_dir.is_dir() {
            let mut entries = Vec::new();
            if let Ok(meta_entries) = fs::read_dir(&meta_dir) {
                for m_entry in meta_entries.flatten() {
                    let m_path = m_entry.path();
                    let m_name = m_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let size = dir_size(&m_path);
                    entries.push(AssetEntry {
                        name: m_name,
                        size,
                        path: m_path,
                    });
                }
            }
            if !entries.is_empty() {
                let total: u64 = entries.iter().map(|e| e.size).sum();
                metadata.groups.push(AssetGroup {
                    name: format!(
                        "Components in {}",
                        path.file_name().and_then(|n| n.to_str()).unwrap_or("data")
                    ),
                    total_size: total,
                    entries,
                });
                metadata.total_size += total;
            }
        }
    }

    let mut grand_total = game_assets.total_size
        + client_jars.total_size
        + libraries.total_size
        + metadata.total_size;
    let mut categories = vec![game_assets, client_jars, libraries, metadata];

    // 5. Instance Data
    if let Some(inst_path) = instances_path {
        if inst_path.is_dir() {
            let mut groups = Vec::new();
            if let Ok(entries) = fs::read_dir(inst_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_dir() || !path.join("instance.cfg").is_file() {
                        continue;
                    }

                    let minecraft_dir = if path.join(".minecraft").is_dir() {
                        Some(path.join(".minecraft"))
                    } else if path.join("minecraft").is_dir() {
                        Some(path.join("minecraft"))
                    } else {
                        None
                    };

                    if let Some(m_dir) = minecraft_dir {
                        let mut inst_entries = Vec::new();
                        let subdirs = vec![
                            ("mods", "Mods"),
                            ("resourcepacks", "Resource Packs"),
                            ("shaderpacks", "Shader Packs"),
                            ("saves", "Worlds"),
                            ("config", "Config"),
                            ("logs", "Logs"),
                        ];
                        for (dir, label) in subdirs {
                            let p = m_dir.join(dir);
                            if p.is_dir() {
                                let size = dir_size(&p);
                                if size > 0 {
                                    inst_entries.push(AssetEntry {
                                        name: label.to_string(),
                                        size,
                                        path: p,
                                    });
                                }
                            }
                        }
                        inst_entries.sort_by(|a, b| b.size.cmp(&a.size));
                        let total: u64 = inst_entries.iter().map(|e| e.size).sum();
                        if total > 0 {
                            let inst_name = {
                                let mut name = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("Unknown")
                                    .to_string();
                                if let Ok(content) = fs::read_to_string(path.join("instance.cfg")) {
                                    for line in content.lines().filter(|l| l.starts_with("name=")) {
                                        name = line["name=".len()..].trim().to_string();
                                    }
                                }
                                name
                            };
                            groups.push(AssetGroup {
                                name: inst_name,
                                entries: inst_entries,
                                total_size: total,
                            });
                        }
                    }
                }
            }
            groups.sort_by(|a, b| b.total_size.cmp(&a.total_size));
            let total: u64 = groups.iter().map(|g| g.total_size).sum();
            grand_total += total;
            categories.push(AssetCategory {
                name: "Instance Data".to_string(),
                icon: "drive-harddisk-symbolic",
                groups,
                total_size: total,
            });
        }
    }

    categories.sort_by(|a, b| b.total_size.cmp(&a.total_size));
    AssetScanResult {
        categories,
        total_size: grand_total,
    }
}

/// Scans for per-Minecraft-version data, grouping the client JAR,
/// version meta JSON, and asset index JSON for each discovered version.
/// This will combine versions found in both primary and shared data paths.
pub fn scan_versions(data_path: &Path, shared_path: Option<&Path>) -> Vec<AssetGroup> {
    let mut groups_map: HashMap<String, AssetGroup> = HashMap::new();
    let paths: Vec<&Path> = if let Some(p) = shared_path {
        vec![data_path, p]
    } else {
        vec![data_path]
    };

    for path in paths {
        let client_jar_dir = path
            .join("libraries")
            .join("com")
            .join("mojang")
            .join("minecraft");
        if let Ok(entries) = std::fs::read_dir(&client_jar_dir) {
            for entry in entries.flatten() {
                let v_dir = entry.path();
                if !v_dir.is_dir() {
                    continue;
                }
                let version = v_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let mut entries = Vec::new();
                let mut total: u64 = 0;

                let jar_size = dir_size(&v_dir);
                if jar_size > 0 {
                    entries.push(AssetEntry {
                        name: format!("Client JAR ({})", format_size(jar_size)),
                        size: jar_size,
                        path: v_dir.clone(),
                    });
                    total += jar_size;
                }

                let meta_path = path
                    .join("meta")
                    .join("net.minecraft")
                    .join(format!("{}.json", version));
                let mut asset_index_id: Option<String> = None;
                if meta_path.exists() {
                    let meta_size = meta_path.metadata().map(|m| m.len()).unwrap_or(0);
                    if let Ok(content) = std::fs::read_to_string(&meta_path) {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                            asset_index_id = val
                                .get("assetIndex")
                                .and_then(|ai| ai.get("id"))
                                .and_then(|id| id.as_str())
                                .map(|s| s.to_string());
                        }
                    }
                    entries.push(AssetEntry {
                        name: format!("Version Metadata ({})", format_size(meta_size)),
                        size: meta_size,
                        path: meta_path,
                    });
                    total += meta_size;
                }

                let index_id = asset_index_id.unwrap_or_else(|| version.clone());
                let index_path = path
                    .join("assets")
                    .join("indexes")
                    .join(format!("{}.json", index_id));
                if index_path.exists() {
                    let index_size = index_path.metadata().map(|m| m.len()).unwrap_or(0);
                    entries.push(AssetEntry {
                        name: format!(
                            "Asset Index \"{}\" ({}) — shared objects not included",
                            index_id,
                            format_size(index_size)
                        ),
                        size: index_size,
                        path: index_path,
                    });
                    total += index_size;
                }

                if !entries.is_empty() {
                    let group = groups_map
                        .entry(version.clone())
                        .or_insert_with(|| AssetGroup {
                            name: version,
                            entries: Vec::new(),
                            total_size: 0,
                        });
                    group.entries.extend(entries);
                    group.total_size += total;
                }
            }
        }
    }

    let mut result: Vec<AssetGroup> = groups_map.into_values().collect();
    result.sort_by(|a, b| b.total_size.cmp(&a.total_size));
    result
}

/// Deletes the file or directory at the given path. Returns the freed bytes.
pub fn delete_asset(path: &Path) -> Result<u64, String> {
    if !path.exists() {
        return Ok(0);
    }
    let size = dir_size(path);
    if path.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|e| format!("Failed to delete {}: {}", path.display(), e))?;
    } else {
        fs::remove_file(path).map_err(|e| format!("Failed to delete {}: {}", path.display(), e))?;
    }
    Ok(size)
}
