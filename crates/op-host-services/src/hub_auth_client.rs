//! Client for the hub's identity endpoints.
//!
//! The online daemon does not own accounts — op-hub does. This module is the
//! one place that asks it two questions:
//!
//! - `GET /api/v1/session` with the browser's `op_hub_session` cookie
//!   forwarded, answering "who is this browser".
//! - `POST /api/v1/tokens/introspect` with the shared internal secret,
//!   answering "which account does this API token belong to".
//!
//! ## This is an internal call, not a public one
//!
//! The hub sits beside this daemon on a private container network and is
//! addressed as `http://backend:8080`, so this deliberately does NOT use
//! `public_https_client` / `provider_dial`: those exist to screen
//! *caller-supplied* destinations and reject exactly the private addresses
//! this call must reach. The URL here comes from the operator's environment,
//! never from a request, so the relevant hardening is different — no proxy,
//! no redirects, tight timeouts, bounded body.
//!
//! ## Caching
//!
//! Every request would otherwise become two, and the hub's session store is
//! Redis-backed. Verdicts are cached under `SHA-256(credential)` — the
//! plaintext credential is never stored, so a memory dump of this daemon does
//! not yield usable session cookies. Positive session entries live 60s,
//! positive token entries `min(300s, time until the token expires)`, and a
//! definitive negative 15s. Upstream failures are never cached; see
//! [`HubAuthError::is_cacheable`].

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::hub_auth_error::HubAuthError;

/// Base URL of the hub, e.g. `http://backend:8080`.
pub const HUB_BASE_URL_ENV: &str = "OPENPENCIL_HUB_BASE_URL";
/// Shared secret sent as `X-OP-Internal-Auth` on the introspection call.
pub const HUB_INTERNAL_AUTH_ENV: &str = "OPENPENCIL_HUB_INTERNAL_AUTH";
/// Path to a file holding that secret. Preferred over the inline value: an
/// orchestrator mounts a secret rather than putting it in the environment,
/// where it is visible to anything that can read the process table.
pub const HUB_INTERNAL_AUTH_FILE_ENV: &str = "OPENPENCIL_HUB_INTERNAL_AUTH_FILE";

/// Longest secret file this reads. The secret is a short opaque token.
const MAX_INTERNAL_AUTH_BYTES: u64 = 4096;

/// Header carrying the internal shared secret.
pub const INTERNAL_AUTH_HEADER: &str = "X-OP-Internal-Auth";

const SESSION_PATH: &str = "/api/v1/session";
const INTROSPECT_PATH: &str = "/api/v1/tokens/introspect";

/// How long a verified browser session is trusted without re-asking.
///
/// The hub's own session has a 30-minute idle lifetime, so a minute of
/// staleness cannot resurrect a session that was already gone when it was
/// cached, and it bounds how long a *logout* stays invisible here.
const SESSION_POSITIVE_TTL: Duration = Duration::from_secs(60);
/// Ceiling on a positive token entry; the token's own expiry can shorten it.
const TOKEN_POSITIVE_TTL: Duration = Duration::from_secs(300);
/// How long a definitive rejection is remembered.
const NEGATIVE_TTL: Duration = Duration::from_secs(15);

/// Connect + response deadline. A connection thread is blocked on this, so it
/// is also the worst case a browser waits before the daemon answers.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Largest identity body accepted. Both shapes are a handful of short
/// strings; anything larger is not a response this daemon should parse.
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

/// Longest credential this client will hash and forward.
const MAX_CREDENTIAL_BYTES: usize = 4096;

/// Cache ceiling. Each entry is one account's short strings, and eviction is
/// the oldest-deadline entry, so a burst of unknown tokens cannot grow this
/// without bound.
const MAX_CACHE_ENTRIES: usize = 4096;

/// One hub account, as `GET /api/v1/session` reports it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HubUser {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub primary_email: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
}

