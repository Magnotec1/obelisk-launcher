use crate::backend::instance::manager::ModLoader;
use crate::backend::runtime::versions::RawVersion;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone)]
pub enum DownloadMsg {
    Progress(String, f32), // message, percent
    DetailedProgress {
        task: String,
        current: usize,
        total: usize,
        item_name: String,
        overall_progress: f32,
    },
    Error(String),
    Finished,
}

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

pub fn download_minecraft_data(
    version: &RawVersion,
    loader: &ModLoader,
    loader_version: Option<&str>,
    data_path: &Path,
    sender: &std::sync::mpsc::Sender<DownloadMsg>,
) -> Result<(), String> {
    let client = reqwest::blocking::Client::new();

    // 1. Download Version Meta
    sender
        .send(DownloadMsg::Progress(
            "Downloading version metadata...".to_string(),
            0.1,
        ))
        .unwrap();
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

    // Convert to the format our launch logic expects (simplified)
    // Actually, let's just save it as is and adjust launch.rs if needed.
    // Other formats might be slightly different but Mojang's is the source.
    let meta_json = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    fs::write(&meta_file, meta_json).map_err(|e| e.to_string())?;

    // 2. Download Asset Index
    if let Some(asset_index) = &meta.asset_index {
        sender
            .send(DownloadMsg::Progress(
                "Downloading asset index...".to_string(),
                0.2,
            ))
            .unwrap();
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
        let sender_clone = sender.clone(); // Clone sender for parallel use
        let client_clone = client.clone(); // Clone client for parallel use

        // We parallelize the asset downloads
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
                    // Each thread gets its own client or shares one. Reqwest Client is designed to be shared.
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
                if current % 10 == 0 || current == total_assets {
                    let progress = 0.2 + (current as f32 / total_assets as f32) * 0.4;
                    sender_clone
                        .send(DownloadMsg::DetailedProgress {
                            task: "Downloading assets".to_string(),
                            current,
                            total: total_assets,
                            item_name: name.clone(),
                            overall_progress: progress,
                        })
                        .ok();
                }
                Ok(())
            })?;
    }

    // 4. Download Client Jar
    if let Some(downloads) = &meta.downloads {
        sender
            .send(DownloadMsg::Progress(
                "Downloading client jar...".to_string(),
                0.7,
            ))
            .unwrap();
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

        libraries
            .par_iter()
            .enumerate()
            .try_for_each(|(_i, lib)| -> Result<(), String> {
                download_lib_internal(lib, data_path, &client, &lib_count, total_libs, sender)
            })?;
    }

    // 6. Download Loader Meta & Libraries
    println!(
        "Checking loader: {:?} (version: {:?})",
        loader, loader_version
    );
    match loader {
        ModLoader::Fabric => {
            if let Some(loader_ver) = loader_version {
                sender
                    .send(DownloadMsg::Progress(
                        "Downloading Fabric metadata...".to_string(),
                        0.95,
                    ))
                    .unwrap();

                // Sanitize version ID for Fabric API (it wants just the game version, e.g. 1.21.1)
                let game_version = version.id.split('-').next().unwrap_or(&version.id);
                let fabric_meta_url = format!(
                    "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
                    game_version, loader_ver
                );

                println!("Fetching Fabric Profile: {}", fabric_meta_url);
                let resp = client
                    .get(&fabric_meta_url)
                    .send()
                    .map_err(|e| format!("Fabric API request failed: {}", e))?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let err_body = resp.text().unwrap_or_else(|_| "No body".into());
                    return Err(format!("Fabric API returned {}: {}", status, err_body));
                }

                let fabric_meta: VersionMeta = resp
                    .json()
                    .map_err(|e| format!("Failed to parse Fabric Profile JSON: {}", e))?;

                let loader_meta_folder = data_path.join("meta").join("net.fabricmc.fabric-loader");
                fs::create_dir_all(&loader_meta_folder)
                    .map_err(|e| format!("Failed to create Fabric meta folder: {}", e))?;

                let loader_meta_file = loader_meta_folder.join(format!("{}.json", loader_ver));
                let loader_meta_json = serde_json::to_string_pretty(&fabric_meta)
                    .map_err(|e| format!("Failed to serialize Fabric meta: {}", e))?;

                println!("Saving Fabric metadata to: {:?}", loader_meta_file);
                fs::write(&loader_meta_file, loader_meta_json)
                    .map_err(|e| format!("Failed to write Fabric metadata file: {}", e))?;

                // Download Intermediary meta too
                let _intermediary_url = format!(
                    "https://meta.fabricmc.net/v2/versions/intermediary/{}",
                    version.id
                );
                // Actually, intermediary usually doesn't have a full profile, it's just a component.
                // But Prism/PolyMC expects a meta file.
                // We'll create a dummy one or fetch it if possible.
                // For now, let's just download Fabric libraries.

                if let Some(libs) = &fabric_meta.libraries {
                    let lib_count = AtomicUsize::new(0);
                    let total_libs = libs.len();
                    for lib in libs {
                        let _ = download_lib_internal(
                            lib, data_path, &client, &lib_count, total_libs, sender,
                        );
                    }
                }
            }
        }
        ModLoader::Quilt => {
            if let Some(loader_ver) = loader_version {
                sender
                    .send(DownloadMsg::Progress(
                        "Downloading Quilt metadata...".to_string(),
                        0.95,
                    ))
                    .unwrap();

                // Sanitize version (e.g., 1.21.1-hotfix -> 1.21.1)
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
                            sender,
                        );
                    }
                }
            }
        }

        ModLoader::Forge => {
            if let Some(loader_ver) = loader_version {
                // Prism Meta format: net.minecraftforge/<version>.json
                let meta_url = format!(
                    "https://meta.prismlauncher.org/v1/net.minecraftforge/{}.json",
                    loader_ver
                );

                sender
                    .send(DownloadMsg::Progress(
                        "Fetching Forge metadata (Prism Meta)...".into(),
                        0.90,
                    ))
                    .unwrap();
                println!("Fetching Forge metadata: {}", meta_url);

                let resp = client
                    .get(&meta_url)
                    .send()
                    .map_err(|e| format!("Forge meta request failed: {}", e))?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let err_body = resp.text().unwrap_or_else(|_| "No body".into());
                    return Err(format!("Forge meta API returned {}: {}", status, err_body));
                }

                // The Prism Meta Forge JSON has 'libraries' and 'mavenFiles', both
                // using the same format as vanilla Library objects.
                #[derive(Deserialize)]
                #[allow(dead_code)]
                struct PrismForgeMeta {
                    libraries: Option<Vec<Library>>,
                    #[serde(rename = "mavenFiles")]
                    maven_files: Option<Vec<Library>>,
                    #[serde(rename = "mainClass")]
                    main_class: Option<String>,
                    #[serde(rename = "minecraftArguments")]
                    minecraft_arguments: Option<String>,
                }

                let resp_text = resp
                    .text()
                    .map_err(|e| format!("Failed to read Forge meta body: {}", e))?;

                // Save the raw metadata for launch.rs to use later
                let loader_meta_folder = data_path.join("meta").join("net.minecraftforge");
                fs::create_dir_all(&loader_meta_folder)
                    .map_err(|e| format!("Failed to create Forge meta folder: {}", e))?;

                let loader_meta_file = loader_meta_folder.join(format!("{}.json", loader_ver));
                println!("Saving Forge metadata to: {:?}", loader_meta_file);
                fs::write(&loader_meta_file, &resp_text)
                    .map_err(|e| format!("Failed to write Forge metadata: {}", e))?;

                let forge_meta: PrismForgeMeta = serde_json::from_str(&resp_text)
                    .map_err(|e| format!("Failed to parse Forge meta JSON: {}", e))?;

                // Collect all libs: runtime libraries + maven files (installer jars, etc.)
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
                    sender
                        .send(DownloadMsg::Progress(
                            format!("Downloading {} Forge libraries...", total_libs),
                            0.92,
                        ))
                        .unwrap();

                    for lib in &all_libs {
                        let _ = download_lib_internal(
                            lib, data_path, &client, &lib_count, total_libs, sender,
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

                sender
                    .send(DownloadMsg::Progress(
                        "Fetching NeoForge metadata (Prism Meta)...".into(),
                        0.90,
                    ))
                    .unwrap();
                println!("Fetching NeoForge metadata: {}", meta_url);

                let resp = client
                    .get(&meta_url)
                    .send()
                    .map_err(|e| format!("NeoForge meta request failed: {}", e))?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let err_body = resp.text().unwrap_or_else(|_| "No body".into());
                    return Err(format!("NeoForge meta API returned {}: {}", status, err_body));
                }

                #[derive(Deserialize)]
                #[allow(dead_code)]
                struct PrismNeoForgeMeta {
                    libraries: Option<Vec<Library>>,
                    #[serde(rename = "mavenFiles")]
                    maven_files: Option<Vec<Library>>,
                    #[serde(rename = "mainClass")]
                    main_class: Option<String>,
                    #[serde(rename = "minecraftArguments")]
                    minecraft_arguments: Option<String>,
                }

                let resp_text = resp
                    .text()
                    .map_err(|e| format!("Failed to read NeoForge meta body: {}", e))?;

                let loader_meta_folder = data_path.join("meta").join("net.neoforged");
                fs::create_dir_all(&loader_meta_folder)
                    .map_err(|e| format!("Failed to create NeoForge meta folder: {}", e))?;

                let loader_meta_file = loader_meta_folder.join(format!("{}.json", loader_ver));
                println!("Saving NeoForge metadata to: {:?}", loader_meta_file);
                fs::write(&loader_meta_file, &resp_text)
                    .map_err(|e| format!("Failed to write NeoForge metadata: {}", e))?;

                let neoforge_meta: PrismNeoForgeMeta = serde_json::from_str(&resp_text)
                    .map_err(|e| format!("Failed to parse NeoForge meta JSON: {}", e))?;

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
                    sender
                        .send(DownloadMsg::Progress(
                            format!("Downloading {} NeoForge libraries...", total_libs),
                            0.92,
                        ))
                        .unwrap();

                    for lib in &all_libs {
                        let _ = download_lib_internal(
                            lib, data_path, &client, &lib_count, total_libs, sender,
                        );
                    }
                }
            }
        }
        _ => {}
    }

    sender
        .send(DownloadMsg::Progress(
            "Finished downloading.".to_string(),
            1.0,
        ))
        .unwrap();
    sender.send(DownloadMsg::Finished).unwrap();

    Ok(())
}

