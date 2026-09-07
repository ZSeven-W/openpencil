//! Provider catalogue requests and parsing (Openverse list + token,
//! Wikimedia list) plus thumbnail materialization. Carved out of the
//! `providers.rs` spine to keep it under the 800-line cap; pure code motion.

use super::{
    pacing::{
        cache_token, cached_token, drop_cached_token, record_response, retry_after_seconds,
        retry_delay, token_lock, wait_for_request,
    },
    read_capped, retain_relevant_hits, two_keyword_retry, WebImageSearchHit,
    WebOpenverseCredentials, MAX_EMBEDDED_IMAGE_BYTES, SEARCH_CANDIDATE_COUNT, SEARCH_RESULT_COUNT,
};

/// Catalogue/token calls get a longer budget than the shared 8 s client
/// timeout: Openverse answers a cold query in ~9 s on a slow day, and a
/// timed-out list request is indistinguishable from "no results" downstream.
const CATALOGUE_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

/// `reqwest::Error`'s `Display` stops at "error sending request"; the cause
/// (timeout, connection reset, TLS) sits in the source chain.
pub(crate) fn error_chain(err: &dyn std::error::Error) -> String {
    let mut parts = vec![err.to_string()];
    let mut source = err.source();
    while let Some(inner) = source {
        parts.push(inner.to_string());
        source = inner.source();
    }
    parts.join(": ")
}

struct TokenResult {
    token: String,
    from_cache: bool,
}

#[derive(Clone)]
pub struct RawHit {
    pub(crate) id: String,
    pub(crate) thumb_url: String,
    pub(crate) attribution: String,
    pub(crate) title: String,
    pub(crate) relevance_metadata: String,
}

/// `None` = request-level failure (429 / network), `Some([])` = the
/// catalogue answered with zero hits (the ladder distinguishes the two).
pub(crate) async fn fetch_openverse_list(
    client: &reqwest::Client,
    query: &str,
    credentials: Option<&WebOpenverseCredentials>,
) -> Option<Vec<RawHit>> {
    fetch_openverse_list_with_aspect(client, query, None, credentials).await
}

pub(crate) async fn fetch_openverse_list_with_aspect(
    client: &reqwest::Client,
    query: &str,
    aspect_ratio: Option<crate::ImageAspectRatio>,
    credentials: Option<&WebOpenverseCredentials>,
) -> Option<Vec<RawHit>> {
    let mut url = reqwest::Url::parse_with_params(
        "https://api.openverse.org/v1/images/",
        &[
            ("q", query),
            ("page_size", &SEARCH_CANDIDATE_COUNT.to_string()),
        ],
    )
    .ok()?;
    if let Some(aspect_ratio) = aspect_ratio {
        url.query_pairs_mut()
            .append_pair("aspect_ratio", aspect_ratio.as_openverse_param());
    }
    let json = fetch_openverse_json(client, url, query, credentials).await?;
    Some(parse_openverse_results(&json))
}

pub(crate) async fn fetch_openverse_json(
    client: &reqwest::Client,
    url: reqwest::Url,
    query: &str,
    credentials: Option<&WebOpenverseCredentials>,
) -> Option<serde_json::Value> {
    let mut token = None;
    let mut token_was_cached = false;
    if let Some(credentials) = credentials {
        if let Some(result) = fetch_openverse_token_with_source(client, credentials, false).await {
            token = Some(result.token);
            token_was_cached = result.from_cache;
        }
    }

    let mut retried_429 = false;
    let mut refreshed_401 = false;
    loop {
        if !wait_for_request().await {
            return None;
        }
        let mut request = client.get(url.clone()).timeout(CATALOGUE_REQUEST_TIMEOUT);
        if let Some(token) = token.as_deref() {
            request = request.bearer_auth(token);
        }
        let resp = match request.send().await {
            Ok(resp) => resp,
            Err(err) => {
                // A transport failure (today: Openverse cold queries taking
                // longer than the shared 8 s client timeout) used to vanish
                // as a silent `k=0`; name it so the log tells a slow API
                // apart from an empty catalogue.
                eprintln!(
                    "[ENRICH] openverse: request failed for \"{query}\": {}",
                    error_chain(&err)
                );
                return None;
            }
        };
        let status = resp.status();
        let retry_after = if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            Some(retry_after_seconds(resp.headers()))
        } else {
            None
        };
        record_response(resp.headers());
        if !status.is_success() {
            if let Some(retry_after) = retry_after {
                eprintln!(
                    "[ENRICH] openverse: HTTP {status} for \"{query}\" (retry-after={retry_after}s)"
                );
            } else {
                eprintln!("[ENRICH] openverse: HTTP {status} for \"{query}\"");
            }
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS && !retried_429 {
                retried_429 = true;
                retry_delay(resp.headers()).await;
                continue;
            }
            if status == reqwest::StatusCode::UNAUTHORIZED && token_was_cached && !refreshed_401 {
                refreshed_401 = true;
                if let Some(credentials) = credentials {
                    drop_cached_token(&credentials.client_id);
                    if let Some(result) =
                        fetch_openverse_token_with_source(client, credentials, true).await
                    {
                        token = Some(result.token);
                        token_was_cached = result.from_cache;
                    } else {
                        token = None;
                        token_was_cached = false;
                    }
                    continue;
                }
            }
            return None;
        }
        return read_json_capped(resp).await;
    }
}

