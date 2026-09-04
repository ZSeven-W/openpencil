//! Browser image-search route for the web daemon.
//!
//! `POST /api/ai/image/search` mirrors the desktop image panel's Search
//! popover backend (`op-host-desktop/src/image_panel_host.rs`: Openverse →
//! two-keyword retry → Wikimedia, thumbnails embedded as `data:` URLs) so
//! the wasm shell can drain its `search_epoch` through the daemon instead
//! of leaving the popover loading forever. Openverse credentials come from
//! the request body (browser-held) or fall back to the daemon's persisted
//! agent settings. Openverse / Wikimedia are product-constant public hosts
//! — the same operator-trust tier as the desktop path — so they dial with
//! a plain client; nothing in this route dials a browser-supplied URL.
//!
//! Unlike the desktop, fetched thumbnails are NOT re-encoded/down-scaled
//! here: `image_downscale` needs skia and this crate must stay GL-free for
//! `op-host-web-server`. The 4 MiB per-image cap still bounds what can be
//! embedded.

mod catalog;
mod download;
mod relevance;

pub(crate) use catalog::wikimedia_info_is_image;
pub use catalog::{
    fetch_openverse_token, parse_openverse_results, parse_wikimedia_results, RawHit,
};
pub use download::{
    fetch_image_bytes, fetch_image_data_url, normalize_image_mime_header, read_capped,
    sniff_image_mime,
};
pub use relevance::simplify_search_query;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use catalog::{
    fetch_openverse_list, fetch_relevant_wikimedia_list, materialize_first_thumb,
    materialize_thumbs,
};
use relevance::{retain_relevant_hits, two_keyword_retry};

#[cfg(test)]
use catalog::fetch_relevant_wikimedia_list_with;
#[cfg(test)]
use relevance::{
    core_query_words, metadata_is_scene_heavy, query_requests_scene, SCENE_HEAVY_RESULT_WORDS,
};

/// Cap on concurrently running image jobs (search + generate combined).
/// Each job blocks one connection thread for up to minutes of provider
/// network; without a ceiling a page could exhaust the daemon's threads.
const MAX_IN_FLIGHT_IMAGE_JOBS: usize = 4;

static IN_FLIGHT_IMAGE_JOBS: AtomicUsize = AtomicUsize::new(0);

/// RAII slot for one running image job. `acquire` fails once
/// [`MAX_IN_FLIGHT_IMAGE_JOBS`] jobs are running (route answers 429).
pub struct ImageJobSlot(());

impl ImageJobSlot {
    pub fn acquire() -> Option<Self> {
        IN_FLIGHT_IMAGE_JOBS
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < MAX_IN_FLIGHT_IMAGE_JOBS).then_some(n + 1)
            })
            .ok()
            .map(|_| Self(()))
    }
}

impl Drop for ImageJobSlot {
    fn drop(&mut self) {
        IN_FLIGHT_IMAGE_JOBS.fetch_sub(1, Ordering::AcqRel);
    }
}

/// TS popover requests `count: 5` (desktop parity).
const SEARCH_RESULT_COUNT: usize = 5;
/// Fetch a wider catalogue window before relevance ranking. The public route
/// still materializes at most [`SEARCH_RESULT_COUNT`] thumbnails.
const SEARCH_CANDIDATE_COUNT: usize = 20;
pub const MAX_EMBEDDED_IMAGE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, PartialEq, Eq)]
pub struct WebOpenverseCredentials {
    pub client_id: String,
    pub client_secret: String,
}

impl WebOpenverseCredentials {
    /// `None` unless both parts are non-empty after trimming.
    pub fn from_parts(client_id: &str, client_secret: &str) -> Option<Self> {
        let client_id = client_id.trim();
        let client_secret = client_secret.trim();
        if client_id.is_empty() || client_secret.is_empty() {
            None
        } else {
            Some(Self {
                client_id: client_id.to_string(),
                client_secret: client_secret.to_string(),
            })
        }
    }
}

/// One search hit ready for the JSON reply / the desktop popover.
pub struct WebImageSearchHit {
    pub id: String,
    pub thumb_data_url: String,
    pub attribution: String,
}

pub struct WebImageSearchOutcome {
    pub results: Vec<WebImageSearchHit>,
    /// `"openverse"` / `"wikimedia"`, `None` when nothing landed.
    pub source: Option<&'static str>,
}

/// Why a `POST /api/ai/image/search` body was refused. Both variants answer
/// HTTP 400; the enum exists so the route reports WHICH client mistake was
/// made instead of matching on prose, and `Display` reproduces the exact
/// sentence the JSON reply already carried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchRequestError {
    /// The body is not JSON, or is JSON but not an object.
    InvalidBody,
    /// The body is a valid object but carries no non-blank `query`.
    MissingQuery,
}

