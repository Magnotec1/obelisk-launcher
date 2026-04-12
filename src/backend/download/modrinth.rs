use crate::backend::instance::manager::ModLoader;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

const MODRINTH_API_BASE: &str = "https://api.modrinth.com/v2";
const USER_AGENT: &str = "minecraft-launcher-rs (github.com/magnotec/minecraft-manager)";
const CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes

// ---------------------------------------------------------------------------
// Shared HTTP client (reused across all requests)
// ---------------------------------------------------------------------------

static HTTP_CLIENT: LazyLock<reqwest::blocking::Client> = LazyLock::new(|| {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .expect("Failed to build HTTP client")
});

// ---------------------------------------------------------------------------
// Caches
// ---------------------------------------------------------------------------

struct CacheEntry<T> {
    data: T,
    fetched_at: Instant,
}

impl<T> CacheEntry<T> {
    fn is_fresh(&self) -> bool {
        self.fetched_at.elapsed() < CACHE_TTL
    }
}

static SEARCH_CACHE: LazyLock<Mutex<HashMap<String, CacheEntry<ModrinthSearchResult>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static PROJECT_CACHE: LazyLock<Mutex<HashMap<String, CacheEntry<ModrinthProject>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static VERSIONS_CACHE: LazyLock<Mutex<HashMap<String, CacheEntry<Vec<ModrinthVersion>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Clear all caches (used by the Refresh button).
pub fn clear_caches() {
    SEARCH_CACHE.lock().unwrap().clear();
    PROJECT_CACHE.lock().unwrap().clear();
    VERSIONS_CACHE.lock().unwrap().clear();
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthProject {
    #[serde(alias = "id")]
    pub project_id: String,
    pub slug: String,
    pub author: Option<String>,
    pub title: String,
    pub description: String,
    pub categories: Vec<String>,
    #[serde(default)]
    pub display_categories: Option<Vec<String>>,
    pub client_side: String,
    pub server_side: String,
    pub body: Option<String>,
    pub icon_url: Option<String>,
    pub project_type: String,
    pub downloads: u64,
    #[serde(alias = "followers")]
    pub follows: u64,

    // --- Extra fields from search endpoint ---
    pub date_created: Option<String>,
    pub date_modified: Option<String>,
    pub latest_version: Option<String>,
    /// In search results this is a plain string (e.g. "MIT").
    /// In project details this is an object {"id","name","url"}.
    /// We accept both by deserializing as raw JSON.
    #[serde(default)]
    pub license: Option<serde_json::Value>,
    pub color: Option<u32>,

    // --- Extra fields from project detail endpoint ---
    #[serde(default)]
    pub loaders: Option<Vec<String>>,
    #[serde(default)]
    pub game_versions: Option<Vec<String>>,
    pub source_url: Option<String>,
    pub wiki_url: Option<String>,
    pub discord_url: Option<String>,
    pub published: Option<String>,
    pub updated: Option<String>,
    #[serde(default)]
    pub gallery: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub donation_urls: Option<Vec<serde_json::Value>>,
}

impl ModrinthProject {
    /// Return the license as a human-readable string regardless of shape.
    pub fn license_name(&self) -> Option<String> {
        match &self.license {
            Some(serde_json::Value::String(s)) => Some(s.clone()),
            Some(serde_json::Value::Object(obj)) => {
                obj.get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            }
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthVersion {
    pub id: String,
    pub project_id: String,
    pub author_id: String,
    pub name: String,
    pub version_number: String,
    pub changelog: Option<String>,
    pub downloads: u64,
    pub version_type: String,
    pub status: String,
    pub requested_status: Option<String>,
    pub files: Vec<ModrinthFile>,
    pub dependencies: Vec<ModrinthDependency>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub date_published: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthDependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub file_name: Option<String>,
    pub dependency_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthFile {
    pub hashes: HashMap<String, String>,
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
    pub file_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthSearchResult {
    pub hits: Vec<ModrinthProject>,
    pub offset: u32,
    pub limit: u32,
    pub total_hits: u32,
}

// ---------------------------------------------------------------------------
// API functions (with caching)
// ---------------------------------------------------------------------------

fn build_search_cache_key(
    query: &str,
    limit: u32,
    offset: u32,
    game_version: Option<&str>,
    loader: Option<&ModLoader>,
    project_type: Option<&str>,
) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}",
        query,
        limit,
        offset,
        game_version.unwrap_or(""),
        loader.map(|l| l.as_str()).unwrap_or(""),
        project_type.unwrap_or("")
    )
}

pub fn search_mods(
    query: &str,
    limit: u32,
    offset: u32,
    game_version: Option<&str>,
    loader: Option<ModLoader>,
    project_type: Option<&str>,
) -> Result<ModrinthSearchResult, String> {
    let cache_key =
        build_search_cache_key(query, limit, offset, game_version, loader.as_ref(), project_type);

    // Check cache
    {
        let cache = SEARCH_CACHE.lock().unwrap();
        if let Some(entry) = cache.get(&cache_key) {
            if entry.is_fresh() {
                return Ok(entry.data.clone());
            }
        }
    }

    let mut facets = Vec::new();
    if let Some(pt) = project_type {
        facets.push(format!("[\"project_type:{}\"]", pt));
    } else {
        facets.push("[\"project_type:mod\"]".to_string());
    }

    if let Some(v) = game_version {
        facets.push(format!("[\"versions:{}\"]", v));
    }

    if let Some(ref l) = loader {
        if *l != ModLoader::None {
            facets.push(format!("[\"categories:{}\"]", l.as_str().to_lowercase()));
        }
    }

    let facets_str = if facets.is_empty() {
        String::new()
    } else {
        format!("[{}]", facets.join(","))
    };

    let mut url = format!(
        "{}/search?query={}&limit={}&offset={}",
        MODRINTH_API_BASE, query, limit, offset
    );
    if !facets_str.is_empty() {
        url.push_str(&format!("&facets={}", urlencoding::encode(&facets_str)));
    }

    let response = HTTP_CLIENT.get(url).send().map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Modrinth API error: {}", response.status()));
    }

    let result: ModrinthSearchResult = response.json().map_err(|e| e.to_string())?;

    // Store in cache
    {
        let mut cache = SEARCH_CACHE.lock().unwrap();
        cache.insert(
            cache_key,
            CacheEntry {
                data: result.clone(),
                fetched_at: Instant::now(),
            },
        );
    }

    Ok(result)
}

pub fn get_project_versions(
    project_id: &str,
    game_version: Option<&str>,
    loader: Option<ModLoader>,
) -> Result<Vec<ModrinthVersion>, String> {
    let cache_key = format!(
        "versions|{}|{}|{}",
        project_id,
        game_version.unwrap_or(""),
        loader.as_ref().map(|l| l.as_str()).unwrap_or("")
    );

    // Check cache
    {
        let cache = VERSIONS_CACHE.lock().unwrap();
        if let Some(entry) = cache.get(&cache_key) {
            if entry.is_fresh() {
                return Ok(entry.data.clone());
            }
        }
    }

    let mut url = format!("{}/project/{}/version", MODRINTH_API_BASE, project_id);

    let mut params = Vec::new();
    if let Some(v) = game_version {
        params.push(format!("game_versions=[\"{}\"]", v));
    }
    if let Some(ref l) = loader {
        if *l != ModLoader::None {
            params.push(format!("loaders=[\"{}\"]", l.as_str().to_lowercase()));
        }
    }

    if !params.is_empty() {
        url.push_str(&format!("?{}", params.join("&")));
    }

    let response = HTTP_CLIENT.get(url).send().map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Modrinth API error: {}", response.status()));
    }

    let result: Vec<ModrinthVersion> = response.json().map_err(|e| e.to_string())?;

    // Store in cache
    {
        let mut cache = VERSIONS_CACHE.lock().unwrap();
        cache.insert(
            cache_key,
            CacheEntry {
                data: result.clone(),
                fetched_at: Instant::now(),
            },
        );
    }

    Ok(result)
}

pub fn get_project(id_or_slug: &str) -> Result<ModrinthProject, String> {
    // Check cache
    {
        let cache = PROJECT_CACHE.lock().unwrap();
        if let Some(entry) = cache.get(id_or_slug) {
            if entry.is_fresh() {
                return Ok(entry.data.clone());
            }
        }
    }

    let url = format!("{}/project/{}", MODRINTH_API_BASE, id_or_slug);
    let response = HTTP_CLIENT.get(url).send().map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Modrinth API error: {}", response.status()));
    }

    let result: ModrinthProject = response.json().map_err(|e| e.to_string())?;

    // Store in cache
    {
        let mut cache = PROJECT_CACHE.lock().unwrap();
        cache.insert(
            id_or_slug.to_string(),
            CacheEntry {
                data: result.clone(),
                fetched_at: Instant::now(),
            },
        );
    }

    Ok(result)
}

