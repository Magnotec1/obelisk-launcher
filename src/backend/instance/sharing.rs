use serde::{Deserialize, Serialize};
use crate::backend::instance::manager::{Instance, ModLoader, create_instance, CreateInstanceOptions};
use crate::backend::download::sources::modrinth;
use std::path::Path;
use sha2::{Sha512, Digest};
use std::io::{Read, Write};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SharedMod {
    pub project_id: String,
    pub version_id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SharedInstance {
    pub name: String,
    pub minecraft_version: String,
    pub mod_loader: ModLoader,
    pub loader_version: Option<String>,
    pub mods: Vec<SharedMod>,
}

impl SharedInstance {
    pub fn to_code(&self) -> Result<String, String> {
        let json = serde_json::to_string(self).map_err(|e| e.to_string())?;
        
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;
        
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
        let compressed = encoder.finish().map_err(|e| e.to_string())?;
        
        use base64::{Engine as _, engine::general_purpose};
        Ok(general_purpose::URL_SAFE_NO_PAD.encode(compressed))
    }

    pub fn from_code(code: &str) -> Result<Self, String> {
        let code = code.trim();
        use base64::{Engine as _, engine::general_purpose};
        let decoded = general_purpose::URL_SAFE_NO_PAD.decode(code).map_err(|e| e.to_string())?;
        
        use flate2::read::ZlibDecoder;
        let mut decoder = ZlibDecoder::new(&decoded[..]);
        let mut json = String::new();
        decoder.read_to_string(&mut json).map_err(|e| e.to_string())?;
        
        serde_json::from_str(&json).map_err(|e| e.to_string())
    }
}

pub fn export_instance(instance: &Instance) -> Result<SharedInstance, String> {
    let (mod_loader, loader_version) = instance.get_loader_info();
    
    let mut shared_mods = Vec::new();
    
    // Collecting mods might take time, but we just do it sequentially for now
    for m in &instance.mods {
        let mod_path = instance.minecraft_dir.join("mods").join(&m.filename);
        if let Ok(mut file) = std::fs::File::open(&mod_path) {
            let mut hasher = Sha512::new();
            let mut buffer = [0; 8192];
            while let Ok(n) = file.read(&mut buffer) {
                if n == 0 { break; }
                hasher.update(&buffer[..n]);
            }
            let hash = hex::encode(hasher.finalize());
            
            if let Ok(version) = modrinth::get_version_by_hash(&hash, "sha512") {
                shared_mods.push(SharedMod {
                    project_id: version.project_id,
                    version_id: version.id,
                    name: m.name.clone(),
                });
            }
        }
    }
    
    Ok(SharedInstance {
        name: instance.name.clone(),
        minecraft_version: instance.minecraft_version.clone().unwrap_or_default(),
        mod_loader,
        loader_version,
        mods: shared_mods,
    })
}

pub fn import_shared_instance(
    shared: SharedInstance,
    instances_path: &Path,
    progress: impl Fn(String),
) -> Result<(), String> {
    progress(format!("Creating instance {}...", shared.name));
    // 1. Create the base instance
    let options = CreateInstanceOptions {
        name: shared.name.clone(),
        minecraft_version: shared.minecraft_version.clone(),
        mod_loader: shared.mod_loader.clone(),
        loader_version: shared.loader_version,
    };
    
    let instance_dir = create_instance(instances_path, options)?;
    let mods_dir = if instance_dir.join(".minecraft").is_dir() {
        instance_dir.join(".minecraft").join("mods")
    } else {
        instance_dir.join("minecraft").join("mods")
    };
    
    // 2. Download mods from Modrinth
    let total_mods = shared.mods.len();
    let mut failed_mods = Vec::new();
    for (i, sm) in shared.mods.into_iter().enumerate() {
        progress(format!("Downloading mod {} of {} ({})...", i + 1, total_mods, sm.name));
        // We use the version_id directly if possible
        if let Err(e) = modrinth::install_mod_with_dependencies(
            &sm.project_id,
            Some(sm.version_id),
            &shared.minecraft_version,
            shared.mod_loader.clone(),
            &mods_dir,
            |_, _| {},
        ) {
            eprintln!("Failed to install mod {}: {}", sm.name, e);
            failed_mods.push(sm.name);
        }
    }
    
    if !failed_mods.is_empty() {
        return Err(format!("Failed to install some mods: {}", failed_mods.join(", ")));
    }
    
    Ok(())
}

pub fn export_instance_to_zip(
    instance: &Instance,
    output_path: &Path,
    progress: impl Fn(f64, String),
) -> Result<(), String> {
    let file = std::fs::File::create(output_path).map_err(|e| e.to_string())?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let instance_path = &instance.path;

    // First, count files for progress calculation
    use walkdir::WalkDir;
    let total_files = WalkDir::new(instance_path).into_iter().count();
    let mut current_file = 0;

    // Walk through the instance directory
    for entry in WalkDir::new(instance_path) {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let name = path.strip_prefix(instance_path).map_err(|e| e.to_string())?;

        current_file += 1;
        if current_file % 10 == 0 || current_file == total_files {
            let p = current_file as f64 / total_files as f64;
            progress(p, format!("Zipping: {}", name.display()));
        }

        // Skip some folders to keep the zip size reasonable
        if path.is_dir() {
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if dir_name == "logs" || dir_name == "cache" || dir_name == "screenshots" {
                continue;
            }
            if name.as_os_str().is_empty() {
                continue;
            }
            zip.add_directory(name.to_string_lossy(), options).map_err(|e| e.to_string())?;
        } else {
            // Skip large files or irrelevant ones if needed
            zip.start_file(name.to_string_lossy(), options).map_err(|e| e.to_string())?;
            let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
            zip.write_all(&buffer).map_err(|e| e.to_string())?;
        }
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn import_instance_from_zip(
    zip_path: &Path,
    instances_root: &Path,
    progress: impl Fn(f64, String),
) -> Result<(), String> {
    progress(0.0, format!("Opening zip file {}...", zip_path.display()));
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;

    // Determine the base folder name
    let zip_stem = zip_path.file_stem().and_then(|s| s.to_str()).unwrap_or("Imported Instance");
    
    // Check if the zip has a single top-level directory
    let mut common_prefix = None;
    let mut has_files_at_root = false;

    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name();
        
        if let Some(first_slash) = name.find('/') {
            let prefix = &name[..first_slash];
            match common_prefix {
                None => common_prefix = Some(prefix.to_string()),
                Some(ref p) if p != prefix => {
                    has_files_at_root = true;
                    break;
                }
                _ => {}
            }
        } else if !name.is_empty() {
            has_files_at_root = true;
            break;
        }
    }

    let prefix_to_strip = if !has_files_at_root { common_prefix } else { None };

    // Sanitize folder name
    let folder_name = zip_stem.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect::<String>();
    let mut target_dir = instances_root.join(&folder_name);
    
    // Handle collisions
    if target_dir.exists() {
        let mut i = 1;
        while target_dir.exists() {
            target_dir = instances_root.join(format!("{}_{}", folder_name, i));
            i += 1;
        }
    }

    std::fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

    let total_files = archive.len();
    for i in 0..total_files {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name().to_string();
        
        let p = (i + 1) as f64 / total_files as f64;
        if i % 10 == 0 || i + 1 == total_files {
            progress(p, format!("Extracting: {}", name));
        }

        let stripped_name = if let Some(ref prefix) = prefix_to_strip {
            if name.starts_with(prefix) && name.len() > prefix.len() + 1 {
                &name[prefix.len() + 1..]
            } else {
                continue; // Skip the root folder entry itself
            }
        } else {
            &name
        };

        if stripped_name.is_empty() {
            continue;
        }

        let outpath = target_dir.join(stripped_name);

        if name.ends_with('/') {
            std::fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
                }
            }
            let mut outfile = std::fs::File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }
    }

    progress(1.0, "Instance imported successfully.".to_string());
    Ok(())
}