/// Catalogue-list bodies are small JSON; 4 MiB bounds a misbehaving reply.
async fn read_json_capped(resp: reqwest::Response) -> Option<serde_json::Value> {
    let bytes = read_capped(resp, MAX_EMBEDDED_IMAGE_BYTES).await?;
    serde_json::from_slice(&bytes).ok()
}

pub fn parse_openverse_results(json: &serde_json::Value) -> Vec<RawHit> {
    let Some(results) = json.get("results").and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };
    results
        .iter()
        .filter_map(|r| {
            let thumb = r
                .get("thumbnail")
                .and_then(serde_json::Value::as_str)
                .or_else(|| r.get("url").and_then(serde_json::Value::as_str))?;
            let license = format!(
                "{} {}",
                r.get("license")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
                r.get("license_version")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
            );
            Some(RawHit {
                id: r
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                thumb_url: thumb.to_string(),
                attribution: r
                    .get("attribution")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| license.trim().to_string()),
                title: r
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                relevance_metadata: openverse_relevance_metadata(r),
            })
        })
        .take(SEARCH_CANDIDATE_COUNT)
        .collect()
}

fn openverse_relevance_metadata(result: &serde_json::Value) -> String {
    let mut parts = Vec::new();
    if let Some(title) = result.get("title").and_then(serde_json::Value::as_str) {
        parts.push(title.to_string());
    }
    if let Some(tags) = result.get("tags") {
        match tags {
            serde_json::Value::Array(items) => {
                for item in items {
                    if let Some(tag) = item
                        .as_str()
                        .or_else(|| item.get("name").and_then(serde_json::Value::as_str))
                    {
                        parts.push(tag.to_string());
                    }
                }
            }
            serde_json::Value::String(tags) => parts.push(tags.clone()),
            _ => {}
        }
    }
    parts.join(" ")
}

