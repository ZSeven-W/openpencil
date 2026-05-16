//! Token storage and management for OAuth authentication

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Errors that can occur during token operations
#[derive(Debug, Error)]
pub enum TokenError {
    /// Token has expired
    #[error("Token has expired")]
    Expired,

    /// Token not found in storage
    #[error("Token not found")]
    NotFound,

    /// I/O error during storage operations
    #[error("Storage I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// OAuth token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    /// Access token for API calls
    pub access_token: String,

    /// Refresh token for obtaining new access tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,

    /// Token type (usually "Bearer")
    #[serde(default = "default_token_type")]
    pub token_type: String,

    /// Scopes granted to this token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,

    /// Unix timestamp when token expires
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
}

fn default_token_type() -> String {
    "Bearer".to_string()
}

impl TokenInfo {
    /// Create a new token info from OAuth response
    #[must_use]
    pub fn new(
        access_token: String,
        refresh_token: Option<String>,
        expires_in: Option<u64>,
        scope: Option<String>,
    ) -> Self {
        let expires_at = expires_in.map(|seconds| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_secs()
                + seconds
        });

        Self {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            scope,
            expires_at,
        }
    }

    /// Check if the token is expired (with 60 second buffer)
    #[must_use]
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_secs();
            // Consider expired if within 60 seconds of expiration
            now + 60 >= expires_at
        } else {
            false
        }
    }

    /// Get the Authorization header value
    #[must_use]
    pub fn authorization_header(&self) -> String {
        format!("{} {}", self.token_type, self.access_token)
    }

    /// Get remaining validity duration, if known
    #[must_use]
    pub fn remaining_validity(&self) -> Option<Duration> {
        self.expires_at.and_then(|expires_at| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_secs();
            if expires_at > now {
                Some(Duration::from_secs(expires_at - now))
            } else {
                None
            }
        })
    }
}

/// Token storage for persisting OAuth tokens
#[derive(Debug, Clone)]
pub struct TokenStorage {
    storage_path: PathBuf,
}

impl Default for TokenStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenStorage {
    /// Create token storage with default path (platform-specific config directory)
    #[must_use]
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("claude-sdk");

        Self {
            storage_path: config_dir.join("oauth_token.json"),
        }
    }

    /// Create token storage with custom path
    #[must_use]
    pub fn with_path(path: PathBuf) -> Self {
        Self { storage_path: path }
    }

    /// Get the storage path
    #[must_use]
    pub fn path(&self) -> &PathBuf {
        &self.storage_path
    }

    /// Load token from storage
    ///
    /// # Errors
    ///
    /// Returns `TokenError::NotFound` if no token exists,
    /// or I/O and JSON errors if reading fails.
    pub fn load(&self) -> Result<TokenInfo, TokenError> {
        if !self.storage_path.exists() {
            return Err(TokenError::NotFound);
        }

        let content = std::fs::read_to_string(&self.storage_path)?;
        let token: TokenInfo = serde_json::from_str(&content)?;

        Ok(token)
    }

    /// Load token if valid (not expired)
    ///
    /// # Errors
    ///
    /// Returns `TokenError::Expired` if token exists but is expired,
    /// or `TokenError::NotFound` if no token exists.
    pub fn load_valid(&self) -> Result<TokenInfo, TokenError> {
        let token = self.load()?;
        if token.is_expired() {
            Err(TokenError::Expired)
        } else {
            Ok(token)
        }
    }

    /// Save token to storage
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written or serialization fails.
    pub fn save(&self, token: &TokenInfo) -> Result<(), TokenError> {
        // Ensure parent directory exists
        if let Some(parent) = self.storage_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(token)?;

        // Write token file
        std::fs::write(&self.storage_path, &content)?;

        // Set restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&self.storage_path, perms)?;
        }

        Ok(())
    }

    /// Delete stored token
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be deleted.
    pub fn delete(&self) -> Result<(), TokenError> {
        if self.storage_path.exists() {
            std::fs::remove_file(&self.storage_path)?;
        }
        Ok(())
    }

    /// Check if a valid token exists
    #[must_use]
    pub fn has_valid_token(&self) -> bool {
        self.load_valid().is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_token_info_new() {
        let token = TokenInfo::new(
            "access123".to_string(),
            Some("refresh456".to_string()),
            Some(3600),
            Some("user:profile".to_string()),
        );

        assert_eq!(token.access_token, "access123");
        assert_eq!(token.refresh_token, Some("refresh456".to_string()));
        assert_eq!(token.token_type, "Bearer");
        assert!(token.expires_at.is_some());
        assert!(!token.is_expired());
    }

    #[test]
    fn test_token_expired() {
        let mut token = TokenInfo::new("access123".to_string(), None, Some(3600), None);

        // Set expiration to past
        token.expires_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - 100,
        );

        assert!(token.is_expired());
    }

    #[test]
    fn test_authorization_header() {
        let token = TokenInfo::new("access123".to_string(), None, None, None);
        assert_eq!(token.authorization_header(), "Bearer access123");
    }

    #[test]
    fn test_token_storage_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("token.json");
        let storage = TokenStorage::with_path(storage_path);

        let token = TokenInfo::new(
            "test_access".to_string(),
            Some("test_refresh".to_string()),
            Some(7200),
            Some("org:create_api_key".to_string()),
        );

        storage.save(&token).unwrap();
        let loaded = storage.load().unwrap();

        assert_eq!(loaded.access_token, "test_access");
        assert_eq!(loaded.refresh_token, Some("test_refresh".to_string()));
    }

    #[test]
    fn test_token_storage_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("nonexistent.json");
        let storage = TokenStorage::with_path(storage_path);

        let result = storage.load();
        assert!(matches!(result, Err(TokenError::NotFound)));
    }

    #[test]
    fn test_has_valid_token() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("token.json");
        let storage = TokenStorage::with_path(storage_path);

        assert!(!storage.has_valid_token());

        let token = TokenInfo::new("access".to_string(), None, Some(3600), None);
        storage.save(&token).unwrap();

        assert!(storage.has_valid_token());
    }
}
