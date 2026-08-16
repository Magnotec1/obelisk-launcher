use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use serde::{Deserialize, Serialize};
use crate::backend::instance::manager::{ModLoader, create_instance, CreateInstanceOptions, get_minecraft_dir};

const MODRINTH_API_BASE: &str = "https://api.modrinth.com/v2";
const USER_AGENT: &str = "obelisk-launcher-rs (github.com/magnotec/obelisk-launcher)";

pub(crate) static HTTP_CLIENT: std::sync::LazyLock<reqwest::blocking::Client> = std::sync::LazyLock::new(|| {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .expect("Failed to build HTTP client")
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackInfo {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub author: Option<String>,
    pub downloads: u64,
    pub follows: u64,
    pub categories: Vec<String>,
    pub latest_version_name: Option<String>,
    pub latest_version_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackDetails {
    pub info: ModpackInfo,
    pub body: Option<String>,
    pub screenshots: Vec<String>,
    pub wiki_url: Option<String>,
    pub discord_url: Option<String>,
    pub source_url: Option<String>,
    pub client_side: String,
    pub server_side: String,
    pub license_name: Option<String>,
    pub loaders: Vec<String>,
    pub game_versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackVersionInfo {
    pub id: String,
    pub name: String,
    pub version_number: String,
    pub download_url: String,
    pub filename: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub date_published: String,
}

pub trait ModpackSource: Send + Sync {
    fn name(&self) -> &'static str;
    fn get_popular(&self, limit: u32, offset: u32) -> Result<Vec<ModpackInfo>, String>;
    fn search(&self, query: &str, limit: u32, offset: u32, game_version: Option<&str>, loader: Option<ModLoader>) -> Result<Vec<ModpackInfo>, String>;
    fn get_details(&self, id_or_slug: &str) -> Result<ModpackDetails, String>;
    fn get_versions(&self, id_or_slug: &str) -> Result<Vec<ModpackVersionInfo>, String>;
    fn install(&self, name: &str, version: &ModpackVersionInfo, instances_path: &Path, progress_callback: Box<dyn Fn(String, f32) + Send + Sync + 'static>) -> Result<PathBuf, String>;
}

pub struct ModrinthSource;

impl ModpackSource for ModrinthSource {
    fn name(&self) -> &'static str {
        "Modrinth"
    }

    fn get_popular(&self, limit: u32, offset: u32) -> Result<Vec<ModpackInfo>, String> {
        let facets = "[[\"project_type:modpack\"]]";
        let url = format!(
            "{}/search?query=&facets={}&index=downloads&limit={}&offset={}",
            MODRINTH_API_BASE,
            urlencoding::encode(facets),
            limit,
            offset
        );
        let res = HTTP_CLIENT.get(&url).send().map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            return Err(format!("Modrinth API error: Status {}", res.status()));
        }
        let search_res: ModrinthSearchResult = res.json().map_err(|e| e.to_string())?;
        
        Ok(search_res.hits.into_iter().map(|hit| ModpackInfo {
            id: hit.id,
            slug: hit.slug,
            title: hit.title,
            description: hit.description,
            icon_url: hit.icon_url,
            author: hit.author,
            downloads: hit.downloads,
            follows: hit.follows.or(hit.followers).unwrap_or(0),
            categories: hit.categories.unwrap_or_default(),
            latest_version_name: None,
            latest_version_id: hit.latest_version,
        }).collect())
    }

    fn search(&self, query: &str, limit: u32, offset: u32, game_version: Option<&str>, loader: Option<ModLoader>) -> Result<Vec<ModpackInfo>, String> {
        let mut facets_list = vec!["[\"project_type:modpack\"]".to_string()];
        if let Some(v) = game_version {
            facets_list.push(format!("[\"versions:{}\"]", v));
        }
        if let Some(ref l) = loader {
            if *l != ModLoader::None {
                facets_list.push(format!("[\"categories:{}\"]", l.as_str().to_lowercase()));
            }
        }
        let facets_str = format!("[{}]", facets_list.join(","));
        let index_sort = if query.is_empty() { "&index=downloads" } else { "" };

        let url = format!(
            "{}/search?query={}&facets={}{}&limit={}&offset={}",
            MODRINTH_API_BASE,
            urlencoding::encode(query),
            urlencoding::encode(&facets_str),
            index_sort,
            limit,
            offset
        );
        let res = HTTP_CLIENT.get(&url).send().map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            return Err(format!("Modrinth API error: Status {}", res.status()));
        }
        let search_res: ModrinthSearchResult = res.json().map_err(|e| e.to_string())?;
        
        Ok(search_res.hits.into_iter().map(|hit| ModpackInfo {
            id: hit.id,
            slug: hit.slug,
            title: hit.title,
            description: hit.description,
            icon_url: hit.icon_url,
            author: hit.author,
            downloads: hit.downloads,
            follows: hit.follows.or(hit.followers).unwrap_or(0),
            categories: hit.categories.unwrap_or_default(),
            latest_version_name: None,
            latest_version_id: hit.latest_version,
        }).collect())
    }

    fn get_details(&self, id_or_slug: &str) -> Result<ModpackDetails, String> {
        let url = format!("{}/project/{}", MODRINTH_API_BASE, id_or_slug);
        let res = HTTP_CLIENT.get(&url).send().map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            return Err(format!("Modrinth API error: Status {}", res.status()));
        }
        let raw: ModrinthProjectDetails = res.json().map_err(|e| e.to_string())?;
        
        let screenshots = raw.gallery.unwrap_or_default().into_iter().map(|img| img.url).collect();
        
        let info = ModpackInfo {
            id: raw.id,
            slug: raw.slug,
            title: raw.title,
            description: raw.description,
            icon_url: raw.icon_url,
            author: None,
            downloads: raw.downloads,
            follows: raw.follows.or(raw.followers).unwrap_or(0),
            categories: raw.categories.unwrap_or_default(),
            latest_version_name: None,
            latest_version_id: None,
        };

        let license_name = match &raw.license {
            Some(serde_json::Value::String(s)) => Some(s.clone()),
            Some(serde_json::Value::Object(obj)) => obj
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            _ => None,
        };
        
        Ok(ModpackDetails {
            info,
            body: raw.body,
            screenshots,
            wiki_url: raw.wiki_url,
            discord_url: raw.discord_url,
            source_url: raw.source_url,
            client_side: raw.client_side,
            server_side: raw.server_side,
            license_name,
            loaders: raw.loaders.unwrap_or_default(),
            game_versions: raw.game_versions.unwrap_or_default(),
        })
    }

    fn get_versions(&self, id_or_slug: &str) -> Result<Vec<ModpackVersionInfo>, String> {
        let url = format!("{}/project/{}/version", MODRINTH_API_BASE, id_or_slug);
        let res = HTTP_CLIENT.get(&url).send().map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            return Err(format!("Modrinth API error: Status {}", res.status()));
        }
        let raw_versions: Vec<ModrinthRawVersion> = res.json().map_err(|e| e.to_string())?;
        
        Ok(raw_versions.into_iter().filter_map(|v| {
            let primary_file = v.files.first()?;
            Some(ModpackVersionInfo {
                id: v.id,
                name: v.name,
                version_number: v.version_number,
                download_url: primary_file.url.clone(),
                filename: primary_file.filename.clone(),
                game_versions: v.game_versions,
                loaders: v.loaders,
                date_published: v.date_published,
            })
        }).collect())
    }

    fn install(&self, name: &str, version: &ModpackVersionInfo, instances_path: &Path, progress_callback: Box<dyn Fn(String, f32) + Send + Sync + 'static>) -> Result<PathBuf, String> {
        install_mrpack(name, &version.download_url, instances_path, &*progress_callback, &|_, _, _| {})
    }
}

