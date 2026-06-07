use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModLoader {
    None,
    Fabric,
    Forge,
    Quilt,
    NeoForge,
}

impl ModLoader {
    pub fn as_str(&self) -> &str {
        match self {
            ModLoader::None => "None",
            ModLoader::Fabric => "Fabric",
            ModLoader::Forge => "Forge",
            ModLoader::Quilt => "Quilt",
            ModLoader::NeoForge => "NeoForge",
        }
    }

    /// Returns the MMC component UID for this loader, or None if no loader is used.
    pub fn uid(&self) -> Option<&str> {
        match self {
            ModLoader::None => None,
            ModLoader::Fabric => Some("net.fabricmc.fabric-loader"),
            ModLoader::Forge => Some("net.minecraftforge"),
            ModLoader::Quilt => Some("org.quiltmc.quilt-loader"),
            ModLoader::NeoForge => Some("net.neoforged"),
        }
    }

    /// Returns the display name used in mmc-pack.json cachedName.
    pub fn cached_name(&self) -> Option<&str> {
        match self {
            ModLoader::None => None,
            ModLoader::Fabric => Some("Fabric Loader"),
            ModLoader::Forge => Some("Forge"),
            ModLoader::Quilt => Some("Quilt Loader"),
            ModLoader::NeoForge => Some("NeoForge"),
        }
    }

    pub fn all() -> Vec<ModLoader> {
        vec![
            ModLoader::None,
            ModLoader::Fabric,
            ModLoader::Forge,
            ModLoader::Quilt,
            ModLoader::NeoForge,
        ]
    }
}