/// The session envelope. Only `user` is consumed — `csrf_token` is the
/// browser's business and `capabilities` is the hub UI's.
#[derive(Debug, Clone, Deserialize)]
struct SessionEnvelope {
    user: HubUser,
}

/// One API token, as `POST /api/v1/tokens/introspect` reports it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HubToken {
    pub active: bool,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub expires_at_unix: Option<u64>,
}

/// What a cached lookup resolved to.
#[derive(Debug, Clone)]
enum Verdict {
    Session(Box<HubUser>),
    Token(Box<HubToken>),
    /// A definitive negative. The reason is not kept: every caller turns it
    /// into the same 401.
    Denied,
}

struct CacheEntry {
    verdict: Verdict,
    expires_at: Instant,
}

#[derive(Default)]
struct HubAuthCache {
    /// Keyed by `SHA-256(credential)` — never the credential itself.
    entries: HashMap<[u8; 32], CacheEntry>,
}

impl HubAuthCache {
    fn get(&mut self, key: &[u8; 32], now: Instant) -> Option<Verdict> {
        let entry = self.entries.get(key)?;
        if entry.expires_at <= now {
            self.entries.remove(key);
            return None;
        }
        Some(entry.verdict.clone())
    }

    fn insert(&mut self, key: [u8; 32], verdict: Verdict, ttl: Duration, now: Instant) {
        if ttl.is_zero() {
            return;
        }
        if self.entries.len() >= MAX_CACHE_ENTRIES {
            self.entries.retain(|_, entry| entry.expires_at > now);
        }
        if self.entries.len() >= MAX_CACHE_ENTRIES {
            // Still full of live entries: drop the one that expires soonest,
            // which is the one whose loss costs the least.
            if let Some(soonest) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.expires_at)
                .map(|(key, _)| *key)
            {
                self.entries.remove(&soonest);
            }
        }
        self.entries.insert(
            key,
            CacheEntry {
                verdict,
                expires_at: now + ttl,
            },
        );
    }
}

/// Blocking facade over the hub's identity endpoints.
///
/// Shared across connection threads; the cache is the only mutable state and
/// is behind its own mutex, never held across the HTTP call.
pub struct HubAuthClient {
    base_url: String,
    internal_auth: Option<String>,
    http: reqwest::Client,
    cache: Mutex<HubAuthCache>,
}

impl HubAuthClient {
    /// Build the client from the operator's environment.
    ///
    /// `Ok(None)` means no hub is configured, which is not an error — the
    /// online loop falls back to its development verifier and says so.
    pub fn from_env() -> Result<Option<Self>, HubAuthError> {
        let Some(base_url) = std::env::var(HUB_BASE_URL_ENV)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        else {
            return Ok(None);
        };
        Self::new(&base_url, internal_auth_from_env()).map(Some)
    }

