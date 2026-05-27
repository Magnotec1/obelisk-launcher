use serde::{Deserialize, Serialize};
use crate::backend::instance::manager::ModLoader;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoaderVersion {
    pub version: String,
    #[serde(default)]
    pub stable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionType {
    Release,
    Snapshot,
    OldBeta,
    OldAlpha,
    Experiment,
}

impl VersionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            VersionType::Release => "release",
            VersionType::Snapshot => "snapshot",
            VersionType::OldBeta => "old_beta",
            VersionType::OldAlpha => "old_alpha",
            VersionType::Experiment => "experiment",
        }
    }

    pub fn all() -> Vec<VersionType> {
        vec![
            VersionType::Release,
            VersionType::Snapshot,
            VersionType::OldBeta,
            VersionType::OldAlpha,
            VersionType::Experiment,
        ]
    }
}

impl std::fmt::Display for VersionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftVersion {
    pub id: String,
    pub version_type: VersionType,
    pub raw: RawVersion,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RawVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
}


pub fn fetch_fabric_versions_for_game(game_version: &str) -> Result<Vec<LoaderVersion>, String> {
    crate::backend::download::sources::minecraft::fetch_fabric_versions_for_game(game_version)
}

pub fn fetch_versions() -> Result<Vec<MinecraftVersion>, String> {
    crate::backend::download::sources::minecraft::fetch_versions()
}

/// filters versions by allowed types.
pub fn filter_versions(
    versions: &[MinecraftVersion],
    allowed: &[VersionType],
) -> Vec<MinecraftVersion> {
    versions
        .iter()
        .filter(|v| allowed.contains(&v.version_type))
        .cloned()
        .collect()
}

/// finds a specific version by its id.
pub fn find_version_by_id(id: &str) -> Result<Option<MinecraftVersion>, String> {
    crate::backend::download::sources::minecraft::find_version_by_id(id)
}

pub fn fetch_quilt_versions_for_game(game_version: &str) -> Result<Vec<LoaderVersion>, String> {
    crate::backend::download::sources::minecraft::fetch_quilt_versions_for_game(game_version)
}

pub fn fetch_forge_versions_for_game(game_version: &str) -> Result<Vec<LoaderVersion>, String> {
    crate::backend::download::sources::minecraft::fetch_forge_versions_for_game(game_version)
}

pub fn fetch_neoforge_versions_for_game(game_version: &str) -> Result<Vec<LoaderVersion>, String> {
    crate::backend::download::sources::minecraft::fetch_neoforge_versions_for_game(game_version)
}

pub fn fetch_loader_versions_by_uid(uid: &str, game_version: &str) -> Result<Vec<LoaderVersion>, String> {
    crate::backend::download::sources::minecraft::fetch_loader_versions_by_uid(uid, game_version)
}

pub fn fetch_loader_versions(loader: &ModLoader, game_version: &str) -> Result<Vec<LoaderVersion>, String> {
    crate::backend::download::sources::minecraft::fetch_loader_versions(loader, game_version)
}