pub fn get_version_by_hash(hash: &str, algorithm: &str) -> Result<ModrinthVersion, String> {
    let url = format!("{}/version_file/{}?algorithm={}", MODRINTH_API_BASE, hash, algorithm);
    let response = HTTP_CLIENT.get(url).send().map_err(|e| e.to_string())?;
    
    if response.status() == 404 {
        return Err("Hash not found on Modrinth".to_string());
    }
    
    if !response.status().is_success() {
        return Err(format!("Modrinth API error: {}", response.status()));
    }

    let result: ModrinthVersion = response.json().map_err(|e| e.to_string())?;
    Ok(result)
}

pub fn resolve_dependencies(
    version: &ModrinthVersion,
    game_version: &str,
    loader: ModLoader,
    resolved: &mut HashMap<String, ModrinthVersion>,
) -> Result<(), String> {
    for dep in &version.dependencies {
        if dep.dependency_type != "required" {
            continue;
        }

        let project_id = match &dep.project_id {
            Some(id) => id,
            None => continue,
        };

        if resolved.contains_key(project_id) {
            continue;
        }

        let versions = get_project_versions(project_id, Some(game_version), Some(loader.clone()))?;
        if let Some(dep_version) = versions.first() {
            resolved.insert(project_id.clone(), dep_version.clone());
            resolve_dependencies(dep_version, game_version, loader.clone(), resolved)?;
        } else {
            return Err(format!(
                "Could not find compatible version for dependency {}",
                project_id
            ));
        }
    }
    Ok(())
}