    /// Build a client against `base_url` (scheme + authority, no path).
    pub fn new(base_url: &str, internal_auth: Option<String>) -> Result<Self, HubAuthError> {
        let parsed = reqwest::Url::parse(base_url).map_err(|_| HubAuthError::NotConfigured)?;
        if !matches!(parsed.scheme(), "http" | "https")
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(HubAuthError::NotConfigured);
        }
        let http = reqwest::Client::builder()
            .use_rustls_tls()
            // No proxy: this is a container-network call, and honouring an
            // ambient HTTP_PROXY would send session cookies to a third party.
            .no_proxy()
            // No redirects: a redirect from the identity endpoint would be a
            // way to make this daemon replay a credential somewhere else.
            .redirect(reqwest::redirect::Policy::none())
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|_| HubAuthError::Upstream)?;
        Ok(Self {
            base_url: parsed.as_str().trim_end_matches('/').to_string(),
            internal_auth,
            http,
            cache: Mutex::new(HubAuthCache::default()),
        })
    }

    /// Resolve a browser session cookie value to its hub account.
    pub fn verify_session(&self, cookie: &str) -> Result<HubUser, HubAuthError> {
        let credential = checked_credential(cookie)?;
        let key = cache_key(b"session", credential);
        match self.cached(&key) {
            Some(Verdict::Session(user)) => return Ok(*user),
            Some(Verdict::Denied) => return Err(HubAuthError::Unauthenticated),
            Some(Verdict::Token(_)) | None => {}
        }
        let outcome = self.fetch_session(credential);
        self.remember(key, &outcome, |_| SESSION_POSITIVE_TTL);
        outcome
    }

    /// Resolve an API bearer token to its hub account.
    pub fn introspect_token(&self, bearer: &str) -> Result<HubToken, HubAuthError> {
        let credential = checked_credential(bearer)?;
        let key = cache_key(b"token", credential);
        match self.cached(&key) {
            Some(Verdict::Token(token)) => return Ok(*token),
            Some(Verdict::Denied) => return Err(HubAuthError::Unauthenticated),
            Some(Verdict::Session(_)) | None => {}
        }
        let outcome = self.fetch_introspection(credential);
        self.remember(key, &outcome, token_positive_ttl);
        outcome
    }

    fn cached(&self, key: &[u8; 32]) -> Option<Verdict> {
        self.cache
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(key, Instant::now())
    }

    /// Write the cache entry a completed lookup earns, if any.
    fn remember<T: Clone + Into<Verdict>>(
        &self,
        key: [u8; 32],
        outcome: &Result<T, HubAuthError>,
        positive_ttl: impl Fn(&T) -> Duration,
    ) {
        let now = Instant::now();
        let (verdict, ttl) = match outcome {
            Ok(value) => (value.clone().into(), positive_ttl(value)),
            // Only a definitive verdict is cached. An upstream failure stays
            // uncached so a hub blip does not become a guaranteed rejection
            // window for everyone who retries inside it.
            Err(error) if error.is_cacheable() => (Verdict::Denied, NEGATIVE_TTL),
            Err(_) => return,
        };
        self.cache
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(key, verdict, ttl, now);
    }

    fn fetch_session(&self, cookie: &str) -> Result<HubUser, HubAuthError> {
        let url = format!("{}{SESSION_PATH}", self.base_url);
        let cookie_header = format!(
            "{}={cookie}",
            crate::web_canvas_server::tenant_auth::SESSION_COOKIE_NAME
        );
        let request = self
            .http
            .get(&url)
            .header(reqwest::header::COOKIE, cookie_header)
            .header(reqwest::header::ACCEPT, "application/json");
        let envelope: SessionEnvelope = self.send_json(request)?;
        if envelope.user.id.trim().is_empty() || envelope.user.username.trim().is_empty() {
            return Err(HubAuthError::MalformedResponse);
        }
        Ok(envelope.user)
    }

    fn fetch_introspection(&self, bearer: &str) -> Result<HubToken, HubAuthError> {
        let Some(secret) = self.internal_auth.as_deref() else {
            return Err(HubAuthError::MissingInternalAuth);
        };
        let url = format!("{}{INTROSPECT_PATH}", self.base_url);
        let request = self
            .http
            .post(&url)
            .header(INTERNAL_AUTH_HEADER, secret)
            .header(reqwest::header::ACCEPT, "application/json")
            .json(&serde_json::json!({ "token": bearer }));
        let token: HubToken = self.send_json(request)?;
        // An inactive token is a definitive negative, not a malformed answer:
        // it is exactly what introspection is for.
        if !token.active {
            return Err(HubAuthError::Unauthenticated);
        }
        if token.user_id.trim().is_empty() {
            return Err(HubAuthError::MalformedResponse);
        }
        Ok(token)
    }

    /// Send one identity request and decode its bounded JSON body.
    ///
    /// Status mapping is the security-relevant part: 401/403/404 are the
    /// hub's definitive "no", and everything else — including every 5xx and
    /// every transport failure — fails closed WITHOUT becoming a cacheable
    /// negative.
    fn send_json<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<T, HubAuthError> {
        let response = crate::chat_runtime::block_on_anywhere(async move {
            let response = request.send().await.map_err(|_| HubAuthError::Upstream)?;
            let status = response.status();
            if matches!(status.as_u16(), 401 | 403 | 404) {
                return Err(HubAuthError::Unauthenticated);
            }
            if !status.is_success() {
                return Err(HubAuthError::Upstream);
            }
            // Bound the body before parsing: a compromised or confused hub
            // must not be able to make this daemon buffer without limit.
            let bytes = response.bytes().await.map_err(|_| HubAuthError::Upstream)?;
            if bytes.len() > MAX_RESPONSE_BYTES {
                return Err(HubAuthError::MalformedResponse);
            }
            Ok(bytes)
        })?;
        serde_json::from_slice(&response).map_err(|_| HubAuthError::MalformedResponse)
    }
}

