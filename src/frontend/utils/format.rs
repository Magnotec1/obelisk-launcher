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
