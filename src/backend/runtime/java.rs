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

fn scan_flatpak_dir(base: &Path, versions: &mut Vec<JavaInstance>) {
    if !base.exists() {
        return;
    }
    for entry in walkdir::WalkDir::new(base)
        .max_depth(9)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let path = entry.path();
            if path.file_name().map_or(false, |name| name == "java") {
                if path.parent().map_or(false, |parent| parent.file_name().map_or(false, |p_name| p_name == "bin")) {
                    if let Some(mut instance) = probe_java(path) {
                        instance.source = JavaSource::System;
                        if !versions.iter().any(|v| v.path == instance.path) {
                            versions.push(instance);
                        }
                    }
                }
            }
        }
    }
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
                        } else {
                            // The installation folder has a bin/java, but it is unexecutable (e.g. wrong dynamic linker/libc type)
                            // or corrupt. Let's delete it so it can heal.
                            let _ = fs::remove_dir_all(&path);
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

    // 3. Scan Flatpak OpenJDK extensions inside the sandbox & host Flatpak runtimes
    scan_flatpak_dir(Path::new("/usr/lib/sdk"), &mut versions);
    scan_flatpak_dir(Path::new("/var/lib/flatpak/runtime"), &mut versions);
    if let Ok(entries) = fs::read_dir("/home") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let user_flatpak = path.join(".local/share/flatpak/runtime");
                scan_flatpak_dir(&user_flatpak, &mut versions);
            }
        }
    }

    // 4. Check "java" in PATH
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

pub fn probe_java(path: &Path) -> Option<JavaInstance> {
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

/// Parse a Java version string and extract the major version as an integer.
/// E.g. "1.8.0_421" -> 8, "17.0.12" -> 17, "21.0.3" -> 21.
pub fn get_java_major_version(version_str: &str) -> Option<u32> {
    let parts: Vec<&str> = version_str.split('.').collect();
    if parts.is_empty() {
        return None;
    }
    let first = parts[0];
    if first == "1" {
        if parts.len() > 1 {
            let second = parts[1];
            let clean: String = second.chars().take_while(|c| c.is_ascii_digit()).collect();
            clean.parse::<u32>().ok()
        } else {
            None
        }
    } else {
        let clean: String = first.chars().take_while(|c| c.is_ascii_digit()).collect();
        clean.parse::<u32>().ok()
    }
}/// Returns the required Java major version based on the Minecraft version.
pub fn get_required_java_version(mc_version: &str) -> u32 {
    let parts: Vec<&str> = mc_version.split('.').collect();
    if !parts.is_empty() {
        let first = parts[0];
        if first == "1" {
            if parts.len() >= 2 {
                if let Ok(minor) = parts[1].parse::<u32>() {
                    if minor >= 26 {
                        return 25;
                    }
                    if minor == 20 {
                        // Minecraft 1.20.5+ requires Java 21
                        if parts.len() >= 3 {
                            if let Some(patch_str) = parts.get(2) {
                                let clean_patch: String = patch_str.chars().take_while(|c| c.is_ascii_digit()).collect();
                                if let Ok(patch) = clean_patch.parse::<u32>() {
                                    if patch >= 5 {
                                        return 21;
                                    }
                                }
                            }
                        }
                        return 17;
                    }
                    if minor >= 21 {
                        return 21;
                    } else if minor >= 18 {
                        return 17;
                    } else if minor == 17 {
                        return 16;
                    } else {
                        return 8;
                    }
                }
            }
        } else {
            // Check for year-based versions (e.g. 26.1) or snapshots (e.g. 26w02a)
            let leading_digits: String = first.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(year_ver) = leading_digits.parse::<u32>() {
                if year_ver >= 26 {
                    return 25;
                } else if year_ver >= 24 {
                    return 21;
                } else if year_ver >= 18 {
                    return 17;
                } else if year_ver == 17 {
                    return 16;
                } else {
                    return 8;
                }
            }
        }
    }
    // Default to Java 25 for modern/unknown versions
    25
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_required_java_version() {
        assert_eq!(get_required_java_version("1.8.9"), 8);
        assert_eq!(get_required_java_version("1.12.2"), 8);
        assert_eq!(get_required_java_version("1.16.5"), 8);
        assert_eq!(get_required_java_version("1.17.1"), 16);
        assert_eq!(get_required_java_version("1.18.2"), 17);
        assert_eq!(get_required_java_version("1.20.1"), 17);
        assert_eq!(get_required_java_version("1.20.4"), 17);
        assert_eq!(get_required_java_version("1.20.5"), 21);
        assert_eq!(get_required_java_version("1.20.6"), 21);
        assert_eq!(get_required_java_version("1.21"), 21);
        assert_eq!(get_required_java_version("1.21.4"), 21);
        assert_eq!(get_required_java_version("1.25"), 21);
        assert_eq!(get_required_java_version("1.26"), 25);
        assert_eq!(get_required_java_version("1.27"), 25);
        assert_eq!(get_required_java_version("26.1"), 25);
        assert_eq!(get_required_java_version("26.1-snapshot-1"), 25);
        assert_eq!(get_required_java_version("24w14a"), 21);
        assert_eq!(get_required_java_version("26w02a"), 25);
    }
}
