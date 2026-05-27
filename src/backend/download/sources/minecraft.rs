use crate::backend::instance::manager::ModLoader;
use crate::backend::runtime::versions::{
    LoaderVersion, MinecraftVersion, RawVersion, VersionType,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

const VERSION_MANIFEST_URL: &str =
    "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Deserialize, Serialize, Debug)]
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

#[derive(Deserialize, Serialize, Debug)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub url: String,
    #[serde(rename = "totalSize")]
    pub total_size: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Downloads {
    pub client: Artifact,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Artifact {
    pub path: Option<String>,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibDownloads>,
    #[serde(flatten)]
    pub legacy_info: Option<LegacyLibInfo>,
    pub rules: Option<Vec<Rule>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LegacyLibInfo {
    pub url: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LibDownloads {
    pub artifact: Option<Artifact>,
    pub classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Rule {
    pub action: String,
    pub os: Option<Os>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Os {
    pub name: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GameArguments {
    pub game: Option<Vec<serde_json::Value>>,
    pub jvm: Option<Vec<serde_json::Value>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AssetObjects {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

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

pub fn fetch_versions() -> Result<Vec<MinecraftVersion>, String> {
    let response = reqwest::blocking::get(VERSION_MANIFEST_URL)
        .map_err(|e| format!("failed to fetch version manifest: {}", e))?;

    #[derive(Deserialize)]
    struct VersionManifest {
        versions: Vec<RawVersion>,
    }

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

pub fn find_version_by_id(id: &str) -> Result<Option<MinecraftVersion>, String> {
    let all = fetch_versions()?;
    Ok(all.into_iter().find(|v| v.id == id))
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
    let url = "https://meta.prismlauncher.org/v1/net.minecraftforge/index.json";
    let response =
        reqwest::blocking::get(url).map_err(|e| format!("failed to fetch forge index: {}", e))?;

    if response.status() == 404 {
        return Ok(Vec::new());
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

    let mut valid_versions = Vec::new();
    for v in index.versions {
        if let Some(reqs) = v.requires {
            let is_for_game = reqs.iter().any(|req| {
                req.uid == "net.minecraft" && req.equals.as_deref() == Some(game_version)
            });

            if is_for_game {
                valid_versions.push(LoaderVersion {
                    version: v.version,
                    stable: true,
                });
            }
        }
    }

    Ok(valid_versions)
}

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
        .filter(|v| !v.contains('+'))
        .map(|v| {
            let stable = !v.contains("-beta");
            LoaderVersion {
                version: v.clone(),
                stable,
            }
        })
        .collect();

    valid_versions.reverse();
    Ok(valid_versions)
}

pub fn fetch_loader_versions_by_uid(
    uid: &str,
    game_version: &str,
) -> Result<Vec<LoaderVersion>, String> {
    match uid {
        "net.fabricmc.fabric-loader" => fetch_fabric_versions_for_game(game_version),
        "org.quiltmc.quilt-loader" => fetch_quilt_versions_for_game(game_version),
        "net.minecraftforge" => fetch_forge_versions_for_game(game_version),
        "net.neoforged" => fetch_neoforge_versions_for_game(game_version),
        _ => Err(format!("Unsupported loader UID: {}", uid)),
    }
}

pub fn fetch_loader_versions(
    loader: &ModLoader,
    game_version: &str,
) -> Result<Vec<LoaderVersion>, String> {
    if let Some(uid) = loader.uid() {
        fetch_loader_versions_by_uid(uid, game_version)
    } else {
        Ok(Vec::new())
    }
}

pub fn download_minecraft_data_internal<F>(
    version: &RawVersion,
    loader: &ModLoader,
    loader_version: Option<&str>,
    data_path: &Path,
    progress_callback: F,
) -> Result<(), String>
where
    F: Fn(String, f32) + Clone + Send + Sync + 'static,
{
    let client = reqwest::blocking::Client::new();

    // 1. Download Version Meta
    progress_callback("Downloading version metadata...".to_string(), 0.1);
    let meta_resp = client
        .get(&version.url)
        .send()
        .map_err(|e| format!("Failed to fetch version meta: {}", e))?;
    let meta: VersionMeta = meta_resp
        .json()
        .map_err(|e| format!("Failed to parse version meta: {}", e))?;

    let meta_folder = data_path.join("meta").join("net.minecraft");
    fs::create_dir_all(&meta_folder).map_err(|e| e.to_string())?;
    let meta_file = meta_folder.join(format!("{}.json", version.id));

    let meta_json = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    fs::write(&meta_file, meta_json).map_err(|e| e.to_string())?;

    // 2. Download Asset Index
    if let Some(asset_index) = &meta.asset_index {
        progress_callback("Downloading asset index...".to_string(), 0.2);
        let index_resp = client
            .get(&asset_index.url)
            .send()
            .map_err(|e| format!("Failed to fetch asset index: {}", e))?;
        let index_content = index_resp.text().map_err(|e| e.to_string())?;

        let index_folder = data_path.join("assets").join("indexes");
        fs::create_dir_all(&index_folder).map_err(|e| e.to_string())?;
        fs::write(
            index_folder.join(format!("{}.json", asset_index.id)),
            &index_content,
        )
        .map_err(|e| e.to_string())?;

        let assets: AssetObjects =
            serde_json::from_str(&index_content).map_err(|e| e.to_string())?;

        // 3. Download Assets
        let objects_folder = data_path.join("assets").join("objects");
        fs::create_dir_all(&objects_folder).map_err(|e| e.to_string())?;

        let downloaded_count = AtomicUsize::new(0);
        let total_assets = assets.objects.len();
        let client_clone = client.clone();
        let callback_clone = progress_callback.clone();

        assets
            .objects
            .into_par_iter()
            .try_for_each(|(name, obj)| -> Result<(), String> {
                let hash = &obj.hash;
                let prefix = &hash[0..2];
                let path = objects_folder.join(prefix).join(hash);

                let mut needs_download = !path.exists();
                if !needs_download {
                    if let Ok(meta) = path.metadata() {
                        if meta.len() != obj.size {
                            needs_download = true;
                        }
                    } else {
                        needs_download = true;
                    }
                }

                if needs_download {
                    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
                    let url = format!(
                        "https://resources.download.minecraft.net/{}/{}",
                        prefix, hash
                    );
                    let mut resp = client_clone
                        .get(url)
                        .send()
                        .map_err(|e| e.to_string())?
                        .error_for_status()
                        .map_err(|e| e.to_string())?;
                    let mut file = fs::File::create(&path).map_err(|e| e.to_string())?;
                    std::io::copy(&mut resp, &mut file).map_err(|e| e.to_string())?;
                }

                let current = downloaded_count.fetch_add(1, Ordering::SeqCst) + 1;
                if current % 25 == 0 || current == total_assets {
                    let progress = 0.2 + (current as f32 / total_assets as f32) * 0.4;
                    callback_clone(format!("Downloading asset: {}", name), progress);
                }
                Ok(())
            })?;
    }

    // 4. Download Client Jar
    if let Some(downloads) = &meta.downloads {
        progress_callback("Downloading client jar...".to_string(), 0.7);
        let client_jar_path = data_path
            .join("libraries")
            .join("com")
            .join("mojang")
            .join("minecraft")
            .join(&version.id);
        fs::create_dir_all(&client_jar_path).map_err(|e| e.to_string())?;
        let client_jar_file = client_jar_path.join(format!("minecraft-{}-client.jar", version.id));

        let mut needs_client_jar = !client_jar_file.exists();
        if !needs_client_jar {
            if let Ok(file_meta) = client_jar_file.metadata() {
                if file_meta.len() != downloads.client.size {
                    needs_client_jar = true;
                }
            } else {
                needs_client_jar = true;
            }
        }

        if needs_client_jar {
            let mut resp = client
                .get(&downloads.client.url)
                .send()
                .map_err(|e| e.to_string())?
                .error_for_status()
                .map_err(|e| e.to_string())?;
            let mut file = fs::File::create(&client_jar_file).map_err(|e| e.to_string())?;
            std::io::copy(&mut resp, &mut file).map_err(|e| e.to_string())?;
        }
    }

    // 5. Download Libraries
    if let Some(libraries) = &meta.libraries {
        let lib_count = AtomicUsize::new(0);
        let total_libs = libraries.len();
        let callback_clone = progress_callback.clone();

        libraries
            .par_iter()
            .enumerate()
            .try_for_each(|(_i, lib)| -> Result<(), String> {
                download_lib_internal(
                    lib,
                    data_path,
                    &client,
                    &lib_count,
                    total_libs,
                    callback_clone.clone(),
                )
            })?;
    }

    // 6. Loader Meta & Libraries
    match loader {
        ModLoader::Fabric => {
            if let Some(loader_ver) = loader_version {
                progress_callback("Downloading Fabric metadata...".to_string(), 0.95);

                let game_version = version.id.split('-').next().unwrap_or(&version.id);
                let fabric_meta_url = format!(
                    "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
                    game_version, loader_ver
                );

                let resp = client
                    .get(&fabric_meta_url)
                    .send()
                    .map_err(|e| format!("Fabric API request failed: {}", e))?;

                if !resp.status().is_success() {
                    return Err(format!(
                        "Fabric API returned status {}",
                        resp.status()
                    ));
                }

                let fabric_meta: VersionMeta = resp
                    .json()
                    .map_err(|e| format!("Failed to parse Fabric Profile: {}", e))?;

                let loader_meta_folder = data_path.join("meta").join("net.fabricmc.fabric-loader");
                fs::create_dir_all(&loader_meta_folder).map_err(|e| e.to_string())?;

                let loader_meta_file = loader_meta_folder.join(format!("{}.json", loader_ver));
                let loader_meta_json = serde_json::to_string_pretty(&fabric_meta).unwrap();
                fs::write(&loader_meta_file, loader_meta_json).map_err(|e| e.to_string())?;

                if let Some(libs) = &fabric_meta.libraries {
                    let lib_count = AtomicUsize::new(0);
                    let total_libs = libs.len();
                    for lib in libs {
                        let _ = download_lib_internal(
                            lib,
                            data_path,
                            &client,
                            &lib_count,
                            total_libs,
                            progress_callback.clone(),
                        );
                    }
                }

                let _ = ensure_intermediary(game_version, data_path, &client, progress_callback.clone());
            }
        }
        ModLoader::Quilt => {
            if let Some(loader_ver) = loader_version {
                progress_callback("Downloading Quilt metadata...".to_string(), 0.95);

                let game_version = version.id.split('-').next().unwrap_or(&version.id);
                let quilt_meta_url = format!(
                    "https://meta.quiltmc.org/v3/versions/loader/{}/{}/profile/json",
                    game_version, loader_ver
                );

                let resp = client
                    .get(&quilt_meta_url)
                    .send()
                    .map_err(|e| format!("Quilt API failed: {}", e))?;
                let quilt_meta: VersionMeta = resp
                    .json()
                    .map_err(|e| format!("Failed to parse Quilt JSON: {}", e))?;

                let loader_meta_folder = data_path.join("meta").join("org.quiltmc.quilt-loader");
                fs::create_dir_all(&loader_meta_folder).map_err(|e| e.to_string())?;

                let loader_meta_file = loader_meta_folder.join(format!("{}.json", loader_ver));
                fs::write(
                    &loader_meta_file,
                    serde_json::to_string_pretty(&quilt_meta).unwrap(),
                )
                .map_err(|e| e.to_string())?;

                if let Some(libs) = &quilt_meta.libraries {
                    let lib_count = AtomicUsize::new(0);
                    for lib in libs {
                        let _ = download_lib_internal(
                            lib,
                            data_path,
                            &client,
                            &lib_count,
                            libs.len(),
                            progress_callback.clone(),
                        );
                    }
                }

                let _ = ensure_intermediary(game_version, data_path, &client, progress_callback.clone());
            }
        }
        ModLoader::Forge => {
            if let Some(loader_ver) = loader_version {
                let meta_url = format!(
                    "https://meta.prismlauncher.org/v1/net.minecraftforge/{}.json",
                    loader_ver
                );

                progress_callback("Fetching Forge metadata (Prism)...".into(), 0.90);

                let resp = client
                    .get(&meta_url)
                    .send()
                    .map_err(|e| format!("Forge meta request failed: {}", e))?;

                if !resp.status().is_success() {
                    return Err(format!("Forge meta API returned status {}", resp.status()));
                }

                #[derive(Deserialize)]
                struct PrismForgeMeta {
                    libraries: Option<Vec<Library>>,
                    #[serde(rename = "mavenFiles")]
                    maven_files: Option<Vec<Library>>,
                }

                let resp_text = resp.text().map_err(|e| e.to_string())?;

                let loader_meta_folder = data_path.join("meta").join("net.minecraftforge");
                fs::create_dir_all(&loader_meta_folder).map_err(|e| e.to_string())?;

                let loader_meta_file = loader_meta_folder.join(format!("{}.json", loader_ver));
                fs::write(&loader_meta_file, &resp_text).map_err(|e| e.to_string())?;

                let forge_meta: PrismForgeMeta = serde_json::from_str(&resp_text)
                    .map_err(|e| format!("Failed to parse Forge meta: {}", e))?;

                let mut all_libs: Vec<&Library> = Vec::new();
                if let Some(ref libs) = forge_meta.libraries {
                    all_libs.extend(libs.iter());
                }
                if let Some(ref maven) = forge_meta.maven_files {
                    all_libs.extend(maven.iter());
                }

                if !all_libs.is_empty() {
                    let total_libs = all_libs.len();
                    let lib_count = AtomicUsize::new(0);
                    for lib in &all_libs {
                        let _ = download_lib_internal(
                            lib,
                            data_path,
                            &client,
                            &lib_count,
                            total_libs,
                            progress_callback.clone(),
                        );
                    }
                }
            }
        }
        ModLoader::NeoForge => {
            if let Some(loader_ver) = loader_version {
                let meta_url = format!(
                    "https://meta.prismlauncher.org/v1/net.neoforged/{}.json",
                    loader_ver
                );

                progress_callback("Fetching NeoForge metadata (Prism)...".into(), 0.90);

                let resp = client
                    .get(&meta_url)
                    .send()
                    .map_err(|e| format!("NeoForge meta request failed: {}", e))?;

                if !resp.status().is_success() {
                    return Err(format!("NeoForge meta API returned status {}", resp.status()));
                }

                #[derive(Deserialize)]
                struct PrismNeoForgeMeta {
                    libraries: Option<Vec<Library>>,
                    #[serde(rename = "mavenFiles")]
                    maven_files: Option<Vec<Library>>,
                }

                let resp_text = resp.text().map_err(|e| e.to_string())?;

                let loader_meta_folder = data_path.join("meta").join("net.neoforged");
                fs::create_dir_all(&loader_meta_folder).map_err(|e| e.to_string())?;

                let loader_meta_file = loader_meta_folder.join(format!("{}.json", loader_ver));
                fs::write(&loader_meta_file, &resp_text).map_err(|e| e.to_string())?;

                let neoforge_meta: PrismNeoForgeMeta = serde_json::from_str(&resp_text)
                    .map_err(|e| format!("Failed to parse NeoForge meta: {}", e))?;

                let mut all_libs: Vec<&Library> = Vec::new();
                if let Some(ref libs) = neoforge_meta.libraries {
                    all_libs.extend(libs.iter());
                }
                if let Some(ref maven) = neoforge_meta.maven_files {
                    all_libs.extend(maven.iter());
                }

                if !all_libs.is_empty() {
                    let total_libs = all_libs.len();
                    let lib_count = AtomicUsize::new(0);
                    for lib in &all_libs {
                        let _ = download_lib_internal(
                            lib,
                            data_path,
                            &client,
                            &lib_count,
                            total_libs,
                            progress_callback.clone(),
                        );
                    }
                }
            }
        }
        _ => {}
    }

    progress_callback("Finished downloading.".to_string(), 1.0);
    Ok(())
}

fn download_lib_internal<F>(
    lib: &Library,
    data_path: &Path,
    client: &reqwest::blocking::Client,
    lib_count: &AtomicUsize,
    total_libs: usize,
    progress_callback: F,
) -> Result<(), String>
where
    F: Fn(String, f32) + Send + 'static,
{
    let mut artifacts_to_download: Vec<Artifact> = Vec::new();

    if let Some(downloads) = &lib.downloads {
        if let Some(artifact) = &downloads.artifact {
            artifacts_to_download.push(artifact.clone());
        }
        if let Some(classifiers) = &downloads.classifiers {
            if let Some(native_artifact) = classifiers.get("natives-linux") {
                artifacts_to_download.push(native_artifact.clone());
            }
        }
    }

    if artifacts_to_download.is_empty() {
        let legacy_url = lib
            .legacy_info
            .as_ref()
            .and_then(|l| l.url.as_ref())
            .cloned();
        let legacy_sha1 = lib
            .legacy_info
            .as_ref()
            .and_then(|l| l.sha1.clone())
            .unwrap_or_default();
        let legacy_size = lib
            .legacy_info
            .as_ref()
            .and_then(|l| l.size)
            .unwrap_or(0);

        let maven_base = legacy_url.unwrap_or_else(|| {
            "https://libraries.minecraft.net/".to_string()
        });

        artifacts_to_download.push(Artifact {
            url: maven_base,
            sha1: legacy_sha1,
            size: legacy_size,
            path: None,
        });
    }

    for artifact in artifacts_to_download {
        let mut allowed = true;
        if let Some(rules) = &lib.rules {
            allowed = false;
            for rule in rules {
                if rule.action == "allow" {
                    if let Some(os) = &rule.os {
                        if os.name == "linux" {
                            allowed = true;
                        }
                    } else {
                        allowed = true;
                    }
                } else if rule.action == "disallow" {
                    if let Some(os) = &rule.os {
                        if os.name == "linux" {
                            allowed = false;
                        }
                    }
                }
            }
        }

        if allowed {
            let rel_path = if let Some(p) = &artifact.path {
                p.clone()
            } else {
                let parts: Vec<&str> = lib.name.split(':').collect();
                if parts.len() < 3 {
                    continue;
                }
                let group = parts[0].replace('.', "/");
                let artifact_id = parts[1];
                let version = parts[2];

                let mut filename = format!("{}-{}", artifact_id, version);

                if parts.len() > 3 {
                    let extra = parts[3];
                    let classifier = extra.split('@').next().unwrap_or(extra);
                    filename.push_str(&format!("-{}", classifier));
                }

                let extension = if let Some(pos) = lib.name.find('@') {
                    &lib.name[pos + 1..]
                } else {
                    "jar"
                };

                format!(
                    "{}/{}/{}/{}.{}",
                    group, artifact_id, version, filename, extension
                )
            };

            let mut download_url = artifact.url.clone();
            if !download_url.ends_with(".jar") && !download_url.ends_with(".zip") {
                if !download_url.ends_with("/") {
                    download_url.push('/');
                }
                download_url.push_str(&rel_path);
            }

            let full_path = data_path.join("libraries").join(&rel_path);
            let mut needs_lib = !full_path.exists();
            if !needs_lib {
                if let Ok(m) = full_path.metadata() {
                    if m.len() != artifact.size {
                        needs_lib = true;
                    }
                } else {
                    needs_lib = true;
                }
            }

            if needs_lib {
                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut resp = client
                    .get(&download_url)
                    .send()
                    .map_err(|e| e.to_string())?
                    .error_for_status()
                    .map_err(|e| e.to_string())?;
                let mut file = fs::File::create(&full_path).map_err(|e| e.to_string())?;
                std::io::copy(&mut resp, &mut file).map_err(|e| e.to_string())?;
            }

            let current = lib_count.fetch_add(1, Ordering::SeqCst) + 1;
            let progress = 0.7 + (current as f32 / total_libs as f32) * 0.2;
            progress_callback(format!("Downloaded library: {}", lib.name), progress);
        }
    }
    Ok(())
}

fn ensure_intermediary<F>(
    game_version: &str,
    data_path: &Path,
    client: &reqwest::blocking::Client,
    progress_callback: F,
) -> Result<(), String>
where
    F: Fn(String, f32) + Send + 'static,
{
    let intermediary_meta_folder = data_path.join("meta").join("net.fabricmc.intermediary");
    fs::create_dir_all(&intermediary_meta_folder).map_err(|e| e.to_string())?;
    let intermediary_meta_file = intermediary_meta_folder.join(format!("{}.json", game_version));

    if !intermediary_meta_file.exists() {
        let meta_content = serde_json::json!({
            "mainClass": null,
            "libraries": [
                {
                    "name": format!("net.fabricmc:intermediary:{}", game_version),
                    "url": "https://maven.fabricmc.net/"
                }
            ]
        });
        fs::write(
            &intermediary_meta_file,
            serde_json::to_string_pretty(&meta_content).unwrap(),
        )
        .map_err(|e| e.to_string())?;
    }

    let intermediary_lib = Library {
        name: format!("net.fabricmc:intermediary:{}", game_version),
        downloads: None,
        legacy_info: Some(LegacyLibInfo {
            url: Some("https://maven.fabricmc.net/".to_string()),
            sha1: None,
            size: None,
        }),
        rules: None,
    };
    let _ = download_lib_internal(
        &intermediary_lib,
        data_path,
        client,
        &AtomicUsize::new(0),
        1,
        progress_callback,
    );
    Ok(())
}
