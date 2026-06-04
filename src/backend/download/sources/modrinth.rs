use crate::backend::instance::manager::ModLoader;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

const MODRINTH_API_BASE: &str = "https://api.modrinth.com/v2";
const USER_AGENT: &str = "obelisk-launcher-rs (github.com/magnotec/obelisk-launcher)";
const CACHE_TTL: Duration = Duration::from_secs(300);

static HTTP_CLIENT: LazyLock<reqwest::blocking::Client> = LazyLock::new(|| {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .expect("Failed to build HTTP client")
});

struct CacheEntry<T> {
    data: T,
    fetched_at: Instant,
}

impl<T> CacheEntry<T> {
    fn is_fresh(&self) -> bool {
        self.fetched_at.elapsed() < CACHE_TTL
    }
}

static SEARCH_CACHE: LazyLock<Mutex<HashMap<String, CacheEntry<ModSearchResult>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static PROJECT_CACHE: LazyLock<Mutex<HashMap<String, CacheEntry<ModProject>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static VERSIONS_CACHE: LazyLock<Mutex<HashMap<String, CacheEntry<Vec<ModVersion>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn clear_caches() {
    SEARCH_CACHE.lock().unwrap().clear();
    PROJECT_CACHE.lock().unwrap().clear();
    VERSIONS_CACHE.lock().unwrap().clear();
}