// Internal JSON structures
#[derive(Deserialize, Debug)]
struct ModrinthSearchHit {
    #[serde(alias = "project_id")]
    id: String,
    slug: String,
    title: String,
    description: String,
    icon_url: Option<String>,
    author: Option<String>,
    downloads: u64,
    follows: Option<u64>,
    followers: Option<u64>,
    categories: Option<Vec<String>>,
    latest_version: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ModrinthSearchResult {
    hits: Vec<ModrinthSearchHit>,
}

#[derive(Deserialize, Debug)]
struct ModrinthGalleryImage {
    url: String,
}

#[derive(Deserialize, Debug)]
struct ModrinthProjectDetails {
    id: String,
    slug: String,
    title: String,
    description: String,
    icon_url: Option<String>,
    body: Option<String>,
    gallery: Option<Vec<ModrinthGalleryImage>>,
    wiki_url: Option<String>,
    discord_url: Option<String>,
    source_url: Option<String>,
    client_side: String,
    server_side: String,
    downloads: u64,
    followers: Option<u64>,
    follows: Option<u64>,
    categories: Option<Vec<String>>,
    #[serde(default)]
    license: Option<serde_json::Value>,
    #[serde(default)]
    loaders: Option<Vec<String>>,
    #[serde(default)]
    game_versions: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
struct ModrinthRawVersionFile {
    url: String,
    filename: String,
}

#[derive(Deserialize, Debug)]
struct ModrinthRawVersion {
    id: String,
    name: String,
    version_number: String,
    files: Vec<ModrinthRawVersionFile>,
    game_versions: Vec<String>,
    loaders: Vec<String>,
    date_published: String,
}

#[derive(Deserialize, Debug)]
struct MrpackIndex {
    #[serde(rename = "formatVersion")]
    format_version: u32,
    game: String,
    dependencies: HashMap<String, String>,
    files: Vec<MrpackFile>,
}

#[derive(Deserialize, Debug)]
struct MrpackFile {
    path: String,
    downloads: Vec<String>,
    #[serde(default)]
    hashes: Option<HashMap<String, String>>,
    env: Option<MrpackEnv>,
    #[serde(rename = "fileSize")]
    file_size: Option<u64>,
}

#[derive(Deserialize, Debug)]
struct MrpackEnv {
    client: String,
}

pub(crate) fn install_mrpack(
    name: &str,
    download_url: &str,
    instances_path: &Path,
    progress_callback: &(dyn Fn(String, f32) + Send + Sync),
    item_callback: &(dyn Fn(String, crate::backend::download::manager::TaskItemStatus, crate::backend::download::manager::DownloadedItemType) + Send + Sync),
) -> Result<PathBuf, String> {
    item_callback("Modpack Archive".to_string(), crate::backend::download::manager::TaskItemStatus::Pending, crate::backend::download::manager::DownloadedItemType::MinecraftComponent);
    progress_callback("Connecting to download modpack archive...".to_string(), 0.05);

    // Create a temporary path in instances_path
    let temp_dir = instances_path.join(".tmp_modpack_install");
    fs::create_dir_all(&temp_dir).map_err(|e| {
        let err_msg = format!("Failed to create temporary directory: {}", e);
        item_callback("Modpack Archive".to_string(), crate::backend::download::manager::TaskItemStatus::Failed(err_msg.clone()), crate::backend::download::manager::DownloadedItemType::MinecraftComponent);
        err_msg
    })?;
    let temp_file_path = temp_dir.join("pack.mrpack");

    // Download the mrpack zip to the temporary file
    let mut response = match HTTP_CLIENT.get(download_url).send() {
        Ok(res) => {
            if !res.status().is_success() {
                let _ = fs::remove_dir_all(&temp_dir);
                let err_msg = format!("Failed to download modpack archive: Status {}", res.status());
                item_callback("Modpack Archive".to_string(), crate::backend::download::manager::TaskItemStatus::Failed(err_msg.clone()), crate::backend::download::manager::DownloadedItemType::MinecraftComponent);
                return Err(err_msg);
            }
            res
        }
        Err(e) => {
            let _ = fs::remove_dir_all(&temp_dir);
            let err_msg = format!("Failed to download modpack archive: {}", e);
            item_callback("Modpack Archive".to_string(), crate::backend::download::manager::TaskItemStatus::Failed(err_msg.clone()), crate::backend::download::manager::DownloadedItemType::MinecraftComponent);
            return Err(err_msg);
        }
    };

    let mut dest_file = match fs::File::create(&temp_file_path) {
        Ok(f) => f,
        Err(e) => {
            let _ = fs::remove_dir_all(&temp_dir);
            let err_msg = format!("Failed to create temporary file: {}", e);
            item_callback("Modpack Archive".to_string(), crate::backend::download::manager::TaskItemStatus::Failed(err_msg.clone()), crate::backend::download::manager::DownloadedItemType::MinecraftComponent);
            return Err(err_msg);
        }
    };
    if let Err(e) = std::io::copy(&mut response, &mut dest_file) {
        let _ = fs::remove_dir_all(&temp_dir);
        let err_msg = format!("Failed to write to temporary file: {}", e);
        item_callback("Modpack Archive".to_string(), crate::backend::download::manager::TaskItemStatus::Failed(err_msg.clone()), crate::backend::download::manager::DownloadedItemType::MinecraftComponent);
        return Err(err_msg);
    }

    item_callback("Modpack Archive".to_string(), crate::backend::download::manager::TaskItemStatus::Success, crate::backend::download::manager::DownloadedItemType::MinecraftComponent);

    let result = install_mrpack_archive_internal(
        &temp_file_path,
        name,
        instances_path,
        progress_callback,
        item_callback,
    );

    let _ = fs::remove_dir_all(&temp_dir);
    result
}

/// Directly install an mrpack archive from a local file path (e.g. via drag-and-drop or local file selector).
pub fn install_mrpack_from_file(
    archive_path: &Path,
    name: &str,
    instances_path: &Path,
    progress_callback: &(dyn Fn(String, f32) + Send + Sync),
    item_callback: &(dyn Fn(String, crate::backend::download::manager::TaskItemStatus, crate::backend::download::manager::DownloadedItemType) + Send + Sync),
) -> Result<PathBuf, String> {
    install_mrpack_archive_internal(
        archive_path,
        name,
        instances_path,
        progress_callback,
        item_callback,
    )
}

fn install_mrpack_archive_internal(
    archive_path: &Path,
    name: &str,
    instances_path: &Path,
    progress_callback: &(dyn Fn(String, f32) + Send + Sync),
    item_callback: &(dyn Fn(String, crate::backend::download::manager::TaskItemStatus, crate::backend::download::manager::DownloadedItemType) + Send + Sync),
) -> Result<PathBuf, String> {
    use rayon::prelude::*;

    progress_callback("Parsing modpack archive...".to_string(), 0.15);

    // Open ZIP and parse modrinth.index.json
    let file = fs::File::open(archive_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    let index: MrpackIndex = {
        let mut index_file = archive.by_name("modrinth.index.json").map_err(|_| "Could not find modrinth.index.json in modpack".to_string())?;
        serde_json::from_reader(&mut index_file).map_err(|e| format!("Failed to parse modrinth.index.json: {}", e))?
    };

    if index.format_version != 1 {
        return Err(format!("Unsupported modpack format version: {}", index.format_version));
    }
    if index.game != "minecraft" {
        return Err(format!("Unsupported game: {}", index.game));
    }

    // Determine Minecraft version and loader
    let mc_version = index.dependencies.get("minecraft").ok_or_else(|| "No minecraft version specified in modpack".to_string())?.clone();
    
    let mut loader = ModLoader::None;
    let mut loader_version = None;
    if let Some(v) = index.dependencies.get("fabric-loader") {
        loader = ModLoader::Fabric;
        loader_version = Some(v.clone());
    } else if let Some(v) = index.dependencies.get("quilt-loader") {
        loader = ModLoader::Quilt;
        loader_version = Some(v.clone());
    } else if let Some(v) = index.dependencies.get("forge") {
        loader = ModLoader::Forge;
        loader_version = Some(v.clone());
    } else if let Some(v) = index.dependencies.get("neoforge") {
        loader = ModLoader::NeoForge;
        loader_version = Some(v.clone());
    }

    item_callback("Create Instance".to_string(), crate::backend::download::manager::TaskItemStatus::Pending, crate::backend::download::manager::DownloadedItemType::MinecraftComponent);
    progress_callback("Creating instance folders...".to_string(), 0.20);

    // Call create_instance
    let options = CreateInstanceOptions {
        name: name.to_string(),
        minecraft_version: mc_version,
        mod_loader: loader,
        loader_version,
    };

    let instance_dir = match create_instance(instances_path, options) {
        Ok(dir) => {
            item_callback("Create Instance".to_string(), crate::backend::download::manager::TaskItemStatus::Success, crate::backend::download::manager::DownloadedItemType::MinecraftComponent);
            dir
        }
        Err(e) => {
            item_callback("Create Instance".to_string(), crate::backend::download::manager::TaskItemStatus::Failed(e.clone()), crate::backend::download::manager::DownloadedItemType::MinecraftComponent);
            return Err(e);
        }
    };
    let minecraft_dir = get_minecraft_dir(&instance_dir);

    item_callback("Extract Overrides".to_string(), crate::backend::download::manager::TaskItemStatus::Pending, crate::backend::download::manager::DownloadedItemType::MinecraftComponent);
    progress_callback("Extracting overrides...".to_string(), 0.25);

    // Copy overrides
    let zip_len = archive.len();
    for i in 0..zip_len {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let filename = file.name().to_string();
        if filename.starts_with("overrides/") && !filename.ends_with('/') {
            let rel_path = &filename["overrides/".len()..];
            let dest_path = minecraft_dir.join(rel_path);
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut dest_file = fs::File::create(&dest_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut dest_file).map_err(|e| e.to_string())?;
        }
    }

    // Copy client-overrides
    for i in 0..zip_len {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let filename = file.name().to_string();
        if filename.starts_with("client-overrides/") && !filename.ends_with('/') {
            let rel_path = &filename["client-overrides/".len()..];
            let dest_path = minecraft_dir.join(rel_path);
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut dest_file = fs::File::create(&dest_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut dest_file).map_err(|e| e.to_string())?;
        }
    }

    item_callback("Extract Overrides".to_string(), crate::backend::download::manager::TaskItemStatus::Success, crate::backend::download::manager::DownloadedItemType::MinecraftComponent);

    // Download external files in parallel
    let client_files: Vec<_> = index.files.into_iter().filter(|f| {
        if let Some(env) = &f.env {
            env.client != "unsupported"
        } else {
            true
        }
    }).collect();

    let total_files = client_files.len();
    let completed_count = std::sync::atomic::AtomicUsize::new(0);
    let download_errors = std::sync::Mutex::new(Vec::new());

    client_files.into_par_iter().for_each(|file_entry| {
        let file_url = match file_entry.downloads.first() {
            Some(url) => url,
            None => {
                let err_msg = format!("No download URL for file {}", file_entry.path);
                download_errors.lock().unwrap().push(err_msg);
                return;
            }
        };

        let filename = Path::new(&file_entry.path).file_name().and_then(|n| n.to_str()).unwrap_or("file").to_string();
        let dest_path = minecraft_dir.join(&file_entry.path);

        // Skip already-downloaded files for retry efficiency
        let mut needs_download = !dest_path.exists();
        if !needs_download {
            if let Ok(meta) = dest_path.metadata() {
                if let Some(expected_size) = file_entry.file_size {
                    if meta.len() != expected_size {
                        needs_download = true;
                    }
                } else if meta.len() == 0 {
                    needs_download = true;
                }
            } else {
                needs_download = true;
            }
        }

        if !needs_download {
            item_callback(filename.clone(), crate::backend::download::manager::TaskItemStatus::Success, crate::backend::download::manager::DownloadedItemType::Mod);
            let current = completed_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            let progress = 0.3 + (current as f32 / total_files as f32) * 0.7;
            progress_callback(format!("Downloaded mod {}/{} ({})", current, total_files, filename), progress);
            return;
        }

        item_callback(filename.clone(), crate::backend::download::manager::TaskItemStatus::Pending, crate::backend::download::manager::DownloadedItemType::Mod);

        if let Some(parent) = dest_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                let err_msg = format!("Failed to create parent directory: {}", e);
                item_callback(filename.clone(), crate::backend::download::manager::TaskItemStatus::Failed(err_msg.clone()), crate::backend::download::manager::DownloadedItemType::Mod);
                download_errors.lock().unwrap().push(format!("{}: {}", filename, err_msg));
                return;
            }
        }

        match HTTP_CLIENT.get(file_url).send() {
            Ok(mut res) => {
                if res.status().is_success() {
                    match fs::File::create(&dest_path) {
                        Ok(mut out) => {
                            if let Err(e) = std::io::copy(&mut res, &mut out) {
                                let err_msg = format!("Failed to write file: {}", e);
                                item_callback(filename.clone(), crate::backend::download::manager::TaskItemStatus::Failed(err_msg.clone()), crate::backend::download::manager::DownloadedItemType::Mod);
                                download_errors.lock().unwrap().push(format!("{}: {}", filename, err_msg));
                                return;
                            } else {
                                // Validate SHA1 if provided
                                if let Some(ref hashes) = file_entry.hashes {
                                    if let Some(expected_sha1) = hashes.get("sha1") {
                                        if let Ok(bytes) = fs::read(&dest_path) {
                                            use sha1::Digest;
                                            let mut hasher = sha1::Sha1::new();
                                            hasher.update(&bytes);
                                            let actual_sha1 = hex::encode(hasher.finalize());
                                            if !actual_sha1.eq_ignore_ascii_case(expected_sha1) {
                                                let err_msg = format!("SHA1 hash mismatch for {}", filename);
                                                item_callback(filename.clone(), crate::backend::download::manager::TaskItemStatus::Failed(err_msg.clone()), crate::backend::download::manager::DownloadedItemType::Mod);
                                                download_errors.lock().unwrap().push(err_msg);
                                                return;
                                            }
                                        }
                                    }
                                }
                                item_callback(filename.clone(), crate::backend::download::manager::TaskItemStatus::Success, crate::backend::download::manager::DownloadedItemType::Mod);
                            }
                        }
                        Err(e) => {
                            let err_msg = format!("Failed to create file: {}", e);
                            item_callback(filename.clone(), crate::backend::download::manager::TaskItemStatus::Failed(err_msg.clone()), crate::backend::download::manager::DownloadedItemType::Mod);
                            download_errors.lock().unwrap().push(format!("{}: {}", filename, err_msg));
                            return;
                        }
                    }
                } else {
                    let err_msg = format!("HTTP Status {}", res.status());
                    item_callback(filename.clone(), crate::backend::download::manager::TaskItemStatus::Failed(err_msg.clone()), crate::backend::download::manager::DownloadedItemType::Mod);
                    download_errors.lock().unwrap().push(format!("{}: {}", filename, err_msg));
                    return;
                }
            }
            Err(e) => {
                let err_msg = e.to_string();
                item_callback(filename.clone(), crate::backend::download::manager::TaskItemStatus::Failed(err_msg.clone()), crate::backend::download::manager::DownloadedItemType::Mod);
                download_errors.lock().unwrap().push(format!("{}: {}", filename, err_msg));
                return;
            }
        }

        let current = completed_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        let progress = 0.3 + (current as f32 / total_files as f32) * 0.7;
        progress_callback(format!("Downloaded mod {}/{} ({})", current, total_files, filename), progress);
    });

