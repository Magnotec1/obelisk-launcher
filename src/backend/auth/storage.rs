use keyring::Entry;

const SERVICE_NAME: &str = "obelisk-launcher";

pub struct CredentialStorage;

impl CredentialStorage {
    /// Save a refresh token to the system keyring.
    pub fn store_refresh_token(uuid: &str, token: &str) -> Result<(), String> {
        let entry = Entry::new(SERVICE_NAME, uuid)
            .map_err(|e| format!("Keyring entry error: {}", e))?;
        entry
            .set_password(token)
            .map_err(|e| format!("Failed to store token in keyring: {}", e))
    }

    /// Retrieve a refresh token from the system keyring.
    pub fn get_refresh_token(uuid: &str) -> Result<String, String> {
        let entry = Entry::new(SERVICE_NAME, uuid)
            .map_err(|e| format!("Keyring entry error: {}", e))?;
        entry
            .get_password()
            .map_err(|e| format!("Failed to retrieve token from keyring: {}", e))
    }

    /// Delete a refresh token from the system keyring.
    pub fn delete_refresh_token(uuid: &str) -> Result<(), String> {
        let entry = Entry::new(SERVICE_NAME, uuid)
            .map_err(|e| format!("Keyring entry error: {}", e))?;
        entry
            .delete_credential()
            .map_err(|e| format!("Failed to delete token from keyring: {}", e))
    }
}
