use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JavaSource {
    System,
    Launcher,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JavaInstance {
    pub name: String,
    pub path: PathBuf,
    pub version: String,
    pub source: JavaSource,
}

pub fn find_java_versions(launcher_java_dir: Option<&Path>) -> Vec<JavaInstance> {
    let mut versions = Vec::new();

    // 1. Check launcher-specific Java installations
    if let Some(java_dir) = launcher_java_dir {
        if let Ok(entries) = fs::read_dir(java_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let java_bin = path.join("bin/java");
                    if java_bin.exists() {
                        if let Some(mut instance) = probe_java(&java_bin) {
                            instance.source = JavaSource::Launcher;
                            versions.push(instance);
                        }
                    }
                }
            }
        }
    }

    // 2. Check common system paths on Linux
    let common_paths = vec![
        PathBuf::from("/usr/lib/jvm"),
        PathBuf::from("/usr/lib64/jvm"),
        PathBuf::from("/usr/java"),
    ];

    for base_path in common_paths {
        if let Ok(entries) = fs::read_dir(base_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let java_bin = path.join("bin/java");
                    if java_bin.exists() {
                        if let Some(mut instance) = probe_java(&java_bin) {
                            instance.source = JavaSource::System;
                            versions.push(instance);
                        }
                    }
                }
            }
        }
    }

    // 3. Check "java" in PATH
    if let Some(path_java) = which_java() {
        if !versions.iter().any(|v| v.path == path_java) {
            if let Some(mut instance) = probe_java(&path_java) {
                instance.source = JavaSource::System;
                versions.push(instance);
            }
        }
    }

    versions.sort_by(|a, b| b.version.cmp(&a.version));
    versions
}

fn which_java() -> Option<PathBuf> {
    Command::new("which")
        .arg("java")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    return Some(PathBuf::from(path_str));
                }
            }
            None
        })
}

fn probe_java(path: &Path) -> Option<JavaInstance> {
    // Try to get version using -version
    // Note: java -version outputs to STDERR
    let output = Command::new(path).arg("-version").output().ok()?;

    let out_str = String::from_utf8_lossy(&output.stderr);
    let first_line = out_str.lines().next()?;

    // Example lines:
    // openjdk version "17.0.12" 2024-07-16
    // java version "1.8.0_421"

    let version = if let Some(idx) = first_line.find('"') {
        let rest = &first_line[idx + 1..];
        if let Some(end_idx) = rest.find('"') {
            rest[..end_idx].to_string()
        } else {
            "Unknown".to_string()
        }
    } else {
        "Unknown".to_string()
    };

    let name = path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "System Java".to_string());

    Some(JavaInstance {
        name,
        path: path.to_path_buf(),
        version,
        source: JavaSource::System, // Default, will be overridden
    })
}
