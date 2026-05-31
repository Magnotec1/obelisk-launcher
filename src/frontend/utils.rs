pub fn open_instance_subfolder(base_dir: &std::path::Path, subfolder: &str) {
    let target_dir = base_dir.join(subfolder);
    if !target_dir.exists() {
        let _ = std::fs::create_dir_all(&target_dir);
    }

    let _ = std::process::Command::new("xdg-open")
        .arg(&target_dir)
        .spawn();
}

pub fn format_size(size: u64) -> String {
    let mb = size as f64 / 1_048_576.0;
    if mb < 1.0 {
        let kb = size as f64 / 1024.0;
        format!("{:.1} KB", kb)
    } else if mb < 1000.0 {
        format!("{:.1} MB", mb)
    } else {
        let gb = mb / 1024.0;
        format!("{:.2} GB", gb)
    }
}

pub fn format_timestamp(ms: i64) -> String {
    // using chrono if available, else quick calculation
    // let us just format as Date string if possible, or skip it
    // Wait, since we don't have chrono, we can do it manually or skip last played for now, or use `time` crate. Let's just skip last played if it's too complicated without date crate, or format it simply.
    format!(
        "{} days ago",
        (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
            - ms)
            / (1000 * 60 * 60 * 24)
    )
}
