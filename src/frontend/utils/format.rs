use gtk::glib;

/// Formats a byte count into a human-readable string (KB, MB, GB).
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

/// Formats a playtime in seconds into a friendly human-readable duration.
pub fn format_playtime(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m", minutes)
    } else if seconds > 0 {
        format!("{}s", seconds)
    } else {
        "Never played".to_string()
    }
}

/// Formats a duration in seconds without "Never played" fallback.
pub fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m", minutes)
    } else {
        format!("{}s", seconds)
    }
}

/// Formats large numerical counts (e.g. downloads) into compact numbers (1.2K, 3.4M).
pub fn format_downloads(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Escapes special markup characters for Pango / GTK labels.
pub fn escape_pango(text: &str) -> String {
    glib::markup_escape_text(text).to_string()
}

/// Formats millisecond epoch timestamps into days elapsed.
pub fn format_timestamp(ms: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let days = (now - ms).max(0) / (1000 * 60 * 60 * 24);
    if days == 0 {
        "today".to_string()
    } else if days == 1 {
        "1 day ago".to_string()
    } else {
        format!("{} days ago", days)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(512), "0.5 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 * 1024 * 2), "2.00 GB");
    }

    #[test]
    fn test_format_playtime() {
        assert_eq!(format_playtime(0), "Never played");
        assert_eq!(format_playtime(45), "45s");
        assert_eq!(format_playtime(125), "2m");
        assert_eq!(format_playtime(3665), "1h 1m");
    }

    #[test]
    fn test_format_downloads() {
        assert_eq!(format_downloads(500), "500");
        assert_eq!(format_downloads(1500), "1.5K");
        assert_eq!(format_downloads(2_500_000), "2.5M");
    }

    #[test]
    fn test_escape_pango() {
        assert_eq!(escape_pango("Hello <World> & Co"), "Hello &lt;World&gt; &amp; Co");
    }
}
