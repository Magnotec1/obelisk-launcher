pub mod minecraft;
pub mod java;
pub mod modrinth;

pub fn map_reqwest_error(e: reqwest::Error) -> String {
    if e.is_connect() || e.is_timeout() {
        "No internet connection available. Please check your network and try again.".to_string()
    } else {
        let err_str = e.to_string();
        if err_str.contains("dns") || err_str.contains("lookup") || err_str.contains("connection") || err_str.contains("connect") {
            "No internet connection available. Please check your network and try again.".to_string()
        } else {
            format!("Network error: {}", err_str)
        }
    }
}