impl From<HubUser> for Verdict {
    fn from(user: HubUser) -> Self {
        Self::Session(Box::new(user))
    }
}

impl From<HubToken> for Verdict {
    fn from(token: HubToken) -> Self {
        Self::Token(Box::new(token))
    }
}

/// Positive TTL for a token: its own expiry can only shorten the ceiling.
fn token_positive_ttl(token: &HubToken) -> Duration {
    let Some(expires_at) = token.expires_at_unix else {
        return TOKEN_POSITIVE_TTL;
    };
    let now = crate::web_canvas_server::tenant::now_unix();
    let remaining = Duration::from_secs(expires_at.saturating_sub(now));
    TOKEN_POSITIVE_TTL.min(remaining)
}

/// Read the internal shared secret, preferring the file form.
///
/// The file wins when both are set: an orchestrator that mounts a secret has
/// made the stronger statement, and silently preferring a stale inline value
/// would be the wrong way to resolve the disagreement.
pub fn internal_auth_from_env() -> Option<String> {
    if let Some(path) = std::env::var(HUB_INTERNAL_AUTH_FILE_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return read_secret_file(&path);
    }
    std::env::var(HUB_INTERNAL_AUTH_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Read a bounded, single-line secret from `path`.
///
/// A missing or unusable file yields `None`, which makes introspection answer
/// `MissingInternalAuth` — a diagnosable refusal — rather than calling the hub
/// with an empty or partial secret.
fn read_secret_file(path: &str) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_INTERNAL_AUTH_BYTES {
        eprintln!("openpencil: {HUB_INTERNAL_AUTH_FILE_ENV} is not a readable regular file");
        return None;
    }
    let body = std::fs::read_to_string(path).ok()?;
    let secret = body.trim().to_string();
    if secret.is_empty() || !secret.bytes().all(|byte| byte.is_ascii_graphic()) {
        eprintln!("openpencil: {HUB_INTERNAL_AUTH_FILE_ENV} does not hold a usable secret");
        return None;
    }
    Some(secret)
}

/// Reject a credential this client will not forward.
///
/// The hub is entitled to assume the daemon screened obvious garbage, and a
/// credential with a control character would let a caller inject a header.
fn checked_credential(value: &str) -> Result<&str, HubAuthError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.len() > MAX_CREDENTIAL_BYTES
        || !trimmed
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b';')
    {
        return Err(HubAuthError::InvalidCredential);
    }
    Ok(trimmed)
}

/// `SHA-256(domain || 0x00 || credential)`.
///
/// Domain-separated so a value that happens to work as both a cookie and a
/// token cannot have one lookup's verdict answer the other's question.
fn cache_key(domain: &[u8], credential: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update([0u8]);
    hasher.update(credential.as_bytes());
    hasher.finalize().into()
}

#[cfg(test)]
#[path = "hub_auth_client_tests.rs"]
mod tests;
