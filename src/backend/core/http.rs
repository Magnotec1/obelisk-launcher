use std::sync::LazyLock;
use std::time::Duration;

pub const USER_AGENT: &str = "obelisk-launcher-rs (github.com/magnotec/obelisk-launcher)";

/// Global reusable HTTP client with connection pooling, keep-alive, and standard timeout.
pub static HTTP_CLIENT: LazyLock<reqwest::blocking::Client> = LazyLock::new(|| {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(Duration::from_secs(90))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
});

/// Returns a reference to the shared blocking HTTP client.
pub fn client() -> &'static reqwest::blocking::Client {
    &HTTP_CLIENT
}
