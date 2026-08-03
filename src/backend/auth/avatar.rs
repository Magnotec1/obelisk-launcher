use crate::config::Config;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

const AVATAR_API_URL: &str = "https://crafatar.com/avatars";
const USER_AGENT: &str = "obelisk-launcher-rs (github.com/magnotec/obelisk-launcher)";

/// Returns the cache file path for an account's avatar PNG.
pub fn get_avatar_cache_path(uuid: &str) -> PathBuf {
    let dir = Config::get_data_dir().join("avatars");
    let _ = fs::create_dir_all(&dir);
    dir.join(format!("{}.png", uuid))
}

/// Checks if an avatar is already cached locally.
pub fn is_avatar_cached(uuid: &str) -> bool {
    let path = get_avatar_cache_path(uuid);
    path.exists() && path.metadata().map(|m| m.len() > 0).unwrap_or(false)
}

/// Synchronously fetches and caches a 2D head avatar PNG for the given UUID.
pub fn fetch_and_cache_avatar(uuid: &str) -> Result<PathBuf, String> {
    let cache_path = get_avatar_cache_path(uuid);
    if is_avatar_cached(uuid) {
        return Ok(cache_path);
    }

    let url = format!("{}/{}?size=64&overlay", AVATAR_API_URL, uuid);
    let client = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("Avatar fetch failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Avatar request failed with status: {}", resp.status()));
    }

    let bytes = resp.bytes().map_err(|e| format!("Failed to read avatar bytes: {}", e))?;
    fs::write(&cache_path, &bytes).map_err(|e| format!("Failed to save avatar image: {}", e))?;

    Ok(cache_path)
}
