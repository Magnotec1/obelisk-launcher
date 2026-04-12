use serde::Deserialize;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Deserialize, Debug, Clone)]
pub struct AdoptiumRelease {
    pub release_name: String,
    pub binaries: Vec<AdoptiumBinary>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AdoptiumBinary {
    pub package: AdoptiumPackage,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AdoptiumPackage {
    pub link: String,
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AdoptiumInfoResponse {
    pub available_releases: Vec<u32>,
    pub available_lts_releases: Vec<u32>,
}

#[derive(Debug, Clone)]
pub enum JavaDownloadProgress {
    Downloading { current: u64, total: u64 },
    Extracting,
    Finished(PathBuf),
    Error(String),
}

pub struct JavaDownloadManager;

impl JavaDownloadManager {
    pub fn get_available_versions() -> Result<Vec<u32>, String> {
        let url = "https://api.adoptium.net/v3/info/available_releases";
        let response = reqwest::blocking::get(url)
            .map_err(|e| format!("Failed to fetch versions: {}", e))?;
        
        let info: AdoptiumInfoResponse = response
            .json()
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        let mut versions = info.available_releases;
        versions.sort_by(|a, b| b.cmp(a));
        Ok(versions)
    }

    pub fn download_and_extract_with_progress<F>(
        version: u32,
        target_dir: &Path,
        cancel_flag: Arc<AtomicBool>,
        progress_callback: F,
    ) where F: Fn(JavaDownloadProgress) + Send + 'static {
        let arch = if cfg!(target_arch = "x86_64") {
            "x64"
        } else if cfg!(target_arch = "aarch64") {
            "aarch64"
        } else {
            progress_callback(JavaDownloadProgress::Error("Unsupported architecture".to_string()));
            return;
        };

        let url = format!(
            "https://api.adoptium.net/v3/assets/feature_releases/{}/ga?architecture={}&image_type=jre&os=linux&project=jdk",
            version, arch
        );

        let response_res = reqwest::blocking::get(&url);
        let releases: Vec<AdoptiumRelease> = match response_res {
            Ok(r) => match r.json() {
                Ok(j) => j,
                Err(e) => {
                    progress_callback(JavaDownloadProgress::Error(format!("JSON error: {}", e)));
                    return;
                }
            },
            Err(e) => {
                progress_callback(JavaDownloadProgress::Error(format!("Network error: {}", e)));
                return;
            }
        };

        let release = match releases.first() {
            Some(r) => r,
            None => {
                progress_callback(JavaDownloadProgress::Error("No release found".into()));
                return;
            }
        };
        let binary = match release.binaries.first() {
            Some(b) => b,
            None => {
                progress_callback(JavaDownloadProgress::Error("No binaries found".into()));
                return;
            }
        };
        
        let download_url = binary.package.link.clone();
        let filename = binary.package.name.clone();

        if let Err(e) = fs::create_dir_all(target_dir) {
            progress_callback(JavaDownloadProgress::Error(e.to_string()));
            return;
        }
        let download_path = target_dir.join(&filename);

        // Download loop
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
                    // Report progress every 100ms or so to avoid flooding the message loop
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
        // Final 100% report
        progress_callback(JavaDownloadProgress::Downloading { current: downloaded, total: total_size });

        // Extraction
        progress_callback(JavaDownloadProgress::Extracting);
        
        let output = match Command::new("tar")
            .arg("-xzf")
            .arg(&download_path)
            .arg("-C")
            .arg(target_dir)
            .output() {
                Ok(o) => o,
                Err(e) => {
                    progress_callback(JavaDownloadProgress::Error(e.to_string()));
                    return;
                }
            };

        if !output.status.success() {
            progress_callback(JavaDownloadProgress::Error(String::from_utf8_lossy(&output.stderr).to_string()));
            return;
        }

        let _ = fs::remove_file(&download_path);

        // Find binary
        if let Ok(entries) = fs::read_dir(target_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("bin/java").exists() {
                    progress_callback(JavaDownloadProgress::Finished(path));
                    return;
                }
            }
        }

        progress_callback(JavaDownloadProgress::Error("Extraction failed to find java binary".to_string()));
    }
}
