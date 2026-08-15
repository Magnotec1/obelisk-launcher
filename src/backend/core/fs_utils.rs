use std::fs;
use std::io;
use std::path::Path;

/// Atomically writes content to a file by writing to a temporary sibling file and renaming it.
pub fn atomic_write(path: &Path, content: impl AsRef<[u8]>) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = path.with_extension(format!(
        "tmp.{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));

    fs::write(&tmp_path, content)?;
    fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Ensures the directory for a path exists.
pub fn ensure_dir(dir_path: &Path) -> io::Result<()> {
    if !dir_path.exists() {
        fs::create_dir_all(dir_path)?;
    }
    Ok(())
}
