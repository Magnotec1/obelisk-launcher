use serde::{Deserialize, Serialize};

const VERSION_MANIFEST_URL: &str =
    "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoaderVersion {
    pub version: String,
    #[serde(default)]
    pub stable: bool,
}

pub fn fetch_fabric_versions_for_game(game_version: &str) -> Result<Vec<LoaderVersion>, String> {
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}",
        game_version
    );
    let response = reqwest::blocking::get(url).map_err(|e| {
        format!(
            "failed to fetch fabric loader versions for {}: {}",
            game_version, e
        )
    })?;

    #[derive(Deserialize)]
    struct FabricLoaderResponse {
        loader: LoaderVersion,
    }

    let versions: Vec<FabricLoaderResponse> = response
        .json()
        .map_err(|e| format!("failed to parse fabric loader versions: {}", e))?;

    Ok(versions.into_iter().map(|r| r.loader).collect())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VersionType {
    Release,
    Snapshot,
    OldBeta,
    OldAlpha,
    Experiment,
}

impl VersionType {
    pub fn as_str(&self) -> &str {
        match self {
            VersionType::Release => "Release",
            VersionType::Snapshot => "Snapshot",
            VersionType::OldBeta => "Beta",
            VersionType::OldAlpha => "Alpha",
            VersionType::Experiment => "Experiment",
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

#[derive(Debug, Clone)]
pub struct MinecraftVersion {
    pub id: String,
    pub version_type: VersionType,
    pub raw: RawVersion,
}

#[derive(Deserialize)]
struct VersionManifest {
    versions: Vec<RawVersion>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RawVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
}

/// known april fools / experiment snapshot ids
const EXPERIMENT_IDS: &[&str] = &[
    "20w14infinite",
    "22w13oneblockatatime",
    "23w13a_or_b",
    "24w14potato",
    "25w14craftmine",
    "3D Shareware v1.34",
    "15w14a",
    "1.RV-Pre1",
    "2.0",
];

fn is_experiment(id: &str) -> bool {
    EXPERIMENT_IDS.iter().any(|&exp| exp == id)
}

fn classify_version(raw: &RawVersion) -> VersionType {
    if is_experiment(&raw.id) {
        return VersionType::Experiment;
    }
    match raw.version_type.as_str() {
        "release" => VersionType::Release,
        "snapshot" => VersionType::Snapshot,
        "old_beta" => VersionType::OldBeta,
        "old_alpha" => VersionType::OldAlpha,
        _ => VersionType::Snapshot,
    }
}

/// fetches the version manifest from mojang and returns all versions.
/// this is a blocking call and should be run from a background thread.
pub fn fetch_versions() -> Result<Vec<MinecraftVersion>, String> {
    let response = reqwest::blocking::get(VERSION_MANIFEST_URL)
        .map_err(|e| format!("failed to fetch version manifest: {}", e))?;

    let manifest: VersionManifest = response
        .json()
        .map_err(|e| format!("failed to parse version manifest: {}", e))?;

    let versions = manifest
        .versions
        .iter()
        .map(|raw| MinecraftVersion {
            id: raw.id.clone(),
            version_type: classify_version(raw),
            raw: raw.clone(),
        })
        .collect();

    Ok(versions)
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
    let all = fetch_versions()?;
    Ok(all.into_iter().find(|v| v.id == id))
}

pub fn fetch_quilt_versions_for_game(game_version: &str) -> Result<Vec<LoaderVersion>, String> {
    let url = format!(
        "https://meta.quiltmc.org/v3/versions/loader/{}",
        game_version
    );
    let response = reqwest::blocking::get(url).map_err(|e| {
        format!(
            "failed to fetch quilt loader versions for {}: {}",
            game_version, e
        )
    })?;

    #[derive(Deserialize)]
    struct QuiltLoaderResponse {
        loader: LoaderVersion,
    }

    let versions: Vec<QuiltLoaderResponse> = response
        .json()
        .map_err(|e| format!("failed to parse quilt loader versions: {}", e))?;

    Ok(versions.into_iter().map(|r| r.loader).collect())
}

pub fn fetch_forge_versions_for_game(game_version: &str) -> Result<Vec<LoaderVersion>, String> {
    // prism only has one master index for forge
    let url = "https://meta.prismlauncher.org/v1/net.minecraftforge/index.json";

    let response =
        reqwest::blocking::get(url).map_err(|e| format!("failed to fetch forge index: {}", e))?;

    if response.status() == 404 {
        return Err("forge index not found".to_string());
    }

    #[derive(Deserialize)]
    struct PrismRequirement {
        uid: String,
        equals: Option<String>,
    }

    #[derive(Deserialize)]
    struct PrismForgeVersion {
        version: String,
        requires: Option<Vec<PrismRequirement>>,
    }

    #[derive(Deserialize)]
    struct PrismIndex {
        versions: Vec<PrismForgeVersion>,
    }

    let index: PrismIndex = response
        .json()
        .map_err(|e| format!("failed to parse forge index: {}", e))?;

    // filter the master list to only include forge versions that require our specific mc version
    let mut valid_versions = Vec::new();
    for v in index.versions {
        if let Some(reqs) = v.requires {
            let is_for_game = reqs.iter().any(|req| {
                req.uid == "net.minecraft" && req.equals.as_deref() == Some(game_version)
            });

            if is_for_game {
                // map to your loaderversion struct
                valid_versions.push(LoaderVersion {
                    version: v.version,
                    stable: true, // forge doesn't clearly mark stable/beta in this index, defaulting to true
                });
            }
        }
    }

    if valid_versions.is_empty() {
        // instead of throwing an error, you can just return an empty vec here too
        // if you want it to match quilt's 404 behavior for unsupported versions
        return Ok(Vec::new());
    }

    Ok(valid_versions)
}

/// Fetches NeoForge versions compatible with a given Minecraft game version.
///
/// NeoForge versioning: major.minor maps to MC minor.patch.
/// E.g., NeoForge 21.4.x corresponds to MC 1.21.4.
/// For MC 1.20.x, the NeoForge major is 20.
pub fn fetch_neoforge_versions_for_game(game_version: &str) -> Result<Vec<LoaderVersion>, String> {
    let url = "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";

    let response = reqwest::blocking::get(url)
        .map_err(|e| format!("failed to fetch neoforge versions: {}", e))?;

    #[derive(Deserialize)]
    struct NeoForgeVersionList {
        versions: Vec<String>,
    }

    let version_list: NeoForgeVersionList = response
        .json()
        .map_err(|e| format!("failed to parse neoforge versions: {}", e))?;

    // Parse the game version to determine the NeoForge major.minor prefix
    // MC 1.21.4 -> NeoForge prefix "21.4."
    // MC 1.20.2 -> NeoForge prefix "20.2."
    let mc_parts: Vec<&str> = game_version.split('.').collect();
    if mc_parts.len() < 2 {
        return Ok(Vec::new());
    }

    let mc_minor = mc_parts.get(1).unwrap_or(&"0");
    let mc_patch = mc_parts.get(2).unwrap_or(&"0");
    let neoforge_prefix = format!("{}.{}.", mc_minor, mc_patch);

    let mut valid_versions: Vec<LoaderVersion> = version_list
        .versions
        .iter()
        .filter(|v| v.starts_with(&neoforge_prefix))
        // Skip alpha/snapshot versions with '+' in them
        .filter(|v| !v.contains('+'))
        .map(|v| {
            let stable = !v.contains("-beta");
            LoaderVersion {
                version: v.clone(),
                stable,
            }
        })
        .collect();

    // Reverse to show newest versions first
    valid_versions.reverse();

    Ok(valid_versions)
}
