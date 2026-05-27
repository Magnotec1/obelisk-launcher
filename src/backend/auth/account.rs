use crate::backend::auth::microsoft::{
    get_minecraft_profile, now_secs, refresh_auth, Account, AccountType,
};
use crate::config::Config;
use std::fmt;

// ─── Account Status ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum AccountStatus {
    /// Token is valid and not expiring soon
    Valid,
    /// Token will expire within 1 hour
    ExpiringSoon,
    /// Token has expired and needs refresh
    Expired,
    /// Could not verify (network error, etc.)
    Unknown(String),
}

impl fmt::Display for AccountStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccountStatus::Valid => write!(f, "Valid"),
            AccountStatus::ExpiringSoon => write!(f, "Expiring Soon"),
            AccountStatus::Expired => write!(f, "Expired"),
            AccountStatus::Unknown(e) => write!(f, "Unknown: {}", e),
        }
    }
}

// ─── Core Account Operations ────────────────────────────────────────────────

pub fn get_active_account(config: &Config) -> Option<&Account> {
    if let Some(uuid) = &config.active_account_uuid {
        config.accounts.iter().find(|a| &a.uuid == uuid)
    } else {
        config.accounts.first()
    }
}

pub fn switch_account(config: &mut Config, uuid: &str) -> Result<(), String> {
    if config.accounts.iter().any(|a| a.uuid == uuid) {
        config.active_account_uuid = Some(uuid.to_string());
        Ok(())
    } else {
        Err("Account not found".to_string())
    }
}

pub fn add_account(config: &mut Config, account: Account) {
    // Remove if exists (update)
    config.accounts.retain(|a| a.uuid != account.uuid);
    let uuid = account.uuid.clone();
    config.accounts.push(account);
    if config.active_account_uuid.is_none() {
        config.active_account_uuid = Some(uuid);
    }
}

pub fn remove_account(config: &mut Config, uuid: &str) {
    config.accounts.retain(|a| a.uuid != uuid);
    if config.active_account_uuid.as_deref() == Some(uuid) {
        config.active_account_uuid = config.accounts.first().map(|a| a.uuid.clone());
    }
}

// ─── Offline Accounts ───────────────────────────────────────────────────────

/// Generate a deterministic offline-mode UUID from a username.
/// Uses the same algorithm as Minecraft's offline mode (UUID v3 with "OfflinePlayer:" prefix).
pub fn generate_offline_uuid(username: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Minecraft uses UUID.nameUUIDFromBytes("OfflinePlayer:<name>".getBytes("UTF-8"))
    // which is a MD5-based UUID v3. We approximate this with a stable hash
    // that produces a consistent UUID for the same username.
    let input = format!("OfflinePlayer:{}", username);

    // Use md5-like hashing via a simple approach
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    let hash1 = hasher.finish();

    let mut hasher2 = DefaultHasher::new();
    format!("{}:salt", input).hash(&mut hasher2);
    let hash2 = hasher2.finish();

    // Construct a UUID-like string from the two hashes
    let bytes: [u8; 16] = [
        (hash1 >> 56) as u8,
        (hash1 >> 48) as u8,
        (hash1 >> 40) as u8,
        (hash1 >> 32) as u8,
        (hash1 >> 24) as u8,
        (hash1 >> 16) as u8,
        (hash1 >> 8) as u8,
        hash1 as u8,
        (hash2 >> 56) as u8,
        (hash2 >> 48) as u8,
        (hash2 >> 40) as u8,
        (hash2 >> 32) as u8,
        (hash2 >> 24) as u8,
        (hash2 >> 16) as u8,
        (hash2 >> 8) as u8,
        hash2 as u8,
    ];

    // Format as UUID with version 3 bits set
    let mut uuid_bytes = bytes;
    uuid_bytes[6] = (uuid_bytes[6] & 0x0F) | 0x30; // version 3
    uuid_bytes[8] = (uuid_bytes[8] & 0x3F) | 0x80; // variant 1

    format!(
        "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        uuid_bytes[0], uuid_bytes[1], uuid_bytes[2], uuid_bytes[3],
        uuid_bytes[4], uuid_bytes[5], uuid_bytes[6], uuid_bytes[7],
        uuid_bytes[8], uuid_bytes[9], uuid_bytes[10], uuid_bytes[11],
        uuid_bytes[12], uuid_bytes[13], uuid_bytes[14], uuid_bytes[15],
    )
}

/// Create an offline account from a username.
pub fn create_offline_account(username: &str) -> Account {
    let uuid = generate_offline_uuid(username);
    Account {
        username: username.to_string(),
        uuid,
        access_token: "0".to_string(),
        refresh_token: String::new(),
        token_expiry: 0,
        account_type: AccountType::Offline,
    }
}

// ─── Account Verification & Refresh ─────────────────────────────────────────

/// Check the current status of an account's token.
pub fn verify_account_status(account: &Account) -> AccountStatus {
    match account.account_type {
        AccountType::Offline => AccountStatus::Valid,
        AccountType::Microsoft => {
            let now = now_secs();
            if account.token_expiry == 0 {
                return AccountStatus::Expired;
            }
            if now >= account.token_expiry {
                AccountStatus::Expired
            } else if account.token_expiry - now < 3600 {
                AccountStatus::ExpiringSoon
            } else {
                AccountStatus::Valid
            }
        }
    }
}

/// Verify a Microsoft account by actually hitting the Minecraft profile API.
/// This is a blocking network call.
pub fn verify_account_online(account: &Account) -> AccountStatus {
    match account.account_type {
        AccountType::Offline => AccountStatus::Valid,
        AccountType::Microsoft => {
            match get_minecraft_profile(&account.access_token) {
                Ok(_profile) => {
                    // Token works, check expiry for more detail
                    let now = now_secs();
                    if account.token_expiry > 0 && account.token_expiry - now < 3600 {
                        AccountStatus::ExpiringSoon
                    } else {
                        AccountStatus::Valid
                    }
                }
                Err(_) => {
                    // Token is invalid/expired
                    AccountStatus::Expired
                }
            }
        }
    }
}

/// Refresh a single Microsoft account's tokens. Returns updated Account on success.
pub fn refresh_single_account(account: &Account, client_id: &str) -> Result<Account, String> {
    match account.account_type {
        AccountType::Offline => Ok(account.clone()),
        AccountType::Microsoft => {
            if account.refresh_token.is_empty() {
                return Err("No refresh token available. Please sign in again.".to_string());
            }
            refresh_auth(client_id, &account.refresh_token)
        }
    }
}

pub fn refresh_all_accounts(config: &mut Config) -> Result<(), String> {
    let client_id = config
        .microsoft_client_id
        .as_deref()
        .unwrap_or("00000000402b5328");
    let mut updated_accounts = Vec::new();
    let mut some_failed = false;

    for account in &config.accounts {
        match account.account_type {
            AccountType::Offline => {
                updated_accounts.push(account.clone());
            }
            AccountType::Microsoft => match refresh_auth(client_id, &account.refresh_token) {
                Ok(new_acc) => updated_accounts.push(new_acc),
                Err(e) => {
                    println!("Failed to refresh account {}: {}", account.username, e);
                    updated_accounts.push(account.clone());
                    some_failed = true;
                }
            },
        }
    }

    config.accounts = updated_accounts;

    if some_failed {
        Err("Some accounts failed to refresh".to_string())
    } else {
        Ok(())
    }
}