impl std::fmt::Display for ModLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub struct CreateInstanceOptions {
    pub name: String,
    pub minecraft_version: String,
    pub mod_loader: ModLoader,
    pub loader_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InstanceComponent {
    pub uid: String,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct ModInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub filename: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub icon_path: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct ResourcePackInfo {
    pub name: String,
    pub filename: String,
    pub description: Option<String>,
    pub size: u64,
    pub icon_path: Option<String>,
    pub format: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ShaderPackInfo {
    pub name: String,
    pub filename: String,
    pub description: Option<String>,
    pub size: u64,
    pub icon_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorldInfo {
    pub name: String,
    pub folder_name: String,
    pub file_size: u64,
    pub seed: Option<i64>,
    pub mc_version: Option<String>,
    pub last_played: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct Instance {
    pub name: String,
    pub path: PathBuf,
    pub icon_key: Option<String>,
    pub total_time_played: u64,
    pub last_launched: Option<u64>,
    pub minecraft_version: Option<String>,
    pub mod_loader: Option<String>,
    pub components: Vec<InstanceComponent>,
    pub mods: Vec<ModInfo>,
    pub resource_packs: Vec<ResourcePackInfo>,
    pub shader_packs: Vec<ShaderPackInfo>,
    pub worlds: Vec<WorldInfo>,
    pub screenshot_count: usize,
    pub java_path: Option<PathBuf>,
    pub minecraft_dir: PathBuf,
    pub has_mismatch: bool,
    pub feral_gamemode: bool,
    pub discrete_gpu: bool,
    pub zink_vulkan: bool,
    pub use_wayland: bool,
    pub id: String,
}

impl Instance {
    pub fn get_loader_info(&self) -> (ModLoader, Option<String>) {
        for comp in &self.components {
            if comp.uid == "net.fabricmc.fabric-loader" {
                return (ModLoader::Fabric, Some(comp.version.clone()));
            } else if comp.uid == "net.minecraftforge" {
                return (ModLoader::Forge, Some(comp.version.clone()));
            } else if comp.uid == "org.quiltmc.quilt-loader" {
                return (ModLoader::Quilt, Some(comp.version.clone()));
            } else if comp.uid == "net.neoforged" {
                return (ModLoader::NeoForge, Some(comp.version.clone()));
            }
        }

        // Fallback to the parsed mod_loader if no matching component is found
        if let Some(loader_str) = &self.mod_loader {
            match loader_str.as_str() {
                "Fabric" => return (ModLoader::Fabric, None),
                "Forge" => return (ModLoader::Forge, None),
                "Quilt" => return (ModLoader::Quilt, None),
                "NeoForge" => return (ModLoader::NeoForge, None),
                _ => {}
            }
        }

        (ModLoader::None, None)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MmcRequirement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equals: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggests: Option<String>,
    pub uid: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MmcComponent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_requires: Option<Vec<MmcRequirement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependency_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub important: Option<bool>,
    pub uid: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MmcPack {
    pub components: Vec<MmcComponent>,
    pub format_version: u32,
}

#[derive(Deserialize)]
struct FabricModJson {
    id: String,
    name: Option<String>,
    version: String,
    description: Option<String>,
    contact: Option<FabricContact>,
    icon: Option<FabricIcon>,
}

#[derive(Deserialize)]
struct FabricContact {
    homepage: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum FabricIcon {
    Single(String),
    Multiple(std::collections::HashMap<String, String>),
}

#[derive(Deserialize, Debug)]
struct ForgeModsToml {
    mods: Vec<ForgeMod>,
}

#[derive(Deserialize, Debug)]
struct ForgeMod {
    #[serde(rename = "modId")]
    mod_id: String,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    version: String,
    description: Option<String>,
    #[serde(rename = "displayURL")]
    display_url: Option<String>,
    #[serde(rename = "logoFile")]
    pub logo_file: Option<String>,
}

#[derive(Deserialize)]
struct PackMcMeta {
    pack: PackMetaContent,
}

#[derive(Deserialize)]
struct PackMetaContent {
    pub pack_format: i32,
    pub description: serde_json::Value,
}

fn parse_description(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::String(text)) = map.get("text") {
                text.clone()
            } else {
                val.to_string()
            }
        }
        _ => val.to_string(),
    }
}

fn extract_icon_to_cache(
    archive: &mut zip::ZipArchive<fs::File>,
    icon_path_in_jar: &str,
    cache_filename: &str,
) -> Option<String> {
    let clean_path = icon_path_in_jar
        .strip_prefix('/')
        .unwrap_or(icon_path_in_jar);
    if let Ok(mut icon_file) = archive.by_name(clean_path) {
        if let Some(proj_dirs) =
            directories::ProjectDirs::from("com", "magnotec", "obelisk-launcher")
        {
            let icons_dir = proj_dirs.cache_dir().join("icons");
            let _ = fs::create_dir_all(&icons_dir);
            let out_path = icons_dir.join(cache_filename);
            if let Ok(mut out_file) = fs::File::create(&out_path) {
                if std::io::copy(&mut icon_file, &mut out_file).is_ok() {
                    return Some(out_path.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

fn get_mod_info(path: &Path, full: bool) -> ModInfo {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();
    let mut info = ModInfo {
        id: filename.clone(),
        name: filename.clone(),
        version: "Unknown".to_string(),
        filename: filename.clone(),
        description: None,
        homepage: None,
        icon_path: None,
        enabled: !filename.ends_with(".disabled"),
    };

    if !full {
        return info;
    }

    if let Ok(file) = fs::File::open(path) {
        if let Ok(mut archive) = zip::ZipArchive::new(file) {
            // Check for Fabric/Quilt
            let mut found = false;
            for metadata_file in ["fabric.mod.json", "quilt.mod.json"] {
                let metadata = if let Ok(mut mod_json) = archive.by_name(metadata_file) {
                    serde_json::from_reader::<_, FabricModJson>(&mut mod_json).ok()
                } else {
                    None
                };

                if let Some(data) = metadata {
                    info.id = data.id.clone();
                    info.name = data.name.unwrap_or_else(|| data.id.clone());
                    info.version = data.version;
                    info.description = data.description;
                    info.homepage = data.contact.and_then(|c| c.homepage);

                    // Extract icon
                    if let Some(icon) = data.icon {
                        let icon_in_jar = match icon {
                            FabricIcon::Single(s) => s,
                            FabricIcon::Multiple(m) => m
                                .get("96")
                                .or_else(|| m.get("64"))
                                .or_else(|| m.get("32"))
                                .or_else(|| m.values().next())
                                .cloned()
                                .unwrap_or_default(),
                        };

                        if !icon_in_jar.is_empty() {
                            let cache_filename = format!("{}-{}.png", info.id, info.version);
                            info.icon_path =
                                extract_icon_to_cache(&mut archive, &icon_in_jar, &cache_filename);
                        }
                    }

                    found = true;
                    break;
                }
            }

            // Check for Forge (mods.toml)
            if !found {
                let forge_content = {
                    if let Ok(mut mods_toml_file) = archive.by_name("META-INF/mods.toml") {
                        let mut content = String::new();
                        use std::io::Read;
                        if mods_toml_file.read_to_string(&mut content).is_ok() {
                            Some(content)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };

                if let Some(content) = forge_content {
                    if let Ok(data) = toml::from_str::<ForgeModsToml>(&content) {
                        if let Some(mod_data) = data.mods.first() {
                            info.id = mod_data.mod_id.clone();
                            info.name = mod_data
                                .display_name
                                .clone()
                                .unwrap_or_else(|| mod_data.mod_id.clone());
                            info.version = mod_data.version.clone();
                            info.description = mod_data.description.clone();
                            info.homepage = mod_data.display_url.clone();

                            // Extract Forge icon
                            if let Some(logo) = &mod_data.logo_file {
                                let cache_filename =
                                    format!("{}-{}-forge.png", info.id, info.version);
                                info.icon_path =
                                    extract_icon_to_cache(&mut archive, logo, &cache_filename);
                            }
                        }
                    }
                }
            }
        }
    }

    info
}

fn get_resource_pack_info(path: &Path, full: bool) -> ResourcePackInfo {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let mut info = ResourcePackInfo {
        name: filename.clone(),
        filename: filename.clone(),
        description: None,
        size: 0,
        icon_path: None,
        format: None,
    };

    if !full {
        return info;
    }

    // Size calculation
    if path.is_file() {
        info.size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    } else if path.is_dir() {
        info.size = walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter_map(|e| e.metadata().ok())
            .filter(|m| m.is_file())
            .map(|m| m.len())
            .sum();
    }

    if path.is_file() {
        if let Ok(file) = fs::File::open(path) {
            if let Ok(mut archive) = zip::ZipArchive::new(file) {
                // Read pack.mcmeta
                if let Ok(mut meta_file) = archive.by_name("pack.mcmeta") {
                    if let Ok(meta) = serde_json::from_reader::<_, PackMcMeta>(&mut meta_file) {
                        info.format = Some(meta.pack.pack_format);
                        info.description = Some(parse_description(&meta.pack.description));
                    }
                }

                // Extract pack.png icon
                let cache_filename = format!("rp-{}.png", info.filename);
                info.icon_path = extract_icon_to_cache(&mut archive, "pack.png", &cache_filename);
            }
        }
    } else if path.is_dir() {
        // Read pack.mcmeta
        let meta_path = path.join("pack.mcmeta");
        if let Ok(content) = fs::read_to_string(meta_path) {
            if let Ok(meta) = serde_json::from_str::<PackMcMeta>(&content) {
                info.format = Some(meta.pack.pack_format);
                info.description = Some(parse_description(&meta.pack.description));
            }
        }

        // Use pack.png if it exists
        let icon_path = path.join("pack.png");
        if icon_path.exists() {
            info.icon_path = Some(icon_path.to_string_lossy().to_string());
        }
    }

    info
}

fn get_shader_pack_info(path: &Path, full: bool) -> ShaderPackInfo {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let mut info = ShaderPackInfo {
        name: filename.clone(),
        filename: filename.clone(),
        description: None,
        size: 0,
        icon_path: None,
    };

    if !full {
        return info;
    }

    // Size calculation
    if path.is_file() {
        info.size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    } else if path.is_dir() {
        info.size = walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter_map(|e| e.metadata().ok())
            .filter(|m| m.is_file())
            .map(|m| m.len())
            .sum();
    }

    if path.is_file() {
        if let Ok(file) = fs::File::open(path) {
            if let Ok(mut archive) = zip::ZipArchive::new(file) {
                // Some shaderpacks have a pack.png too
                let cache_filename = format!("sp-{}.png", info.filename);
                info.icon_path = extract_icon_to_cache(&mut archive, "pack.png", &cache_filename);
            }
        }
    } else if path.is_dir() {
        let icon_path = path.join("pack.png");
        if icon_path.exists() {
            info.icon_path = Some(icon_path.to_string_lossy().to_string());
        }
    }

    info
}

pub fn scan_instances(instances_path: &Path) -> Vec<Instance> {
    use rayon::prelude::*;
    let mut instances = Vec::new();
    if let Ok(entries) = fs::read_dir(instances_path) {
        let entries: Vec<_> = entries.flatten().collect();
        instances = entries
            .par_iter()
            .filter_map(|entry| {
                let path = entry.path();
                if path.is_dir() {
                    scan_single_instance(&path, false)
                } else {
                    None
                }
            })
            .collect();
    }
    // Sort instances alphabetically by name
    instances.sort_by(|a, b| a.name.cmp(&b.name));
    instances
}

/// Rescans a single instance directory and returns the updated Instance.
/// This is much faster than `scan_instances()` when only one instance changed.
pub fn scan_single_instance(instance_path: &Path, full_scan: bool) -> Option<Instance> {
    if !instance_path.is_dir() {
        return None;
    }

    let has_mismatch =
        instance_path.join(".minecraft").is_dir() && instance_path.join("minecraft").is_dir();

    let minecraft_dir = get_minecraft_dir(instance_path);

    let cfg_path = instance_path.join("instance.cfg");
    if !cfg_path.is_file() {
        return None;
    }

    let mut name = instance_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();
    let mut icon_key = None;
    let mut total_time_played = 0;
    let mut last_launched = None;
    let mut minecraft_version = None;
    let mut mod_loader = None;
    let mut java_path = None;
    let mut feral_gamemode = false;
    let mut discrete_gpu = false;
    let mut zink_vulkan = false;
    let mut use_wayland = false;
    let mut id = None;
    let mut components = Vec::new();

    // Parse instance.cfg
    if let Ok(content) = fs::read_to_string(&cfg_path) {
        let mut in_general = false;
        for line in content.lines() {
            let line = line.trim();
            if line == "[General]" {
                in_general = true;
            } else if line.starts_with('[') && line.ends_with(']') {
                in_general = false;
            } else if in_general {
                if let Some((key, value)) = line.split_once('=') {
                    match key.trim() {
                        "name" => name = value.trim().to_string(),
                        "iconKey" => icon_key = Some(value.trim().to_string()),
                        "totalTimePlayed" => total_time_played = value.trim().parse().unwrap_or(0),
                        "lastLaunchTime" => {
                            last_launched = value.trim().parse().ok();
                        }
                        "IntendedVersion" => minecraft_version = Some(value.trim().to_string()),
                        "JavaPath" => java_path = Some(PathBuf::from(value.trim())),
                        "FeralGameMode" => feral_gamemode = value.trim() == "true",
                        "DiscreteGpu" => discrete_gpu = value.trim() == "true",
                        "ZinkVulkan" => zink_vulkan = value.trim() == "true",
                        "UseWayland" => use_wayland = value.trim() == "true",
                        "instanceId" => id = Some(value.trim().to_string()),
                        _ => {}
                    }
                }
            }
        }
    }

    // Parse mmc-pack.json
    let pack_path = instance_path.join("mmc-pack.json");
    if let Ok(pack_content) = fs::read_to_string(pack_path) {
        if let Ok(pack) = serde_json::from_str::<MmcPack>(&pack_content) {
            for comp in pack.components {
                let c_name = comp.cached_name.clone().unwrap_or_else(|| comp.uid.clone());
                let c_version = comp.version.clone();

                if comp.uid == "net.minecraft" {
                    minecraft_version = Some(c_version.clone());
                } else if comp.uid.contains("fabric-loader") {
                    mod_loader = Some("Fabric".to_string());
                } else if comp.uid == "net.neoforged" {
                    mod_loader = Some("NeoForge".to_string());
                } else if comp.uid.contains("forge") {
                    mod_loader = Some("Forge".to_string());
                } else if comp.uid.contains("quilt-loader") {
                    mod_loader = Some("Quilt".to_string());
                }

                components.push(InstanceComponent {
                    uid: comp.uid.clone(),
                    name: c_name,
                    version: c_version,
                });
            }
        }
    }

    // Scan folders
    let mut mods = Vec::new();
    let mut resource_packs = Vec::new();
    let mut shader_packs = Vec::new();
    let mut worlds = Vec::new();

    {
        let m_dir = &minecraft_dir;
        let mods_dir = m_dir.join("mods");
        if let Ok(mod_entries) = fs::read_dir(mods_dir) {
            for mod_entry in mod_entries.flatten() {
                let m_path = mod_entry.path();
                if m_path.is_file() {
                    let fname = m_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_default();
                    if fname.ends_with(".jar") || fname.ends_with(".jar.disabled") {
                        mods.push(get_mod_info(&m_path, full_scan));
                    }
                }
            }
        }
        mods.sort_by(|a, b| a.name.cmp(&b.name));

        let rp_dir = m_dir.join("resourcepacks");
        if let Ok(rp_entries) = fs::read_dir(rp_dir) {
            for rp_entry in rp_entries.flatten() {
                resource_packs.push(get_resource_pack_info(&rp_entry.path(), full_scan));
            }
        }
        resource_packs.sort_by(|a, b| a.name.cmp(&b.name));

        let sp_dir = m_dir.join("shaderpacks");
        if let Ok(sp_entries) = fs::read_dir(sp_dir) {
            for sp_entry in sp_entries.flatten() {
                shader_packs.push(get_shader_pack_info(&sp_entry.path(), full_scan));
            }
        }
        shader_packs.sort_by(|a, b| a.name.cmp(&b.name));

        let worlds_dir = m_dir.join("saves");
        if let Ok(w_entries) = fs::read_dir(worlds_dir) {
            for w_entry in w_entries.flatten() {
                if w_entry.path().is_dir() {
                    let w_path = w_entry.path();
                    let folder_name = w_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown")
                        .to_string();

                    if !full_scan {
                        worlds.push(WorldInfo {
                            name: folder_name.clone(),
                            folder_name: folder_name.clone(),
                            file_size: 0,
                            seed: None,
                            mc_version: None,
                            last_played: None,
                        });
                        continue;
                    }

                    let mut name = folder_name.clone();
                    let mut seed = None;
                    let mut mc_version = None;
                    let mut last_played = None;

                    let file_size: u64 = walkdir::WalkDir::new(&w_path)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter_map(|e| e.metadata().ok())
                        .filter(|m| m.is_file())
                        .map(|m| m.len())
                        .sum();

                    let level_dat = w_path.join("level.dat");
                    if let Ok(file) = fs::File::open(&level_dat) {
                        use flate2::read::GzDecoder;
                        use std::io::Read;
                        let mut decoder = GzDecoder::new(file);
                        let mut bytes = Vec::new();
                        if decoder.read_to_end(&mut bytes).is_ok() {
                            #[derive(serde::Deserialize, Debug)]
                            struct FastNbtVersion {
                                #[serde(rename = "Name")]
                                name: Option<String>,
                            }
                            #[derive(serde::Deserialize, Debug)]
                            struct FastNbtWorldGenSettings {
                                #[serde(rename = "seed")]
                                seed: Option<i64>,
                            }
                            #[derive(serde::Deserialize, Debug)]
                            struct FastNbtData {
                                #[serde(rename = "LevelName")]
                                level_name: Option<String>,
                                #[serde(rename = "RandomSeed")]
                                random_seed: Option<i64>,
                                #[serde(rename = "WorldGenSettings")]
                                world_gen_settings: Option<FastNbtWorldGenSettings>,
                                #[serde(rename = "LastPlayed")]
                                last_played: Option<i64>,
                                #[serde(rename = "Version")]
                                version: Option<FastNbtVersion>,
                            }
                            #[derive(serde::Deserialize, Debug)]
                            struct FastNbtLevelDat {
                                #[serde(rename = "Data")]
                                data: Option<FastNbtData>,
                            }

                            if let Ok(parsed) = fastnbt::from_bytes::<FastNbtLevelDat>(&bytes) {
                                if let Some(data) = parsed.data {
                                    if let Some(level_name) = data.level_name {
                                        name = level_name;
                                    }
                                    seed = data
                                        .random_seed
                                        .or_else(|| data.world_gen_settings.and_then(|s| s.seed));
                                    last_played = data.last_played;
                                    if let Some(ver) = data.version {
                                        mc_version = ver.name;
                                    }
                                }
                            }
                        }
                    }

                    worlds.push(WorldInfo {
                        name,
                        folder_name,
                        file_size,
                        seed,
                        mc_version,
                        last_played,
                    });
                }
            }
        }
        worlds.sort_by(|a, b| a.name.cmp(&b.name));
    }

    let screenshot_count = {
        let ss_dir = minecraft_dir.join("screenshots");
        if let Ok(entries) = fs::read_dir(ss_dir) {
            entries.flatten().filter(|e| e.path().is_file()).count()
        } else {
            0
        }
    };

    // Ensure we have a persistent ID
    let final_id = if let Some(id) = id {
        id
    } else {
        let new_id = uuid::Uuid::new_v4().to_string();
        let _ = update_cfg_key(instance_path, "instanceId", &new_id);
        new_id
    };

    Some(Instance {
        name,
        path: instance_path.to_path_buf(),
        icon_key,
        total_time_played,
        last_launched,
        minecraft_version,
        mod_loader,
        components,
        mods,
        resource_packs,
        shader_packs,
        worlds,
        screenshot_count,
        java_path,
        minecraft_dir,
        has_mismatch,
        feral_gamemode,
        discrete_gpu,
        zink_vulkan,
        use_wayland,
        id: final_id,
    })
}

pub fn create_instance(
    instances_path: &Path,
    options: CreateInstanceOptions,
) -> Result<PathBuf, String> {
    // Sanitize the folder name: replace spaces and special chars
    let folder_name = options
        .name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();

    if folder_name.is_empty() {
        return Err("Instance name cannot be empty".to_string());
    }

    let instance_dir = instances_path.join(&folder_name);
    if instance_dir.exists() {
        return Err(format!(
            "An instance folder '{}' already exists",
            folder_name
        ));
    }

    // Create directory structure
    let minecraft_dir = instance_dir.join(".minecraft");
    fs::create_dir_all(&minecraft_dir)
        .map_err(|e| format!("Failed to create instance directory: {}", e))?;

    // Create mods directory
    fs::create_dir_all(minecraft_dir.join("mods"))
        .map_err(|e| format!("Failed to create mods directory: {}", e))?;

    // Write instance.cfg
    let cfg_content = format!(
        "[General]\n\
         name={}\n\
         iconKey=default\n\
         totalTimePlayed=0\n\
         instanceId={}\n\
         IntendedVersion={}\n",
        options.name,
        uuid::Uuid::new_v4().to_string(),
        options.minecraft_version
    );
    fs::write(instance_dir.join("instance.cfg"), cfg_content)
        .map_err(|e| format!("Failed to write instance.cfg: {}", e))?;

    // Build mmc-pack.json
    let mut components = Vec::new();

    // 1. LWJGL 3
    components.push(MmcComponent {
        uid: "org.lwjgl3".to_string(),
        version: "3.3.3".to_string(),
        cached_name: Some("LWJGL 3".to_string()),
        cached_version: Some("3.3.3".to_string()),
        dependency_only: Some(true),
        cached_requires: None,
        important: None,
    });

    // 2. Minecraft
    components.push(MmcComponent {
        uid: "net.minecraft".to_string(),
        version: options.minecraft_version.clone(),
        cached_name: Some("Minecraft".to_string()),
        cached_version: Some(options.minecraft_version.clone()),
        important: Some(true),
        cached_requires: Some(vec![MmcRequirement {
            uid: "org.lwjgl3".to_string(),
            suggests: Some("3.3.3".to_string()),
            equals: None,
        }]),
        dependency_only: None,
    });

    // 3. Mod Loader
    match options.mod_loader {
        ModLoader::Fabric => {
            components.push(MmcComponent {
                uid: "net.fabricmc.intermediary".to_string(),
                version: options.minecraft_version.clone(),
                cached_name: Some("Intermediary Mappings".to_string()),
                cached_version: Some(options.minecraft_version.clone()),
                dependency_only: Some(true),
                cached_requires: Some(vec![MmcRequirement {
                    uid: "net.minecraft".to_string(),
                    equals: Some(options.minecraft_version.clone()),
                    suggests: None,
                }]),
                important: None,
            });
            let loader_ver = options.loader_version.as_deref().unwrap_or("0.16.10");
            components.push(MmcComponent {
                uid: "net.fabricmc.fabric-loader".to_string(),
                version: loader_ver.to_string(),
                cached_name: Some("Fabric Loader".to_string()),
                cached_version: Some(loader_ver.to_string()),
                cached_requires: Some(vec![MmcRequirement {
                    uid: "net.fabricmc.intermediary".to_string(),
                    equals: None,
                    suggests: None,
                }]),
                dependency_only: None,
                important: None,
            });
        }
        ModLoader::Forge => {
            let loader_ver = options.loader_version.as_deref().unwrap_or("54.1.2");
            components.push(MmcComponent {
                uid: "net.minecraftforge".to_string(),
                version: loader_ver.to_string(),
                cached_name: Some("Forge".to_string()),
                cached_version: Some(loader_ver.to_string()),
                cached_requires: None,
                dependency_only: None,
                important: None,
            });
        }
        ModLoader::Quilt => {
            components.push(MmcComponent {
                uid: "net.fabricmc.intermediary".to_string(),
                version: options.minecraft_version.clone(),
                cached_name: Some("Intermediary Mappings".to_string()),
                cached_version: Some(options.minecraft_version.clone()),
                dependency_only: Some(true),
                cached_requires: Some(vec![MmcRequirement {
                    uid: "net.minecraft".to_string(),
                    equals: Some(options.minecraft_version.clone()),
                    suggests: None,
                }]),
                important: None,
            });
            let loader_ver = options.loader_version.as_deref().unwrap_or("0.26.3");
            components.push(MmcComponent {
                uid: "org.quiltmc.quilt-loader".to_string(),
                version: loader_ver.to_string(), // Dummy version for now
                cached_name: Some("Quilt Loader".to_string()),
                cached_version: Some(loader_ver.to_string()),
                cached_requires: Some(vec![MmcRequirement {
                    uid: "net.fabricmc.intermediary".to_string(),
                    equals: None,
                    suggests: None,
                }]),
                dependency_only: None,
                important: None,
            });
        }
        ModLoader::NeoForge => {
            let loader_ver = options.loader_version.as_deref().unwrap_or("21.4.156");
            components.push(MmcComponent {
                uid: "net.neoforged".to_string(),
                version: loader_ver.to_string(),
                cached_name: Some("NeoForge".to_string()),
                cached_version: Some(loader_ver.to_string()),
                cached_requires: None,
                dependency_only: None,
                important: None,
            });
        }
        ModLoader::None => {}
    }

    let pack = MmcPack {
        components,
        format_version: 1,
    };

    let pack_str = serde_json::to_string_pretty(&pack)
        .map_err(|e| format!("Failed to serialize mmc-pack.json: {}", e))?;
    fs::write(instance_dir.join("mmc-pack.json"), pack_str)
        .map_err(|e| format!("Failed to write mmc-pack.json: {}", e))?;

    Ok(instance_dir)
}

// ---------------------------------------------------------------------------
// Instance modification helpers
// ---------------------------------------------------------------------------

/// All known mod loader UIDs used to identify loader components in mmc-pack.json.
const LOADER_UIDS: &[&str] = &[
    "net.fabricmc.fabric-loader",
    "net.minecraftforge",
    "org.quiltmc.quilt-loader",
    "net.neoforged",
    "net.fabricmc.intermediary",
];

pub fn is_loader_component(uid: &str) -> bool {
    LOADER_UIDS.iter().any(|&u| uid == u)
}

pub fn get_minecraft_dir(instance_path: &Path) -> PathBuf {
    if instance_path.join(".minecraft").is_dir() {
        instance_path.join(".minecraft")
    } else if instance_path.join("minecraft").is_dir() {
        instance_path.join("minecraft")
    } else {
        instance_path.to_path_buf()
    }
}

/// Reads `mmc-pack.json` from an instance directory as a JSON Value.
/// Returns the parsed value, preserving all fields for round-trip editing.
fn read_pack(instance_path: &Path) -> Result<MmcPack, String> {
    let pack_path = instance_path.join("mmc-pack.json");
    let content = fs::read_to_string(&pack_path)
        .map_err(|e| format!("Failed to read mmc-pack.json: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse mmc-pack.json: {}", e))
}

/// Writes a JSON Value back to `mmc-pack.json` in the instance directory.
fn write_pack(instance_path: &Path, pack: &MmcPack) -> Result<(), String> {
    let pack_path = instance_path.join("mmc-pack.json");
    let content = serde_json::to_string_pretty(pack)
        .map_err(|e| format!("Failed to serialize mmc-pack.json: {}", e))?;
    fs::write(pack_path, content).map_err(|e| format!("Failed to write mmc-pack.json: {}", e))
}

/// Updates `instance.cfg` by replacing (or inserting) a key=value pair in the
/// [General] section. Preserves all other content.
pub fn update_cfg_key(instance_path: &Path, key: &str, value: &str) -> Result<(), String> {
    let cfg_path = instance_path.join("instance.cfg");
    let content =
        fs::read_to_string(&cfg_path).map_err(|e| format!("Failed to read instance.cfg: {}", e))?;

    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let mut found = false;
    let mut in_general = false;
    let target = format!("{}={}", key, value);

    for line in lines.iter_mut() {
        let trimmed = line.trim();
        if trimmed == "[General]" {
            in_general = true;
        } else if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_general = false;
        } else if in_general {
            if let Some((k, _)) = trimmed.split_once('=') {
                if k.trim() == key {
                    *line = target.clone();
                    found = true;
                }
            }
        }
    }

    // If the key wasn't found, insert it after [General]
    if !found {
        if let Some(pos) = lines.iter().position(|l| l.trim() == "[General]") {
            lines.insert(pos + 1, target);
        } else {
            // No [General] section exists — create one
            lines.push("[General]".to_string());
            lines.push(target);
        }
    }

    let new_content = lines.join("\n");
    fs::write(&cfg_path, &new_content).map_err(|e| format!("Failed to write instance.cfg: {}", e))
}

/// Removes a key from the [General] section of instance.cfg.
fn remove_cfg_key(instance_path: &Path, key: &str) -> Result<(), String> {
    let cfg_path = instance_path.join("instance.cfg");
    let content =
        fs::read_to_string(&cfg_path).map_err(|e| format!("Failed to read instance.cfg: {}", e))?;

    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let mut in_general = false;
    let mut removed = false;

    lines.retain(|line| {
        let trimmed = line.trim();
        if trimmed == "[General]" {
            in_general = true;
            true
        } else if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_general = false;
            true
        } else if in_general {
            if let Some((k, _)) = trimmed.split_once('=') {
                if k.trim() == key {
                    removed = true;
                    return false;
                }
            }
            true
        } else {
            true
        }
    });

    if removed {
        let new_content = lines.join("\n");
        fs::write(&cfg_path, &new_content)
            .map_err(|e| format!("Failed to write instance.cfg: {}", e))?;
    }
    Ok(())
}

/// Changes the Minecraft version of an existing instance.
/// Updates both `instance.cfg` (IntendedVersion) and `mmc-pack.json`
/// (the `net.minecraft` component's cachedVersion).
pub fn set_minecraft_version(instance_path: &Path, new_version: &str) -> Result<(), String> {
    // 1. Update instance.cfg
    update_cfg_key(instance_path, "IntendedVersion", new_version)?;

    // 2. Update mmc-pack.json
    let mut pack = read_pack(instance_path)?;

    let mut found_mc = false;
    for comp in pack.components.iter_mut() {
        // Update Minecraft
        if comp.uid == "net.minecraft" {
            comp.version = new_version.to_string();
            comp.cached_version = Some(new_version.to_string());
            comp.important = Some(true);
            found_mc = true;
        }
        // Update Intermediary if it exists
        if comp.uid == "net.fabricmc.intermediary" {
            comp.version = new_version.to_string();
            comp.cached_version = Some(new_version.to_string());
            if let Some(ref mut reqs) = comp.cached_requires {
                for req in reqs.iter_mut() {
                    if req.uid == "net.minecraft" {
                        req.equals = Some(new_version.to_string());
                    }
                }
            }
        }
    }

    if !found_mc {
        // Fallback: should not really happen with standard packs
        pack.components.insert(
            0,
            MmcComponent {
                uid: "net.minecraft".to_string(),
                version: new_version.to_string(),
                cached_name: Some("Minecraft".to_string()),
                cached_version: Some(new_version.to_string()),
                important: Some(true),
                cached_requires: Some(vec![MmcRequirement {
                    uid: "org.lwjgl3".to_string(),
                    suggests: Some("3.3.3".to_string()),
                    equals: None,
                }]),
                dependency_only: None,
            },
        );
    }

    write_pack(instance_path, &pack)
}

/// Sets (or replaces) the mod loader for an existing instance.
/// If `loader` is `ModLoader::None`, any existing loader component is removed.
/// Otherwise, any existing loader is replaced with the new one.
/// Only modifies `mmc-pack.json`.
pub fn set_mod_loader(instance_path: &Path, loader: &ModLoader) -> Result<(), String> {
    let default_version = match loader {
        ModLoader::Fabric => "0.16.10",
        ModLoader::Forge => "54.1.2",
        ModLoader::Quilt => "0.26.3",
        ModLoader::NeoForge => "21.4.156",
        ModLoader::None => "",
    };
    set_mod_loader_with_version(instance_path, loader, default_version)
}

/// Sets (or replaces) the mod loader for an existing instance, using a specific version.
/// Unlike `set_mod_loader`, this uses the caller-supplied version string instead of hardcoded defaults.
pub fn set_mod_loader_with_version(
    instance_path: &Path,
    loader: &ModLoader,
    version: &str,
) -> Result<(), String> {
    let mut pack = read_pack(instance_path)?;

    let mc_version = pack
        .components
        .iter()
        .find(|c| c.uid == "net.minecraft")
        .map(|c| c.version.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    // Remove any existing loader components
    pack.components
        .retain(|comp| !is_loader_component(&comp.uid));

    match loader {
        ModLoader::Fabric => {
            pack.components.push(MmcComponent {
                uid: "net.fabricmc.intermediary".to_string(),
                version: mc_version.clone(),
                cached_name: Some("Intermediary Mappings".to_string()),
                cached_version: Some(mc_version.clone()),
                dependency_only: Some(true),
                cached_requires: Some(vec![MmcRequirement {
                    uid: "net.minecraft".to_string(),
                    equals: Some(mc_version.clone()),
                    suggests: None,
                }]),
                important: None,
            });
            pack.components.push(MmcComponent {
                uid: "net.fabricmc.fabric-loader".to_string(),
                version: version.to_string(),
                cached_name: Some("Fabric Loader".to_string()),
                cached_version: Some(version.to_string()),
                cached_requires: Some(vec![MmcRequirement {
                    uid: "net.fabricmc.intermediary".to_string(),
                    equals: None,
                    suggests: None,
                }]),
                dependency_only: None,
                important: None,
            });
        }
        ModLoader::Forge => {
            pack.components.push(MmcComponent {
                uid: "net.minecraftforge".to_string(),
                version: version.to_string(),
                cached_name: Some("Forge".to_string()),
                cached_version: Some(version.to_string()),
                cached_requires: None,
                dependency_only: None,
                important: None,
            });
        }
        ModLoader::Quilt => {
            pack.components.push(MmcComponent {
                uid: "net.fabricmc.intermediary".to_string(),
                version: mc_version.clone(),
                cached_name: Some("Intermediary Mappings".to_string()),
                cached_version: Some(mc_version.clone()),
                dependency_only: Some(true),
                cached_requires: Some(vec![MmcRequirement {
                    uid: "net.minecraft".to_string(),
                    equals: Some(mc_version.clone()),
                    suggests: None,
                }]),
                important: None,
            });
            pack.components.push(MmcComponent {
                uid: "org.quiltmc.quilt-loader".to_string(),
                version: version.to_string(),
                cached_name: Some("Quilt Loader".to_string()),
                cached_version: Some(version.to_string()),
                cached_requires: Some(vec![MmcRequirement {
                    uid: "net.fabricmc.intermediary".to_string(),
                    equals: None,
                    suggests: None,
                }]),
                dependency_only: None,
                important: None,
            });
        }
        ModLoader::NeoForge => {
            pack.components.push(MmcComponent {
                uid: "net.neoforged".to_string(),
                version: version.to_string(),
                cached_name: Some("NeoForge".to_string()),
                cached_version: Some(version.to_string()),
                cached_requires: None,
                dependency_only: None,
                important: None,
            });
        }
        ModLoader::None => {}
    }

    write_pack(instance_path, &pack)
}

/// Convenience: removes the mod loader from an instance (sets to None).
pub fn remove_mod_loader(instance_path: &Path) -> Result<(), String> {
    set_mod_loader(instance_path, &ModLoader::None)
}

/// Removes an item (file or directory) from an instance subfolder.
pub fn remove_instance_item(
    instance_path: &Path,
    subfolder: &str,
    filename: &str,
) -> Result<(), String> {
    // Safety check: Prevent parent directory traversal and absolute paths
    if filename.contains("..") || std::path::Path::new(filename).is_absolute() {
        return Err(format!(
            "Safety check failed: Invalid filename '{}'",
            filename
        ));
    }
    if subfolder.contains("..") || std::path::Path::new(subfolder).is_absolute() {
        return Err(format!(
            "Safety check failed: Invalid subfolder '{}'",
            subfolder
        ));
    }
    if filename.trim().is_empty() {
        return Err("Safety check failed: Filename cannot be empty".to_string());
    }

    let minecraft_dir = get_minecraft_dir(instance_path);
    if minecraft_dir == instance_path {
        return Err("Could not find minecraft directory".to_string());
    }
    let target_path = minecraft_dir.join(subfolder).join(filename);
    if target_path.exists() {
        if target_path.is_dir() {
            fs::remove_dir_all(target_path)
                .map_err(|e| format!("Failed to remove directory: {}", e))
        } else {
            fs::remove_file(target_path).map_err(|e| format!("Failed to remove file: {}", e))
        }
    } else {
        Err(format!("Item {} not found in {}", filename, subfolder))
    }
}

pub fn remove_mod(instance_path: &Path, filename: &str) -> Result<(), String> {
    remove_instance_item(instance_path, "mods", filename)
}

/// Copies a file or directory to an instance subfolder.
pub fn add_instance_item(
    instance_path: &Path,
    subfolder: &str,
    source_path: &Path,
) -> Result<(), String> {
    let minecraft_dir = {
        let dir = get_minecraft_dir(instance_path);
        if dir == instance_path {
            instance_path.join(".minecraft")
        } else {
            dir
        }
    };
    let dest_dir = minecraft_dir.join(subfolder);
    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir)
            .map_err(|e| format!("Failed to create directory {}: {}", subfolder, e))?;
    }

    let filename = source_path.file_name().ok_or("Invalid source path")?;
    let dest_path = dest_dir.join(filename);

    if source_path.is_dir() {
        copy_dir_all(source_path, &dest_path)
            .map_err(|e| format!("Failed to copy directory: {}", e))
    } else {
        fs::copy(source_path, dest_path).map_err(|e| format!("Failed to copy file: {}", e))?;
        Ok(())
    }
}

pub fn add_mod(instance_path: &Path, source_path: &Path) -> Result<(), String> {
    add_instance_item(instance_path, "mods", source_path)
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// Removes a component from mmc-pack.json by its UID.
pub fn remove_component(instance_path: &Path, uid: &str) -> Result<(), String> {
    let mut pack = read_pack(instance_path)?;
    pack.components.retain(|comp| comp.uid != uid);
    write_pack(instance_path, &pack)
}

/// Renames the instance in instance.cfg.
/// Does NOT rename the folder to avoid breaking paths/external references.
pub fn rename_instance(instance_path: &Path, new_name: &str) -> Result<(), String> {
    update_cfg_key(instance_path, "name", new_name)
}

fn make_writable_recursive(path: &Path) -> std::io::Result<()> {
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            make_writable_recursive(&entry.path())?;
        }
    }
    if let Ok(metadata) = fs::metadata(path) {
        let mut perms = metadata.permissions();
        if perms.readonly() {
            perms.set_readonly(false);
            let _ = fs::set_permissions(path, perms);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = metadata.permissions();
            let mode = perms.mode();
            if mode & 0o200 == 0 {
                perms.set_mode(mode | 0o200);
                let _ = fs::set_permissions(path, perms);
            }
        }
    }
    Ok(())
}

/// Deletes the entire instance folder.
pub fn delete_instance(instance_path: &Path) -> Result<(), String> {
    if !instance_path.join("instance.cfg").exists() {
        return Err(format!("Safety check failed: The path {} does not contain an instance.cfg and may not be a valid instance.", instance_path.display()));
    }
    
    // Ensure all directories and files are writable recursively before deletion
    let _ = make_writable_recursive(instance_path);

    fs::remove_dir_all(instance_path).map_err(|e| format!("Failed to delete instance: {}", e))
}

pub fn set_instance_java(instance_path: &Path, java_path: &Path) -> Result<(), String> {
    update_cfg_key(instance_path, "JavaPath", &java_path.to_string_lossy())
}

pub fn remove_instance_java(instance_path: &Path) -> Result<(), String> {
    remove_cfg_key(instance_path, "JavaPath")
}

pub fn set_instance_performance_tweak(
    instance_path: &Path,
    key: &str,
    value: bool,
) -> Result<(), String> {
    update_cfg_key(instance_path, key, if value { "true" } else { "false" })
}

pub fn set_component_version(
    instance_path: &Path,
    uid: &str,
    new_version: &str,
) -> Result<(), String> {
    let mut pack = read_pack(instance_path)?;
    for comp in &mut pack.components {
        if comp.uid == uid {
            comp.version = new_version.to_string();
            comp.cached_version = Some(new_version.to_string());
        }
    }

    // special case for minecraft: update instance.cfg IntendedVersion
    if uid == "net.minecraft" {
        let _ = update_cfg_key(instance_path, "IntendedVersion", new_version);
        // Also update intermediary version if it exists
        for comp in &mut pack.components {
            if comp.uid == "net.fabricmc.intermediary" {
                comp.version = new_version.to_string();
                comp.cached_version = Some(new_version.to_string());
            }
        }
    }

    write_pack(instance_path, &pack)
}

pub fn update_instance_playtime(
    instance_path: &Path,
    additional_seconds: u64,
) -> Result<u64, String> {
    let cfg_path = instance_path.join("instance.cfg");
    let content =
        fs::read_to_string(&cfg_path).map_err(|e| format!("Failed to read instance.cfg: {}", e))?;

    let mut current_playtime = 0u64;
    for line in content.lines() {
        if let Some((key, value)) = line.split_once('=') {
            if key.trim() == "totalTimePlayed" {
                current_playtime = value.trim().parse().unwrap_or(0);
                break;
            }
        }
    }

    let new_playtime = current_playtime + additional_seconds;
    update_cfg_key(instance_path, "totalTimePlayed", &new_playtime.to_string())?;
    Ok(new_playtime)
}

pub fn toggle_mod_enabled(
    instance_path: &Path,
    mod_filename: &str,
    enable: bool,
) -> Result<String, String> {
    let minecraft_dir = get_minecraft_dir(instance_path);
    if minecraft_dir == instance_path {
        return Err("Could not find minecraft directory".to_string());
    }
    let mods_dir = minecraft_dir.join("mods");

    let current_path = mods_dir.join(mod_filename);
    if !current_path.exists() {
        return Err(format!("Mod file {} not found", mod_filename));
    }

    let new_filename = if enable {
        if mod_filename.ends_with(".disabled") {
            mod_filename.trim_end_matches(".disabled").to_string()
        } else {
            return Ok(mod_filename.to_string());
        }
    } else {
        if !mod_filename.ends_with(".disabled") {
            format!("{}.disabled", mod_filename)
        } else {
            return Ok(mod_filename.to_string());
        }
    };

    let new_path = mods_dir.join(&new_filename);
    fs::rename(current_path, new_path).map_err(|e| format!("Failed to rename mod file: {}", e))?;

    Ok(new_filename)
}

pub fn rename_world(instance_path: &Path, folder_name: &str, new_name: &str) -> Result<(), String> {
    let minecraft_dir = get_minecraft_dir(instance_path);

    let worlds_dir = minecraft_dir.join("saves");
    let world_path = worlds_dir.join(folder_name);
    let level_dat_path = world_path.join("level.dat");

    if !level_dat_path.exists() {
        return Err("World level.dat not found".to_string());
    }

    // Load, modify, and save level.dat
    let file = fs::File::open(&level_dat_path).map_err(|e| e.to_string())?;
    let mut decoder = flate2::read::GzDecoder::new(file);
    let mut bytes = Vec::new();
    use std::io::Read;
    decoder.read_to_end(&mut bytes).map_err(|e| e.to_string())?;

    let mut value: fastnbt::Value = fastnbt::from_bytes(&bytes).map_err(|e| e.to_string())?;

    // NBT path: Data -> LevelName
    if let fastnbt::Value::Compound(root) = &mut value {
        if let Some(fastnbt::Value::Compound(data)) = root.get_mut("Data") {
            data.insert(
                "LevelName".to_string(),
                fastnbt::Value::String(new_name.to_string()),
            );
        }
    }

    let new_bytes = fastnbt::to_bytes(&value).map_err(|e| e.to_string())?;
    let out_file = fs::File::create(&level_dat_path).map_err(|e| e.to_string())?;
    let mut encoder = flate2::write::GzEncoder::new(out_file, flate2::Compression::default());
    use std::io::Write;
    encoder.write_all(&new_bytes).map_err(|e| e.to_string())?;
    encoder.finish().map_err(|e| e.to_string())?;

    Ok(())
}

pub fn move_world(
    source_instance_path: &Path,
    target_instance_path: &Path,
    folder_name: &str,
) -> Result<(), String> {
    let source_minecraft_dir = get_minecraft_dir(source_instance_path);

    let target_minecraft_dir = get_minecraft_dir(target_instance_path);

    let source_path = source_minecraft_dir.join("saves").join(folder_name);
    let target_saves_dir = target_minecraft_dir.join("saves");
    let target_path = target_saves_dir.join(folder_name);

    if !source_path.exists() {
        return Err("Source world folder not found".to_string());
    }
    if target_path.exists() {
        return Err("Target instance already has a world with this folder name".to_string());
    }

    if !target_saves_dir.exists() {
        fs::create_dir_all(&target_saves_dir).map_err(|e| e.to_string())?;
    }

    fs::rename(source_path, target_path)
        .map_err(|e| format!("Failed to move world directory: {}", e))
}
