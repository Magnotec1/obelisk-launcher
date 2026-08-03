use crate::backend::auth::error::AuthError;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// CLIENT_ID is now passed as a parameter from config.

const MS_DEVICE_CODE_URL: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const MS_TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBOX_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_AUTH_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC_AUTH_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
const USER_AGENT: &str = "obelisk-launcher-rs (github.com/magnotec/obelisk-launcher)";

fn build_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

// ─── Serde Structs ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    pub message: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct MsTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct MsTokenError {
    error: String,
    error_description: Option<String>,
}

#[derive(Debug, Serialize)]
struct XboxAuthRequest {
    #[serde(rename = "Properties")]
    properties: XboxAuthProperties,
    #[serde(rename = "RelyingParty")]
    relying_party: String,
    #[serde(rename = "TokenType")]
    token_type: String,
}

#[derive(Debug, Serialize)]
struct XboxAuthProperties {
    #[serde(rename = "AuthMethod")]
    auth_method: String,
    #[serde(rename = "SiteName")]
    site_name: String,
    #[serde(rename = "RpsTicket")]
    rps_ticket: String,
}

#[derive(Debug, Deserialize)]
struct XboxResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: XboxDisplayClaims,
}

#[derive(Debug, Deserialize)]
struct XboxDisplayClaims {
    xui: Vec<XuiEntry>,
}

#[derive(Debug, Deserialize)]
struct XuiEntry {
    uhs: String,
}

#[derive(Debug, Serialize)]
struct XstsAuthRequest {
    #[serde(rename = "Properties")]
    properties: XstsAuthProperties,
    #[serde(rename = "RelyingParty")]
    relying_party: String,
    #[serde(rename = "TokenType")]
    token_type: String,
}

#[derive(Debug, Serialize)]
struct XstsAuthProperties {
    #[serde(rename = "SandboxId")]
    sandbox_id: String,
    #[serde(rename = "UserTokens")]
    user_tokens: Vec<String>,
}

#[derive(Debug, Serialize)]
struct McAuthRequest {
    #[serde(rename = "identityToken")]
    identity_token: String,
}

#[derive(Debug, Deserialize)]
struct McAuthResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McCape {
    pub id: String,
    pub state: String, // e.g. "ACTIVE" or "INACTIVE"
    pub url: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McSkin {
    pub id: String,
    pub state: String,
    pub url: String,
    pub variant: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub skins: Vec<McSkin>,
    #[serde(default)]
    pub capes: Vec<McCape>,
}

// ─── Account Type ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccountType {
    Microsoft,
    Offline,
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Microsoft
    }
}

// ─── Account (stored in config) ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub refresh_token: String,
    pub token_expiry: u64,
    #[serde(default)]
    pub account_type: AccountType,
}

// ─── Helpers ────────────────────────────────────────────────────────────────

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ─── Public API ─────────────────────────────────────────────────────────────

