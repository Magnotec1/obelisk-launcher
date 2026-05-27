use serde::Deserialize;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Deserialize, Debug, Clone)]
pub struct JavaPackage {
    pub id: String,
    pub distribution: String,
    pub major_version: u32,
    pub java_version: String,
    pub architecture: String,
    pub package_type: String,
    pub filename: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DiscoResponse {
    pub result: Vec<JavaPackage>,
}

#[derive(Debug, Clone)]
pub enum JavaDownloadProgress {
    Downloading { current: u64, total: u64 },
    Extracting,
    Finished(PathBuf),
    Error(String),
}

pub fn get_available_packages() -> Result<Vec<JavaPackage>, String> {
    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x64"
    };

    let url = format!(
        "https://api.foojay.io/disco/v3.0/packages?operating_system=linux&architecture={}&package_type=jdk&release_status=ga&latest=available",
        arch
    );
    
    let response = reqwest::blocking::get(url)
        .map_err(|e| format!("Failed to fetch versions: {}", e))?;
    
    let info: DiscoResponse = response
        .json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    let mut packages = info.result;
    
    let mut seen = std::collections::HashSet::new();
    packages.retain(|p| {
        let key = (p.distribution.clone(), p.major_version);
        if seen.contains(&key) {
            false
        } else {
            seen.insert(key);
            true
        }
    });

    packages.sort_by(|a, b| {
        b.major_version.cmp(&a.major_version)
            .then_with(|| a.distribution.cmp(&b.distribution))
    });
    
    Ok(packages)
}

pub fn download_and_extract_with_progress<F>(
    package_id: &str,
    target_dir: &Path,
    cancel_flag: Arc<AtomicBool>,
    progress_callback: F,
) where
    F: Fn(JavaDownloadProgress) + Send + 'static,
{
    let redirect_url = format!("https://api.foojay.io/disco/v3.0/ids/{}/redirect", package_id);
    
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
        
    let response = match client.get(&redirect_url).send() {
        Ok(r) => r,
        Err(e) => {
            progress_callback(JavaDownloadProgress::Error(format!("Failed to get download link: {}", e)));
            return;
        }
    };

    let download_url = match response.headers().get("location") {
        Some(loc) => loc.to_str().unwrap_or_default().to_string(),
        None => {
            progress_callback(JavaDownloadProgress::Error("No download redirect found".into()));
            return;
        }
    };

    let filename = download_url.split('/').last().unwrap_or("java_runtime.tar.gz");

    if let Err(e) = fs::create_dir_all(target_dir) {
        progress_callback(JavaDownloadProgress::Error(e.to_string()));
        return;
    }
    let download_path = target_dir.join(filename);

    let mut response = match reqwest::blocking::get(&download_url) {
        Ok(r) => r,
        Err(e) => {
            progress_callback(JavaDownloadProgress::Error(e.to_string()));
            return;
        }
    };

    let total_size = response.content_length().unwrap_or(0);
    let mut file = match fs::File::create(&download_path) {
        Ok(f) => f,
        Err(e) => {
            progress_callback(JavaDownloadProgress::Error(e.to_string()));
            return;
        }
    };

    let mut downloaded: u64 = 0;
    let mut buffer = [0; 8192];
    let mut last_progress_report = std::time::Instant::now();

    loop {
        match response.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                if cancel_flag.load(Ordering::Relaxed) {
                    let _ = fs::remove_file(&download_path);
                    return;
                }
                if let Err(e) = file.write_all(&buffer[..n]) {
                    progress_callback(JavaDownloadProgress::Error(e.to_string()));
                    return;
                }
                downloaded += n as u64;
                if last_progress_report.elapsed().as_millis() > 100 {
                    progress_callback(JavaDownloadProgress::Downloading {
                        current: downloaded,
                        total: total_size,
                    });
                    last_progress_report = std::time::Instant::now();
                }
            }
            Err(e) => {
                progress_callback(JavaDownloadProgress::Error(e.to_string()));
                return;
            }
        }
    }
    progress_callback(JavaDownloadProgress::Downloading {
        current: downloaded,
        total: total_size,
    });

    progress_callback(JavaDownloadProgress::Extracting);
    
    let output = match Command::new("tar")
        .arg("-xzf")
        .arg(&download_path)
        .arg("-C")
        .arg(target_dir)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            progress_callback(JavaDownloadProgress::Error(e.to_string()));
            return;
        }
    };

    if !output.status.success() {
        progress_callback(JavaDownloadProgress::Error(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
        return;
    }

    let _ = fs::remove_file(&download_path);

    if let Ok(entries) = fs::read_dir(target_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("bin/java").exists() {
                progress_callback(JavaDownloadProgress::Finished(path));
                return;
            }
        }
    }

    progress_callback(JavaDownloadProgress::Error(
        "Extraction failed to find java binary".to_string(),
    ));
}