pub fn install_mod_with_dependencies(
    project_id: &str,
    target_version: Option<String>,
    game_version: &str,
    loader: ModLoader,
    mods_dir: &Path,
) -> Result<Vec<PathBuf>, String> {
    let version = if let Some(vid) = target_version {
        // Fetch specific version
        let url = format!("{}/version/{}", MODRINTH_API_BASE, vid);
        let response = HTTP_CLIENT.get(&url).send().map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            return Err(format!(
                "Error fetching version {}: {}",
                vid,
                response.status()
            ));
        }
        response
            .json::<ModrinthVersion>()
            .map_err(|e| e.to_string())?
    } else {
        let versions = get_project_versions(project_id, Some(game_version), Some(loader.clone()))?;
        versions
            .first()
            .ok_or_else(|| format!("No compatible version found for project {}", project_id))?
            .clone()
    };

    let mut resolved = HashMap::new();
    resolved.insert(project_id.to_string(), version.clone());
    resolve_dependencies(&version, game_version, loader, &mut resolved)?;

    let mut downloaded_paths = Vec::new();
    for (_, v) in resolved {
        let path = download_mod(&v, mods_dir)?;
        downloaded_paths.push(path);
    }

    Ok(downloaded_paths)
}

pub fn download_mod(version: &ModrinthVersion, mods_dir: &Path) -> Result<PathBuf, String> {
    let file = version
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| version.files.first())
        .ok_or_else(|| "No files found for this version".to_string())?;

    let dest_path = mods_dir.join(&file.filename);

    let mut response = HTTP_CLIENT
        .get(&file.url)
        .send()
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Download error: {}", response.status()));
    }

    let mut dest_file = std::fs::File::create(&dest_path).map_err(|e| e.to_string())?;
    std::io::copy(&mut response, &mut dest_file).map_err(|e| e.to_string())?;

    Ok(dest_path)
}