/// Step 1: Request a device code from Microsoft.
pub fn start_device_code_flow(client_id: &str) -> Result<DeviceCodeResponse, AuthError> {
    let client = build_client();

    let resp = client
        .post(MS_DEVICE_CODE_URL)
        .form(&[
            ("client_id", client_id),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .map_err(|e| AuthError::Network(format!("Failed to request device code: {}", e)))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        return Err(AuthError::DeviceCodeRequestFailed(format!("({}) {}", status, body)));
    }

    resp.json::<DeviceCodeResponse>()
        .map_err(|e| AuthError::DeviceCodeRequestFailed(format!("Failed to parse response: {}", e)))
}

/// Step 2: Poll Microsoft until the user completes auth (blocking).
pub fn poll_for_ms_token(
    client_id: &str,
    device_code: &str,
    interval: u64,
    expires_in: u64,
) -> Result<(String, String), AuthError> {
    let client = build_client();
    let deadline = now_secs() + expires_in;
    let poll_interval = Duration::from_secs(interval.max(5));

    loop {
        if now_secs() >= deadline {
            return Err(AuthError::DeviceCodeExpired);
        }

        std::thread::sleep(poll_interval);

        let resp = client
            .post(MS_TOKEN_URL)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", client_id),
                ("device_code", device_code),
            ])
            .send()
            .map_err(|e| AuthError::Network(format!("Token poll failed: {}", e)))?;

        let status = resp.status();
        let body = resp.text().unwrap_or_default();

        if status.is_success() {
            let token: MsTokenResponse =
                serde_json::from_str(&body).map_err(|e| AuthError::Network(format!("Failed to parse token: {}", e)))?;
            return Ok((token.access_token, token.refresh_token.unwrap_or_default()));
        }

        if let Ok(err) = serde_json::from_str::<MsTokenError>(&body) {
            match err.error.as_str() {
                "authorization_pending" => continue,
                "slow_down" => {
                    std::thread::sleep(Duration::from_secs(5));
                    continue;
                }
                "authorization_declined" => return Err(AuthError::AuthorizationDeclined),
                "expired_token" => return Err(AuthError::DeviceCodeExpired),
                other => {
                    return Err(AuthError::Network(format!(
                        "Auth error: {}: {}",
                        other,
                        err.error_description.unwrap_or_default()
                    )))
                }
            }
        }

        return Err(AuthError::Network(format!("Unexpected response ({}): {}", status, body)));
    }
}

/// Steps 3-6: MS token → Xbox Live → XSTS → Minecraft → Profile.
pub fn complete_auth(ms_access_token: &str, ms_refresh_token: &str) -> Result<Account, AuthError> {
    let (xbox_token, user_hash) = authenticate_xbox_live(ms_access_token)?;
    let (xsts_token, _) = authenticate_xsts(&xbox_token)?;
    let (mc_token, expires_in) = authenticate_minecraft(&xsts_token, &user_hash)?;
    let profile = get_minecraft_profile(&mc_token)?;

    Ok(Account {
        username: profile.name,
        uuid: profile.id,
        access_token: mc_token,
        refresh_token: ms_refresh_token.to_string(),
        token_expiry: now_secs() + expires_in,
        account_type: AccountType::Microsoft,
    })
}

/// Refresh using a stored Microsoft refresh token.
pub fn refresh_auth(client_id: &str, refresh_token: &str) -> Result<Account, AuthError> {
    let client = build_client();

    let resp = client
        .post(MS_TOKEN_URL)
        .form(&[
            ("client_id", client_id),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .map_err(|e| AuthError::TokenRefreshFailed(e.to_string()))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        return Err(AuthError::TokenRefreshFailed(format!("({}) {}", status, body)));
    }

    let token: MsTokenResponse = resp
        .json()
        .map_err(|e| AuthError::TokenRefreshFailed(format!("Failed to parse response: {}", e)))?;

    let new_refresh = token
        .refresh_token
        .unwrap_or_else(|| refresh_token.to_string());

    complete_auth(&token.access_token, &new_refresh)
}

// ─── Internal Steps ─────────────────────────────────────────────────────────

fn authenticate_xbox_live(ms_token: &str) -> Result<(String, String), AuthError> {
    let client = build_client();

    let req = XboxAuthRequest {
        properties: XboxAuthProperties {
            auth_method: "RPS".to_string(),
            site_name: "user.auth.xboxlive.com".to_string(),
            rps_ticket: format!("d={}", ms_token),
        },
        relying_party: "http://auth.xboxlive.com".to_string(),
        token_type: "JWT".to_string(),
    };

    let resp = client
        .post(XBOX_AUTH_URL)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("x-xbl-contract-version", "2")
        .json(&req)
        .send()
        .map_err(|e| AuthError::XboxAuthFailed(e.to_string()))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        return Err(AuthError::XboxAuthFailed(format!("({}) {}", status, body)));
    }

    let xbox: XboxResponse = resp
        .json()
        .map_err(|e| AuthError::XboxAuthFailed(format!("Failed to parse Xbox response: {}", e)))?;

    let uhs = xbox
        .display_claims
        .xui
        .first()
        .ok_or_else(|| AuthError::XboxAuthFailed("No Xbox user hash in response".to_string()))?
        .uhs
        .clone();

    Ok((xbox.token, uhs))
}