    let errors = download_errors.into_inner().unwrap();
    if !errors.is_empty() {
        return Err(format!("Failed to download some modpack files: {}", errors.join("; ")));
    }

    progress_callback("Installation complete!".to_string(), 1.0);

    Ok(instance_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mrpack_index_deserialization() {
        let json = r#"{
            "formatVersion": 1,
            "game": "minecraft",
            "versionId": "1.0.0",
            "name": "Test Pack",
            "dependencies": {
                "minecraft": "1.20.4",
                "fabric-loader": "0.15.7"
            },
            "files": [
                {
                    "path": "mods/test-mod.jar",
                    "hashes": {
                        "sha1": "da39a3ee5e6b4b0d3255bfef95601890afd80709"
                    },
                    "downloads": [
                        "https://example.com/test-mod.jar"
                    ],
                    "fileSize": 1024
                }
            ]
        }"#;

        let index: MrpackIndex = serde_json::from_str(json).expect("Failed to parse MrpackIndex");
        assert_eq!(index.format_version, 1);
        assert_eq!(index.game, "minecraft");
        assert_eq!(index.dependencies.get("minecraft").unwrap(), "1.20.4");
        assert_eq!(index.dependencies.get("fabric-loader").unwrap(), "0.15.7");
        assert_eq!(index.files.len(), 1);
        assert_eq!(index.files[0].path, "mods/test-mod.jar");
        assert_eq!(index.files[0].file_size, Some(1024));
    }
}