// ---------------------------------------------------------------------------
// Clean UI-Friendly Data Structures
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GalleryImage {
    pub url: String,
    pub featured: bool,
    pub title: Option<String>,
    pub description: Option<String>,
    pub created: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModProject {
    pub project_id: String,
    pub slug: String,
    pub author: Option<String>,
    pub title: String,
    pub description: String,
    pub categories: Vec<String>,
    pub display_categories: Option<Vec<String>>,
    pub client_side: String,
    pub server_side: String,
    pub body: Option<String>,
    pub icon_url: Option<String>,
    pub project_type: String,
    pub downloads: u64,
    pub follows: u64,
    pub date_created: Option<String>,
    pub date_modified: Option<String>,
    pub latest_version: Option<String>,
    pub license_name: Option<String>,
    pub color: Option<u32>,
    pub loaders: Option<Vec<String>>,
    pub game_versions: Option<Vec<String>>,
    pub source_url: Option<String>,
    pub wiki_url: Option<String>,
    pub discord_url: Option<String>,
    pub published: Option<String>,
    pub updated: Option<String>,
    pub gallery: Option<Vec<GalleryImage>>,
}

impl ModProject {
    pub fn license_name(&self) -> Option<String> {
        self.license_name.clone()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModVersion {
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
    pub files: Vec<ModFile>,
    pub dependencies: Vec<ModDependency>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub date_published: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModDependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub file_name: Option<String>,
    pub dependency_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModFile {
    pub hashes: HashMap<String, String>,
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
    pub file_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModSearchResult {
    pub hits: Vec<ModProject>,
    pub offset: u32,
    pub limit: u32,
    pub total_hits: u32,
}

// ---------------------------------------------------------------------------
// Internal API structures for parsing raw JSON from Modrinth
// ---------------------------------------------------------------------------

#[derive(Deserialize, Debug)]
struct RawProject {
    #[serde(alias = "project_id")]
    id: String,
    slug: String,
    author: Option<String>,
    title: String,
    description: Option<String>,
    #[serde(default)]
    categories: Option<Vec<String>>,
    #[serde(default)]
    display_categories: Option<Vec<String>>,
    client_side: Option<String>,
    server_side: Option<String>,
    body: Option<String>,
    icon_url: Option<String>,
    project_type: Option<String>,
    downloads: Option<u64>,
    followers: Option<u64>,
    follows: Option<u64>,
    date_created: Option<String>,
    date_modified: Option<String>,
    latest_version: Option<String>,
    #[serde(default)]
    license: Option<serde_json::Value>,
    color: Option<u32>,
    #[serde(default)]
    loaders: Option<Vec<String>>,
    #[serde(default)]
    game_versions: Option<Vec<String>>,
    source_url: Option<String>,
    wiki_url: Option<String>,
    discord_url: Option<String>,
    published: Option<String>,
    updated: Option<String>,
    #[serde(default)]
    gallery: Option<Vec<GalleryImage>>,
}

impl RawProject {
    fn into_clean(self) -> ModProject {
        let license_name = match &self.license {
            Some(serde_json::Value::String(s)) => Some(s.clone()),
            Some(serde_json::Value::Object(obj)) => obj
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            _ => None,
        };

        ModProject {
            project_id: self.id,
            slug: self.slug,
            author: self.author,
            title: self.title,
            description: self.description.unwrap_or_default(),
            categories: self.categories.unwrap_or_default(),
            display_categories: self.display_categories,
            client_side: self.client_side.unwrap_or_else(|| "optional".to_string()),
            server_side: self.server_side.unwrap_or_else(|| "optional".to_string()),
            body: self.body,
            icon_url: self.icon_url,
            project_type: self.project_type.unwrap_or_else(|| "mod".to_string()),
            downloads: self.downloads.unwrap_or(0),
            follows: self.follows.or(self.followers).unwrap_or(0),
            date_created: self.date_created,
            date_modified: self.date_modified,
            latest_version: self.latest_version,
            license_name,
            color: self.color,
            loaders: self.loaders,
            game_versions: self.game_versions,
            source_url: self.source_url,
            wiki_url: self.wiki_url,
            discord_url: self.discord_url,
            published: self.published,
            updated: self.updated,
            gallery: self.gallery,
        }
    }
}

#[derive(Deserialize, Debug)]
struct RawSearchProject {
    #[serde(alias = "project_id")]
    id: String,
    slug: String,
    author: Option<String>,
    title: String,
    description: Option<String>,
    #[serde(default)]
    categories: Option<Vec<String>>,
    #[serde(default)]
    display_categories: Option<Vec<String>>,
    client_side: Option<String>,
    server_side: Option<String>,
    body: Option<String>,
    icon_url: Option<String>,
    project_type: Option<String>,
    downloads: Option<u64>,
    followers: Option<u64>,
    follows: Option<u64>,
    date_created: Option<String>,
    date_modified: Option<String>,
    latest_version: Option<String>,
    #[serde(default)]
    license: Option<serde_json::Value>,
    color: Option<u32>,
    #[serde(default)]
    loaders: Option<Vec<String>>,
    #[serde(default)]
    game_versions: Option<Vec<String>>,
    source_url: Option<String>,
    wiki_url: Option<String>,
    discord_url: Option<String>,
    published: Option<String>,
    updated: Option<String>,
    #[serde(default)]
    gallery: Option<Vec<String>>,
}

impl RawSearchProject {
    fn into_clean(self) -> ModProject {
        let license_name = match &self.license {
            Some(serde_json::Value::String(s)) => Some(s.clone()),
            Some(serde_json::Value::Object(obj)) => obj
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            _ => None,
        };

        let gallery = self.gallery.map(|urls| {
            urls.into_iter()
                .map(|url| GalleryImage {
                    url,
                    featured: false,
                    title: None,
                    description: None,
                    created: String::new(),
                })
                .collect()
        });

        ModProject {
            project_id: self.id,
            slug: self.slug,
            author: self.author,
            title: self.title,
            description: self.description.unwrap_or_default(),
            categories: self.categories.unwrap_or_default(),
            display_categories: self.display_categories,
            client_side: self.client_side.unwrap_or_else(|| "optional".to_string()),
            server_side: self.server_side.unwrap_or_else(|| "optional".to_string()),
            body: self.body,
            icon_url: self.icon_url,
            project_type: self.project_type.unwrap_or_else(|| "mod".to_string()),
            downloads: self.downloads.unwrap_or(0),
            follows: self.follows.or(self.followers).unwrap_or(0),
            date_created: self.date_created,
            date_modified: self.date_modified,
            latest_version: self.latest_version,
            license_name,
            color: self.color,
            loaders: self.loaders,
            game_versions: self.game_versions,
            source_url: self.source_url,
            wiki_url: self.wiki_url,
            discord_url: self.discord_url,
            published: self.published,
            updated: self.updated,
            gallery,
        }
    }
}

#[derive(Deserialize, Debug)]
struct RawSearchResult {
    hits: Vec<RawSearchProject>,
    offset: u32,
    limit: u32,
    total_hits: u32,
}

#[derive(Deserialize, Debug)]
struct RawVersion {
    id: String,
    project_id: String,
    author_id: String,
    name: String,
    version_number: String,
    changelog: Option<String>,
    downloads: u64,
    version_type: String,
    status: String,
    requested_status: Option<String>,
    files: Vec<RawFile>,
    dependencies: Vec<RawDependency>,
    game_versions: Vec<String>,
    loaders: Vec<String>,
    date_published: String,
}

impl RawVersion {
    fn into_clean(self) -> ModVersion {
        ModVersion {
            id: self.id,
            project_id: self.project_id,
            author_id: self.author_id,
            name: self.name,
            version_number: self.version_number,
            changelog: self.changelog,
            downloads: self.downloads,
            version_type: self.version_type,
            status: self.status,
            requested_status: self.requested_status,
            files: self.files.into_iter().map(|f| f.into_clean()).collect(),
            dependencies: self
                .dependencies
                .into_iter()
                .map(|d| d.into_clean())
                .collect(),
            game_versions: self.game_versions,
            loaders: self.loaders,
            date_published: self.date_published,
        }
    }
}

#[derive(Deserialize, Debug)]
struct RawDependency {
    version_id: Option<String>,
    project_id: Option<String>,
    file_name: Option<String>,
    dependency_type: String,
}

impl RawDependency {
    fn into_clean(self) -> ModDependency {
        ModDependency {
            version_id: self.version_id,
            project_id: self.project_id,
            file_name: self.file_name,
            dependency_type: self.dependency_type,
        }
    }
}

#[derive(Deserialize, Debug)]
struct RawFile {
    hashes: HashMap<String, String>,
    url: String,
    filename: String,
    primary: bool,
    size: u64,
    file_type: Option<String>,
}

impl RawFile {
    fn into_clean(self) -> ModFile {
        ModFile {
            hashes: self.hashes,
            url: self.url,
            filename: self.filename,
            primary: self.primary,
            size: self.size,
            file_type: self.file_type,
        }
    }
}

// ---------------------------------------------------------------------------
// API implementation
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
) -> Result<ModSearchResult, String> {
    let cache_key = build_search_cache_key(
        query,
        limit,
        offset,
        game_version,
        loader.as_ref(),
        project_type,
    );

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

    let response = HTTP_CLIENT
        .get(url)
        .send()
        .map_err(super::map_reqwest_error)?;
    if !response.status().is_success() {
        return Err(format!("Modrinth API error: {}", response.status()));
    }

    let result: RawSearchResult = response.json().map_err(|e| e.to_string())?;

    let clean_result = ModSearchResult {
        hits: result.hits.into_iter().map(|h| h.into_clean()).collect(),
        offset: result.offset,
        limit: result.limit,
        total_hits: result.total_hits,
    };

    {
        let mut cache = SEARCH_CACHE.lock().unwrap();
        cache.insert(
            cache_key,
            CacheEntry {
                data: clean_result.clone(),
                fetched_at: Instant::now(),
            },
        );
    }

    Ok(clean_result)
}

pub fn get_project_versions(
    project_id: &str,
    game_version: Option<&str>,
    loader: Option<ModLoader>,
) -> Result<Vec<ModVersion>, String> {
    let cache_key = format!(
        "versions|{}|{}|{}",
        project_id,
        game_version.unwrap_or(""),
        loader.as_ref().map(|l| l.as_str()).unwrap_or("")
    );

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

    let response = HTTP_CLIENT
        .get(url)
        .send()
        .map_err(super::map_reqwest_error)?;
    if !response.status().is_success() {
        return Err(format!("Modrinth API error: {}", response.status()));
    }

    let result: Vec<RawVersion> = response.json().map_err(|e| e.to_string())?;
    let clean_result: Vec<ModVersion> = result.into_iter().map(|v| v.into_clean()).collect();

    {
        let mut cache = VERSIONS_CACHE.lock().unwrap();
        cache.insert(
            cache_key,
            CacheEntry {
                data: clean_result.clone(),
                fetched_at: Instant::now(),
            },
        );
    }

    Ok(clean_result)
}

pub fn get_project(id_or_slug: &str) -> Result<ModProject, String> {
    {
        let cache = PROJECT_CACHE.lock().unwrap();
        if let Some(entry) = cache.get(id_or_slug) {
            if entry.is_fresh() {
                return Ok(entry.data.clone());
            }
        }
    }

    let url = format!("{}/project/{}", MODRINTH_API_BASE, id_or_slug);
    let response = HTTP_CLIENT
        .get(url)
        .send()
        .map_err(super::map_reqwest_error)?;
    if !response.status().is_success() {
        return Err(format!("Modrinth API error: {}", response.status()));
    }

    let result: RawProject = response.json().map_err(|e| e.to_string())?;
    let clean_result = result.into_clean();

    {
        let mut cache = PROJECT_CACHE.lock().unwrap();
        cache.insert(
            id_or_slug.to_string(),
            CacheEntry {
                data: clean_result.clone(),
                fetched_at: Instant::now(),
            },
        );
    }

    Ok(clean_result)
}

pub fn get_version_by_hash(hash: &str, algorithm: &str) -> Result<ModVersion, String> {
    let url = format!(
        "{}/version_file/{}?algorithm={}",
        MODRINTH_API_BASE, hash, algorithm
    );
    let response = HTTP_CLIENT
        .get(url)
        .send()
        .map_err(super::map_reqwest_error)?;

    if response.status() == 404 {
        return Err("Hash not found on Modrinth".to_string());
    }

    if !response.status().is_success() {
        return Err(format!("Modrinth API error: {}", response.status()));
    }

    let result: RawVersion = response.json().map_err(|e| e.to_string())?;
    Ok(result.into_clean())
}

pub fn resolve_dependencies(
    version: &ModVersion,
    game_version: &str,
    loader: ModLoader,
    resolved: &mut HashMap<String, ModVersion>,
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

pub fn install_mod_with_dependencies<F>(
    project_id: &str,
    target_version: Option<String>,
    game_version: &str,
    loader: ModLoader,
    mods_dir: &Path,
    progress_callback: F,
) -> Result<Vec<PathBuf>, String>
where
    F: Fn(String, f32) + Clone + Send + 'static,
{
    progress_callback("Resolving Modrinth dependencies...".to_string(), 0.1);
    let version = if let Some(vid) = target_version {
        let url = format!("{}/version/{}", MODRINTH_API_BASE, vid);
        let response = HTTP_CLIENT
            .get(&url)
            .send()
            .map_err(super::map_reqwest_error)?;
        if !response.status().is_success() {
            return Err(format!(
                "Error fetching version {}: {}",
                vid,
                response.status()
            ));
        }
        let raw: RawVersion = response.json().map_err(|e| e.to_string())?;
        raw.into_clean()
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
    let total_mods = resolved.len();
    let mut current = 0;

    for (_, v) in resolved {
        current += 1;
        let progress = 0.1 + (current as f32 / total_mods as f32) * 0.9;
        progress_callback(format!("Downloading mod: {}", v.name), progress);
        let path = download_mod(&v, mods_dir)?;
        downloaded_paths.push(path);
    }

    Ok(downloaded_paths)
}

pub fn download_mod(version: &ModVersion, mods_dir: &Path) -> Result<PathBuf, String> {
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
        .map_err(super::map_reqwest_error)?;
    if !response.status().is_success() {
        return Err(format!("Download error: {}", response.status()));
    }

    let mut dest_file = std::fs::File::create(&dest_path).map_err(|e| e.to_string())?;
    std::io::copy(&mut response, &mut dest_file).map_err(|e| e.to_string())?;

    Ok(dest_path)
}
