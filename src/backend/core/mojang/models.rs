use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Artifact {
    pub path: Option<String>,
    #[serde(default)]
    pub sha1: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub url: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct LibDownloads {
    pub artifact: Option<Artifact>,
    pub classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct LegacyLibInfo {
    pub url: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibDownloads>,
    #[serde(flatten)]
    pub legacy_info: Option<LegacyLibInfo>,
    pub url: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub rules: Option<Vec<Rule>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Rule {
    pub action: String,
    pub os: Option<Os>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Os {
    pub name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct GameArguments {
    pub game: Option<Vec<serde_json::Value>>,
    pub jvm: Option<Vec<serde_json::Value>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct AssetIndex {
    pub id: String,
    #[serde(default)]
    pub sha1: String,
    #[serde(default)]
    pub url: String,
    #[serde(rename = "totalSize", default)]
    pub total_size: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Downloads {
    pub client: Artifact,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct VersionMeta {
    pub id: String,
    #[serde(rename = "mainClass")]
    pub main_class: Option<String>,
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
    pub arguments: Option<GameArguments>,
    #[serde(rename = "assetIndex")]
    pub asset_index: Option<AssetIndex>,
    pub libraries: Option<Vec<Library>>,
    pub downloads: Option<Downloads>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ComponentMeta {
    #[serde(rename = "mainClass")]
    pub main_class: Option<String>,
    pub libraries: Option<Vec<Library>>,
    #[serde(rename = "mavenFiles")]
    pub maven_files: Option<Vec<Library>>,
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
    pub arguments: Option<GameArguments>,
    #[serde(rename = "assetIndex")]
    pub asset_index: Option<AssetIndex>,
    #[serde(rename = "+tweakers", default)]
    pub tweakers: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct AssetObjects {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}