impl std::fmt::Display for SearchRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchRequestError::InvalidBody => f.write_str("invalid request body"),
            SearchRequestError::MissingQuery => f.write_str("missing query"),
        }
    }
}

impl std::error::Error for SearchRequestError {}

/// Parse the request body and snapshot the daemon-side credential fallback.
/// Returns `(query, credentials)` or the reason for the 400 reply.
pub fn parse_search_request(
    body: &str,
    state: &op_editor_core::EditorState,
) -> Result<(String, Option<WebOpenverseCredentials>), SearchRequestError> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|_| SearchRequestError::InvalidBody)?;
    let obj = value.as_object().ok_or(SearchRequestError::InvalidBody)?;
    let query = obj
        .get("query")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|q| !q.is_empty())
        .ok_or(SearchRequestError::MissingQuery)?;
    // Browser-held credential wins; the daemon's persisted settings are the
    // fallback (both are optional — anonymous Openverse works, rate-limited).
    let request_credentials = obj
        .get("openverse")
        .and_then(serde_json::Value::as_object)
        .and_then(|cred| {
            WebOpenverseCredentials::from_parts(
                cred.get("client_id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
                cred.get("client_secret")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
            )
        });
    let credentials = request_credentials.or_else(|| {
        let settings = &state.editor_ui.agent_settings;
        WebOpenverseCredentials::from_parts(
            &settings.openverse_client_id,
            &settings.openverse_client_secret,
        )
    });
    Ok((query.to_string(), credentials))
}

/// JSON reply body for a finished search.
pub fn search_outcome_to_json(outcome: &WebImageSearchOutcome) -> String {
    let results: Vec<serde_json::Value> = outcome
        .results
        .iter()
        .map(|hit| {
            serde_json::json!({
                "id": hit.id,
                "thumb_data_url": hit.thumb_data_url,
                "attribution": hit.attribution,
            })
        })
        .collect();
    serde_json::json!({
        "ok": true,
        "results": results,
        "source": outcome.source,
    })
    .to_string()
}

/// Run the full search ladder on the calling thread (the connection's own
/// thread — the caller must NOT hold the state lock).
pub fn run_search_blocking(
    query: &str,
    credentials: Option<&WebOpenverseCredentials>,
) -> WebImageSearchOutcome {
    // A private runtime here would panic the moment this sync helper is
    // reached from a tokio worker; `block_on_anywhere` runs the ladder on the
    // shared (enable_all) runtime instead — same IO/timer drivers, no
    // runtime-in-runtime hazard.
    crate::net::block_on_image_runtime(run_search(query, credentials))
}

/// Run the provider ladder for a single usable thumbnail, bounding the whole
/// async ladder (catalog requests, retries, and thumbnail download together)
/// by `remaining`. This is the MCP enrichment path: unlike the Web UI search,
/// it needs only one image and must return before its outer transport timeout.
pub fn run_first_search_blocking_with_timeout(
    query: &str,
    credentials: Option<&WebOpenverseCredentials>,
    remaining: Duration,
) -> WebImageSearchOutcome {
    run_with_timeout(remaining, run_first_search(query, credentials))
        .unwrap_or_else(empty_search_outcome)
}

fn run_with_timeout<F>(remaining: Duration, future: F) -> Option<F::Output>
where
    F: std::future::Future,
{
    if remaining.is_zero() {
        return None;
    }
    crate::net::block_on_image_runtime(
        async move { tokio::time::timeout(remaining, future).await.ok() },
    )
}

fn empty_search_outcome() -> WebImageSearchOutcome {
    WebImageSearchOutcome {
        results: Vec::new(),
        source: None,
    }
}

async fn run_search(
    query: &str,
    credentials: Option<&WebOpenverseCredentials>,
) -> WebImageSearchOutcome {
    let Ok(client) = reqwest::Client::builder()
        .use_rustls_tls()
        .timeout(Duration::from_secs(8))
        .user_agent(concat!("openpencil-web-daemon/", env!("CARGO_PKG_VERSION")))
        .build()
    else {
        return WebImageSearchOutcome {
            results: Vec::new(),
            source: None,
        };
    };
    run_search_with_fetcher(&client, query, credentials, |url: String| {
        let client = client.clone();
        async move { fetch_image_data_url(&client, &url).await }
    })
    .await
}

