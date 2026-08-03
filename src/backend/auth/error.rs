use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum AuthError {
    #[error("Device code authorization request failed: {0}")]
    DeviceCodeRequestFailed(String),

    #[error("Device code expired. Please try again.")]
    DeviceCodeExpired,

    #[error("Authorization was declined by the user.")]
    AuthorizationDeclined,

    #[error("Xbox Live authentication failed: {0}")]
    XboxAuthFailed(String),

    #[error("This account has no Xbox account. Create one at xbox.com.")]
    NoXboxAccount,

    #[error("Child account — an adult must add it to a Microsoft family.")]
    ChildAccountRestriction,

    #[error("XSTS authentication failed: {0}")]
    XstsAuthFailed(String),

    #[error("Minecraft auth failed: {0}")]
    MinecraftAuthFailed(String),

    #[error("This Microsoft account does not own Minecraft Java Edition.")]
    NoMinecraftLicense,

    #[error("Profile request failed: {0}")]
    ProfileRequestFailed(String),

    #[error("Token refresh failed: {0}")]
    TokenRefreshFailed(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Account error: {0}")]
    AccountError(String),
}