fn authenticate_xsts(xbox_token: &str) -> Result<(String, String), AuthError> {
    let client = build_client();

    let req = XstsAuthRequest {
        properties: XstsAuthProperties {
            sandbox_id: "RETAIL".to_string(),
            user_tokens: vec![xbox_token.to_string()],
        },
        relying_party: "rp://api.minecraftservices.com/".to_string(),
        token_type: "JWT".to_string(),
    };

    let resp = client
        .post(XSTS_AUTH_URL)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("x-xbl-contract-version", "2")
        .json(&req)
        .send()
        .map_err(|e| AuthError::XstsAuthFailed(e.to_string()))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        if body.contains("2148916233") {
            return Err(AuthError::NoXboxAccount);
        }
        if body.contains("2148916238") {
            return Err(AuthError::ChildAccountRestriction);
        }
        return Err(AuthError::XstsAuthFailed(format!("({}) {}", status, body)));
    }

    let xsts: XboxResponse = resp
        .json()
        .map_err(|e| AuthError::XstsAuthFailed(format!("Failed to parse XSTS response: {}", e)))?;

    let uhs = xsts
        .display_claims
        .xui
        .first()
        .ok_or_else(|| AuthError::XstsAuthFailed("No user hash in XSTS response".to_string()))?
        .uhs
        .clone();

    Ok((xsts.token, uhs))
}

fn authenticate_minecraft(xsts_token: &str, user_hash: &str) -> Result<(String, u64), AuthError> {
    let client = build_client();

    let req = McAuthRequest {
        identity_token: format!("XBL3.0 x={};{}", user_hash, xsts_token),
    };

    let resp = client
        .post(MC_AUTH_URL)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&req)
        .send()
        .map_err(|e| AuthError::MinecraftAuthFailed(e.to_string()))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        return Err(AuthError::MinecraftAuthFailed(format!("({}) {}", status, body)));
    }

    let mc: McAuthResponse = resp
        .json()
        .map_err(|e| AuthError::MinecraftAuthFailed(format!("Failed to parse MC auth response: {}", e)))?;

    Ok((mc.access_token, mc.expires_in))
}

pub fn get_minecraft_profile(mc_token: &str) -> Result<McProfile, AuthError> {
    let client = build_client();

    let resp = client
        .get(MC_PROFILE_URL)
        .header("Authorization", format!("Bearer {}", mc_token))
        .header("Accept", "application/json")
        .send()
        .map_err(|e| AuthError::ProfileRequestFailed(e.to_string()))?;

    let status = resp.status();
    if !status.is_success() {
        if status.as_u16() == 404 {
            return Err(AuthError::NoMinecraftLicense);
        }
        let body = resp.text().unwrap_or_default();
        return Err(AuthError::ProfileRequestFailed(format!("({}) {}", status, body)));
    }

    resp.json::<McProfile>()
        .map_err(|e| AuthError::ProfileRequestFailed(format!("Failed to parse profile: {}", e)))
}

const MC_SELECT_CAPE_URL: &str = "https://api.minecraftservices.com/minecraft/profile/capes/active";

/// Select an active cape or equip/unequip cape via Minecraft API.
pub fn select_minecraft_cape(mc_token: &str, cape_id: Option<&str>) -> Result<(), AuthError> {
    let client = build_client();

    let resp = if let Some(id) = cape_id {
        let body = serde_json::json!({ "capeId": id });
        client
            .put(MC_SELECT_CAPE_URL)
            .header("Authorization", format!("Bearer {}", mc_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
    } else {
        client
            .delete(MC_SELECT_CAPE_URL)
            .header("Authorization", format!("Bearer {}", mc_token))
            .send()
    }.map_err(|e| AuthError::ProfileRequestFailed(e.to_string()))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        return Err(AuthError::ProfileRequestFailed(format!("Cape update failed ({}): {}", status, body)));
    }

    Ok(())
}
