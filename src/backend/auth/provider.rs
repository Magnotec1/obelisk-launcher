use crate::backend::auth::error::AuthError;
use crate::backend::auth::microsoft::{Account, AccountType};

pub trait AuthProvider: Send + Sync {
    fn account_type(&self) -> AccountType;
    fn refresh(&self, account: &Account) -> Result<Account, AuthError>;
}

pub struct MicrosoftAuthProvider {
    client_id: String,
}

impl MicrosoftAuthProvider {
    pub fn new(client_id: String) -> Self {
        Self { client_id }
    }
}

impl AuthProvider for MicrosoftAuthProvider {
    fn account_type(&self) -> AccountType {
        AccountType::Microsoft
    }

    fn refresh(&self, account: &Account) -> Result<Account, AuthError> {
        if account.refresh_token.is_empty() {
            return Err(AuthError::TokenRefreshFailed("No refresh token available.".to_string()));
        }
        crate::backend::auth::microsoft::refresh_auth(&self.client_id, &account.refresh_token)
    }
}

pub struct OfflineAuthProvider;

impl AuthProvider for OfflineAuthProvider {
    fn account_type(&self) -> AccountType {
        AccountType::Offline
    }

    fn refresh(&self, account: &Account) -> Result<Account, AuthError> {
        Ok(account.clone())
    }
}