/// Single-result variant of [`run_search`]. Provider list lookup keeps the
/// same Openverse → retry → Wikimedia order, but thumbnail materialization
/// stops after the first successful download instead of fetching the whole
/// five-result Web UI page.
async fn run_first_search(
    query: &str,
    credentials: Option<&WebOpenverseCredentials>,
) -> WebImageSearchOutcome {
    let Ok(client) = reqwest::Client::builder()
        .use_rustls_tls()
        .timeout(Duration::from_secs(8))
        .user_agent(concat!("openpencil-web-daemon/", env!("CARGO_PKG_VERSION")))
        .build()
    else {
        return empty_search_outcome();
    };
    run_first_search_with_fetcher(&client, query, credentials, |url: String| {
        let client = client.clone();
        async move { fetch_image_data_url(&client, &url).await }
    })
    .await
}

/// The full search ladder over a caller-supplied client + thumbnail
/// materializer. Shared by this daemon route (plain embed) and the desktop
/// popover (its own user-agent + skia down-scale pass on each thumbnail).
///
/// `fetch_data_url` downloads one thumbnail URL into a `data:` URL; hits
/// whose thumbnails fail to download are dropped.
pub async fn run_search_with_fetcher<F, Fut>(
    client: &reqwest::Client,
    query: &str,
    credentials: Option<&WebOpenverseCredentials>,
    fetch_data_url: F,
) -> WebImageSearchOutcome
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Option<String>>,
{
    // Simplify verbose prompts into keywords (TS simplifySearchQuery).
    let query = simplify_search_query(query);

    // Openverse first; either a zero-result answer or an answer fully removed
    // by the relevance/photo fence retries once with a short concrete subject
    // phrase before falling through to Wikimedia.
    let hits = fetch_relevant_openverse_list(client, &query, credentials).await;
    if let Some(urls) = hits.filter(|h| !h.is_empty()) {
        let results = materialize_thumbs(urls, &fetch_data_url).await;
        if !results.is_empty() {
            return WebImageSearchOutcome {
                results,
                source: Some("openverse"),
            };
        }
    }
    let wiki = fetch_relevant_wikimedia_list(client, &query).await;
    let results = materialize_thumbs(wiki, &fetch_data_url).await;
    let source = (!results.is_empty()).then_some("wikimedia");
    WebImageSearchOutcome { results, source }
}

async fn run_first_search_with_fetcher<F, Fut>(
    client: &reqwest::Client,
    query: &str,
    credentials: Option<&WebOpenverseCredentials>,
    fetch_data_url: F,
) -> WebImageSearchOutcome
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Option<String>>,
{
    let query = simplify_search_query(query);

    let hits = fetch_relevant_openverse_list(client, &query, credentials).await;
    if let Some(urls) = hits.filter(|h| !h.is_empty()) {
        if let Some(result) = materialize_first_thumb(urls, &fetch_data_url).await {
            return WebImageSearchOutcome {
                results: vec![result],
                source: Some("openverse"),
            };
        }
    }

    let wiki = fetch_relevant_wikimedia_list(client, &query).await;
    let Some(result) = materialize_first_thumb(wiki, &fetch_data_url).await else {
        return empty_search_outcome();
    };
    WebImageSearchOutcome {
        results: vec![result],
        source: Some("wikimedia"),
    }
}

/// Fetch and relevance-filter the Openverse catalogue, retrying at most once
/// with the concrete subject phrase. `None` remains a request-level failure:
/// it falls through to Wikimedia without turning a network error into another
/// Openverse request. `Some([])` and non-empty-but-fully-filtered replies share
/// the same single retry, so the two conditions can never trigger duplicate
/// catalogue requests.
async fn fetch_relevant_openverse_list(
    client: &reqwest::Client,
    query: &str,
    credentials: Option<&WebOpenverseCredentials>,
) -> Option<Vec<RawHit>> {
    fetch_relevant_openverse_list_with(query, |candidate| {
        let client = client.clone();
        let credentials = credentials.cloned();
        async move { fetch_openverse_list(&client, &candidate, credentials.as_ref()).await }
    })
    .await
}

async fn fetch_relevant_openverse_list_with<F, Fut>(
    query: &str,
    mut fetch_list: F,
) -> Option<Vec<RawHit>>
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = Option<Vec<RawHit>>>,
{
    let hits = fetch_list(query.to_string()).await?;
    let relevant = retain_relevant_hits(hits, query);
    if !relevant.is_empty() {
        return Some(relevant);
    }

    let Some(retry_query) = two_keyword_retry(query) else {
        return Some(relevant);
    };
    let retry = fetch_list(retry_query).await?;
    // The provider receives a shorter concrete query, but relevance still
    // follows the authored query so photo/studio/isolated intent is not lost.
    Some(retain_relevant_hits(retry, query))
}

#[cfg(test)]
#[path = "web_image_search_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "web_image_search_relevance_tests.rs"]
mod relevance_tests;

#[cfg(test)]
#[path = "web_image_search_retry_tests.rs"]
mod retry_tests;