async fn fetch_wikimedia_list(client: &reqwest::Client, query: &str) -> Vec<RawHit> {
    let Ok(url) = reqwest::Url::parse_with_params(
        "https://commons.wikimedia.org/w/api.php",
        &[
            ("action", "query"),
            ("generator", "search"),
            ("gsrsearch", query),
            ("gsrnamespace", "6"),
            ("gsrlimit", &SEARCH_CANDIDATE_COUNT.to_string()),
            ("prop", "imageinfo"),
            ("iiprop", "url|size|mime|extmetadata"),
            ("iiurlwidth", "800"),
            ("format", "json"),
            ("origin", "*"),
        ],
    ) else {
        return Vec::new();
    };
    let Ok(resp) = client.get(url).send().await else {
        return Vec::new();
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    let Some(json) = read_json_capped(resp).await else {
        return Vec::new();
    };
    parse_wikimedia_results(&json)
}

pub(super) async fn fetch_relevant_wikimedia_list(
    client: &reqwest::Client,
    query: &str,
) -> Vec<RawHit> {
    fetch_relevant_wikimedia_list_with(query, |candidate| {
        let client = client.clone();
        async move { fetch_wikimedia_list(&client, &candidate).await }
    })
    .await
}

pub(super) async fn fetch_relevant_wikimedia_list_with<F, Fut>(
    query: &str,
    mut fetch_list: F,
) -> Vec<RawHit>
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = Vec<RawHit>>,
{
    let hits = fetch_list(query.to_string()).await;
    let relevant = retain_relevant_hits(hits, query);
    if !relevant.is_empty() {
        return relevant;
    }

    let Some(retry_query) = two_keyword_retry(query) else {
        return relevant;
    };
    let retry = fetch_list(retry_query).await;
    // Keep the original photo/studio/isolated contract for concrete retries.
    retain_relevant_hits(retry, query)
}

pub fn parse_wikimedia_results(json: &serde_json::Value) -> Vec<RawHit> {
    let Some(pages) = json
        .get("query")
        .and_then(|q| q.get("pages"))
        .and_then(serde_json::Value::as_object)
    else {
        return Vec::new();
    };
    pages
        .values()
        .filter_map(|page| {
            let info = page.get("imageinfo")?.as_array()?.first()?;
            if !wikimedia_info_is_image(page, info) {
                return None;
            }
            let thumb = info
                .get("thumburl")
                .and_then(serde_json::Value::as_str)
                .or_else(|| info.get("url").and_then(serde_json::Value::as_str))?;
            Some(RawHit {
                id: page
                    .get("pageid")
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                thumb_url: thumb.to_string(),
                attribution: info
                    .get("extmetadata")
                    .and_then(|m| m.get("LicenseShortName"))
                    .and_then(|l| l.get("value"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                title: page
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                relevance_metadata: page
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            })
        })
        .take(SEARCH_CANDIDATE_COUNT)
        .collect()
}

/// Wikimedia can return page-one JPEG thumbnails for PDFs, audio, and video
/// files. Those thumbnails are renderable bytes but are not image-search
/// results, so accepting them turns an archival cover page into a product
/// photo. Trust the source MIME when it is present and retain a title-extension
/// fence for older/test payloads that omit `mime`.
pub(crate) fn wikimedia_info_is_image(page: &serde_json::Value, info: &serde_json::Value) -> bool {
    if let Some(mime) = info.get("mime").and_then(serde_json::Value::as_str) {
        return mime.trim().to_ascii_lowercase().starts_with("image/");
    }

    let title = page
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    ![".pdf", ".ogg", ".oga", ".ogv", ".webm", ".mp3", ".mp4"]
        .iter()
        .any(|extension| title.ends_with(extension))
}

/// Download each hit's thumbnail into a `data:` URL through the caller's
/// materializer. Hits whose thumbnails fail to download are dropped.
pub(super) async fn materialize_thumbs<F, Fut>(
    hits: Vec<RawHit>,
    fetch_data_url: &F,
) -> Vec<WebImageSearchHit>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Option<String>>,
{
    let mut out = Vec::with_capacity(hits.len().min(SEARCH_RESULT_COUNT));
    for hit in hits {
        if let Some(data_url) = fetch_data_url(hit.thumb_url.clone()).await {
            out.push(WebImageSearchHit {
                id: hit.id,
                thumb_data_url: data_url,
                attribution: hit.attribution,
            });
            if out.len() == SEARCH_RESULT_COUNT {
                break;
            }
        }
    }
    out
}

/// Materialize only the first usable thumbnail. Failed downloads advance to
/// the next provider hit, while the first success ends the loop immediately.
pub(super) async fn materialize_first_thumb<F, Fut>(
    hits: Vec<RawHit>,
    fetch_data_url: &F,
) -> Option<WebImageSearchHit>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Option<String>>,
{
    for hit in hits {
        if let Some(data_url) = fetch_data_url(hit.thumb_url).await {
            return Some(WebImageSearchHit {
                id: hit.id,
                thumb_data_url: data_url,
                attribution: hit.attribution,
            });
        }
    }
    None
}

pub async fn fetch_openverse_token(
    client: &reqwest::Client,
    credentials: &WebOpenverseCredentials,
) -> Option<String> {
    fetch_openverse_token_with_source(client, credentials, false)
        .await
        .map(|result| result.token)
}

async fn fetch_openverse_token_with_source(
    client: &reqwest::Client,
    credentials: &WebOpenverseCredentials,
    force_refresh: bool,
) -> Option<TokenResult> {
    let lock = token_lock(&credentials.client_id);
    let _guard = lock.lock().await;
    if !force_refresh {
        if let Some(token) = cached_token(&credentials.client_id) {
            return Some(TokenResult {
                token,
                from_cache: true,
            });
        }
    } else {
        drop_cached_token(&credentials.client_id);
    }

    let mut retried_429 = false;
    loop {
        if !wait_for_request().await {
            return None;
        }
        let resp = client
            .post("https://api.openverse.org/v1/auth_tokens/token/")
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", credentials.client_id.as_str()),
                ("client_secret", credentials.client_secret.as_str()),
            ])
            .timeout(CATALOGUE_REQUEST_TIMEOUT)
            .send()
            .await;
        let resp = match resp {
            Ok(resp) => resp,
            Err(err) => {
                eprintln!(
                    "[ENRICH] openverse-token: request failed: {}",
                    error_chain(&err)
                );
                return None;
            }
        };
        let status = resp.status();
        let retry_after = if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            Some(retry_after_seconds(resp.headers()))
        } else {
            None
        };
        record_response(resp.headers());
        if !status.is_success() {
            if let Some(retry_after) = retry_after {
                eprintln!("[ENRICH] openverse-token: HTTP {status} (retry-after={retry_after}s)");
            } else {
                eprintln!("[ENRICH] openverse-token: HTTP {status}");
            }
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS && !retried_429 {
                retried_429 = true;
                retry_delay(resp.headers()).await;
                continue;
            }
            return None;
        }
        let json = read_json_capped(resp).await?;
        let token = json
            .get("access_token")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(str::to_string)?;
        if let Some(expires_in) = json.get("expires_in").and_then(serde_json::Value::as_u64) {
            cache_token(&credentials.client_id, token.clone(), expires_in);
        }
        return Some(TokenResult {
            token,
            from_cache: false,
        });
    }
}