fn download_lib_internal(
    lib: &Library,
    data_path: &Path,
    client: &reqwest::blocking::Client,
    lib_count: &AtomicUsize,
    total_libs: usize,
    sender: &std::sync::mpsc::Sender<DownloadMsg>,
) -> Result<(), String> {
    let mut artifacts_to_download: Vec<Artifact> = Vec::new();

    // 1. Try modern 'downloads' object
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

    // 2. Fallback to legacy/Fabric top-level URL or Maven-style resolution
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

        // Use provided URL as Maven base, or default to Mojang's Maven repo
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
                // Maven name conversion: group:artifact:version[:classifier][@extension]
                let parts: Vec<&str> = lib.name.split(':').collect();
                if parts.len() < 3 {
                    continue;
                }
                let group = parts[0].replace('.', "/");
                let artifact_id = parts[1];
                let version = parts[2];

                let mut filename = format!("{}-{}", artifact_id, version);

                // Handle classifier (4th part, e.g. ":installer" or ":sources")
                if parts.len() > 3 {
                    let extra = parts[3];
                    // Strip @extension if present in classifier
                    let classifier = extra.split('@').next().unwrap_or(extra);
                    filename.push_str(&format!("-{}", classifier));
                }

                // Handle @extension (e.g. "@zip")
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
            // If URL is just a Maven base, append the relative path
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
                println!("Downloading library: {} from {}", lib.name, download_url);
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
            sender
                .send(DownloadMsg::DetailedProgress {
                    task: "Downloading libraries".to_string(),
                    current,
                    total: total_libs,
                    item_name: lib.name.clone(),
                    overall_progress: progress,
                })
                .ok();
        }
    }
    Ok(())
}
