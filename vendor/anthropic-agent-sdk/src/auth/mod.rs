//! OAuth authentication module for Claude
//!
//! Provides OAuth 2.0 authentication for Claude using PKCE (Proof Key for Code Exchange).
//!
//! # Overview
//!
//! This module implements the Authorization Code flow with PKCE for authenticating
//! with Claude's OAuth endpoints. The flow works as follows:
//!
//! 1. Generate a code verifier and challenge (PKCE)
//! 2. Open browser to authorization URL with code challenge
//! 3. User authenticates and copies the authorization code
//! 4. Exchange code + verifier for access token
//! 5. Cache token for future use
//!
//! # Example
//!
//! ```no_run
//! use anthropic_agent_sdk::auth::{OAuthClient, TokenStorage};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create OAuth client with default settings
//!     let client = OAuthClient::new()?;
//!
//!     // Attempt to load cached token or start OAuth flow
//!     let token = client.authenticate().await?;
//!
//!     // Use token for API calls
//!     println!("Authenticated! Token expires at: {:?}", token.expires_at);
//!     Ok(())
//! }
//! ```
//!
//! # Token Storage
//!
//! Tokens are cached to disk in the platform-specific config directory by default
//! (e.g., `~/Library/Application Support/claude-sdk/` on macOS).
//! The storage location can be customized via [`TokenStorage`].
//!
//! # Security
//!
//! - PKCE prevents authorization code interception attacks
//! - Tokens are stored with user-only permissions (600)
//! - Refresh tokens are used when available to avoid re-authentication

mod oauth;
mod token;

pub use oauth::{AuthResult, OAuthClient, OAuthClientBuilder, OAuthConfig};
pub use token::{TokenError, TokenInfo, TokenStorage};
