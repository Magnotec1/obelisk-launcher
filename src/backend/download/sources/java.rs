use serde::Deserialize;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Deserialize, Debug, Clone)]
pub struct JavaPackage {
    pub id: String,
    pub distribution: String,
    pub major_version: u32,
    pub java_version: String,
    pub architecture: String,
    pub package_type: String,
    pub filename: String,
    #[serde(default)]
    pub size: Option<i64>,
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

    // Detect if we are running on a musl-based system (like Alpine) or glibc
    let libc_type = if Path::new("/lib/ld-musl-x86_64.so.1").exists()
        || Path::new("/lib/ld-musl-aarch64.so.1").exists()
        || Path::new("/lib64/ld-musl-x86_64.so.1").exists()
    {
        "musl"
    } else {
        "glibc"
    };

    let url = format!(
        "https://api.foojay.io/disco/v3.0/packages?operating_system=linux&libc_type={}&architecture={}&package_type=jdk&release_status=ga&latest=available&archive_type=tar.gz",
        libc_type, arch
    );

    let response =
        reqwest::blocking::get(url).map_err(|e| format!("Failed to fetch versions: {}", e))?;

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
        b.major_version
            .cmp(&a.major_version)
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
    let redirect_url = format!(
        "https://api.foojay.io/disco/v3.0/ids/{}/redirect",
        package_id
    );

    let client = match reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            progress_callback(JavaDownloadProgress::Error(format!("HTTP client build error: {}", e)));
            return;
        }
    };

    let mut response = match client.get(&redirect_url).send() {
        Ok(r) => r,
        Err(e) => {
            progress_callback(JavaDownloadProgress::Error(format!(
                "Failed to request Java package: {}",
                e
            )));
            return;
        }
    };

    if !response.status().is_success() {
        progress_callback(JavaDownloadProgress::Error(format!(
            "Download failed with HTTP status {}",
            response.status()
        )));
        return;
    }

    let final_url = response.url().as_str().to_string();
    let clean_url = final_url.split('?').next().unwrap_or(&final_url);
    let clean_url = clean_url.split('#').next().unwrap_or(clean_url);
    let filename = clean_url
        .split('/')
        .next_back()
        .filter(|s| !s.is_empty())
        .unwrap_or("java_runtime.tar.gz");

    if let Err(e) = fs::create_dir_all(target_dir) {
        progress_callback(JavaDownloadProgress::Error(e.to_string()));
        return;
    }
    let download_path = target_dir.join(filename);

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

    // Ensure all downloaded bytes are flushed to disk and file handle is closed before extraction
    drop(file);

    progress_callback(JavaDownloadProgress::Extracting);

    let mut magic = [0u8; 4];
    let is_zip = if let Ok(mut f) = fs::File::open(&download_path) {
        let n = f.read(&mut magic).unwrap_or(0);
        if n >= 4 && magic[0] == 0x50 && magic[1] == 0x4B && magic[2] == 0x03 && magic[3] == 0x04 {
            true
        } else if n >= 2 && magic[0] == 0x1F && magic[1] == 0x8B {
            false
        } else {
            let mut prefix = vec![0u8; 200];
            if let Ok(mut f_start) = fs::File::open(&download_path) {
                let len = f_start.read(&mut prefix).unwrap_or(0);
                let text = String::from_utf8_lossy(&prefix[..len]);
                if text.contains("<html") || text.contains("<!DOCTYPE") || text.contains("Forbidden") || text.contains("Denied") {
                    progress_callback(JavaDownloadProgress::Error(format!(
                        "Download returned server error page instead of archive:\n{}",
                        text.lines().take(3).collect::<Vec<_>>().join(" ")
                    )));
                    let _ = fs::remove_file(&download_path);
                    return;
                }
            }
            filename.to_lowercase().ends_with(".zip")
        }
    } else {
        filename.to_lowercase().ends_with(".zip")
    };

    let output = if is_zip {
        Command::new("unzip")
            .arg("-q")
            .arg("-o")
            .arg(&download_path)
            .arg("-d")
            .arg(target_dir)
            .output()
    } else {
        Command::new("tar")
            .arg("-xf")
            .arg(&download_path)
            .arg("-C")
            .arg(target_dir)
            .output()
    };

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            progress_callback(JavaDownloadProgress::Error(format!(
                "Extraction tool failed: {}. Ensure 'tar' or 'unzip' is installed.",
                e
            )));
            let _ = fs::remove_file(&download_path);
            return;
        }
    };

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
        progress_callback(JavaDownloadProgress::Error(format!(
            "Extraction failed: {}",
            if err_msg.trim().is_empty() {
                String::from_utf8_lossy(&output.stdout).to_string()
            } else {
                err_msg
            }
        )));
        let _ = fs::remove_file(&download_path);
        return;
    }

    let _ = fs::remove_file(&download_path);

    if let Ok(entries) = fs::read_dir(target_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && (path.join("bin/java").exists() || path.join("Contents/Home/bin/java").exists()) {
                progress_callback(JavaDownloadProgress::Finished(path));
                return;
            }
        }
    }

    progress_callback(JavaDownloadProgress::Error(
        "Extraction finished but could not locate 'bin/java' in target directory".to_string(),
    ));
}
