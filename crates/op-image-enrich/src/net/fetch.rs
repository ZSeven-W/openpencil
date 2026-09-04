//! Provider fetches (Openverse + Wikimedia fallback), result ranking and
//! the data-URL embed. Carved out of the `image_search_session.rs` spine
//! to keep it under the 800-line cap; pure code motion.

use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use std::time::Duration;

use crate::net::providers::{
    fetch_openverse_token, normalize_image_mime_header, simplify_search_query,
    wikimedia_info_is_image, WebOpenverseCredentials, MAX_EMBEDDED_IMAGE_BYTES,
};
use crate::ImageAspectRatio;

pub fn fetch_first_image_url_blocking(
    query: &str,
    aspect_ratio: Option<ImageAspectRatio>,
    credentials: Option<&WebOpenverseCredentials>,
    used_urls: &Mutex<HashSet<String>>,
) -> Option<String> {
    let query = query.trim();
    if query.is_empty() {
        return None;
    }
    // Shared runtime bridge: a private per-call runtime aborts with "Cannot
    // start a runtime from within a runtime" once this search is reached from
    // a tokio worker (design-loop / MCP driven runs).
    crate::net::block_on_image_runtime(fetch_first_image_url(
        query,
        aspect_ratio,
        credentials,
        used_urls,
    ))
}

pub async fn fetch_first_image_url(
    query: &str,
    aspect_ratio: Option<ImageAspectRatio>,
    credentials: Option<&WebOpenverseCredentials>,
    used_urls: &Mutex<HashSet<String>>,
) -> Option<String> {
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .timeout(Duration::from_secs(8))
        .user_agent(concat!("openpencil-desktop/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;
    let query = simplify_search_query(query);
    if let Some(url) = fetch_openverse(&client, &query, aspect_ratio, credentials, used_urls).await
    {
        return Some(url);
    }
    let words: Vec<&str> = query.split_whitespace().filter(|w| !w.is_empty()).collect();
    if words.len() > 2 {
        let truncated = words[..2].join(" ");
        if let Some(url) =
            fetch_openverse(&client, &truncated, aspect_ratio, credentials, used_urls).await
        {
            return Some(url);
        }
        if let Some(url) = fetch_wikimedia(&client, &truncated, used_urls).await {
            return Some(url);
        }
    }
    fetch_wikimedia(&client, &query, used_urls).await
}

async fn fetch_openverse(
    client: &reqwest::Client,
    query: &str,
    aspect_ratio: Option<ImageAspectRatio>,
    credentials: Option<&WebOpenverseCredentials>,
    used_urls: &Mutex<HashSet<String>>,
) -> Option<String> {
    let url = openverse_search_url(query, aspect_ratio)?;
    let mut request = client.get(url);
    if let Some(credentials) = credentials {
        if let Some(token) = fetch_openverse_token(client, credentials).await {
            request = request.bearer_auth(token);
        }
    }
    let resp = request.send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    let results = json.get("results")?.as_array()?;
    let (result, identity) = claim_openverse_result(results, query, used_urls)?;
    let mut candidates = Vec::new();
    push_candidate_url(
        &mut candidates,
        result.get("thumbnail").and_then(serde_json::Value::as_str),
    );
    push_candidate_url(
        &mut candidates,
        result.get("url").and_then(serde_json::Value::as_str),
    );
    let outcome = first_unused_renderable_image_src(client, candidates, used_urls).await;
    settle_provider_identity(used_urls, &identity, outcome)
}

/// Titles that mark a result as noise no matter how well it ranks — the
/// classic is a literal "File not found" artwork Openverse serves for
/// weakly-matching queries (measured: it landed in a music-app card,
/// test0711-22). Junk-titled results are skipped; if EVERY result is junk
/// the slot stays empty rather than filling with a meaningless picture.
const JUNK_TITLE_MARKERS: [&str; 8] = [
    "not found",
    "404",
    "placeholder",
    "no image",
    "missing",
    "error",
    "broken",
    "deleted",
];

fn result_title(result: &serde_json::Value) -> String {
    result
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_lowercase()
}

fn meaningful_tokens(value: &str) -> HashSet<String> {
    value
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| token.chars().count() > 2)
        .map(str::to_string)
        .collect()
}

/// Pick the best of the returned results instead of blindly trusting rank 1:
/// drop junk/used entries, then rank by complete query-token overlap. Equal
/// scores preserve provider order, and an all-zero set falls back to the first
/// usable result.
pub fn select_openverse_result<'results>(
    results: &'results [serde_json::Value],
    query: &str,
    used_urls: &HashSet<String>,
) -> Option<&'results serde_json::Value> {
    let is_used = |result: &serde_json::Value| {
        openverse_result_identity(result).is_some_and(|identity| used_urls.contains(&identity))
            // Backward-compatible URL keys keep the pure selector useful to
            // callers/tests, but production claims canonical result identity.
            || ["url", "thumbnail"].iter().any(|key| {
                result
                    .get(*key)
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|url| used_urls.contains(url))
            })
    };
    let non_junk: Vec<&serde_json::Value> = results
        .iter()
        .filter(|result| {
            let title = result_title(result);
            !is_used(result)
                && !JUNK_TITLE_MARKERS
                    .iter()
                    .any(|marker| title.contains(marker))
        })
        .collect();
    let query_tokens = meaningful_tokens(query);
    let mut best = None;
    let mut best_overlap = 0usize;
    for result in non_junk {
        let title_tokens = meaningful_tokens(&result_title(result));
        let overlap = query_tokens.intersection(&title_tokens).count();
        if best.is_none() || overlap > best_overlap {
            best = Some(result);
            best_overlap = overlap;
        }
    }
    best
}

fn openverse_result_identity(result: &serde_json::Value) -> Option<String> {
    if let Some(id) = result.get("id") {
        if let Some(id) = id.as_str().filter(|id| !id.trim().is_empty()) {
            return Some(format!("openverse:{id}"));
        }
        if id.is_number() {
            return Some(format!("openverse:{id}"));
        }
    }
    ["url", "thumbnail"].iter().find_map(|field| {
        result
            .get(*field)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|url| !url.is_empty())
            .map(|url| format!("openverse-url:{url}"))
    })
}

pub fn claim_openverse_result<'results>(
    results: &'results [serde_json::Value],
    query: &str,
    used_images: &Mutex<HashSet<String>>,
) -> Option<(&'results serde_json::Value, String)> {
    let mut used = used_images.lock().unwrap();
    let result = select_openverse_result(results, query, &used)?;
    let identity = openverse_result_identity(result)?;
    used.insert(identity.clone()).then_some((result, identity))
}

pub fn openverse_search_url(
    query: &str,
    aspect_ratio: Option<ImageAspectRatio>,
) -> Option<reqwest::Url> {
    let query = simplify_search_query(query);
    let mut url = reqwest::Url::parse_with_params(
        "https://api.openverse.org/v1/images/",
        &[("q", query.as_str()), ("page_size", "10")],
    )
    .ok()?;
    if let Some(aspect_ratio) = aspect_ratio {
        url.query_pairs_mut()
            .append_pair("aspect_ratio", aspect_ratio.as_openverse_param());
    }
    Some(url)
}

async fn fetch_wikimedia(
    client: &reqwest::Client,
    query: &str,
    used_urls: &Mutex<HashSet<String>>,
) -> Option<String> {
    let url = reqwest::Url::parse_with_params(
        "https://commons.wikimedia.org/w/api.php",
        &[
            ("action", "query"),
            ("generator", "search"),
            ("gsrsearch", query),
            ("gsrnamespace", "6"),
            ("gsrlimit", "1"),
            ("prop", "imageinfo"),
            ("iiprop", "url|size|mime"),
            ("iiurlwidth", "800"),
            ("format", "json"),
            ("origin", "*"),
        ],
    )
    .ok()?;
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    let pages = json.get("query")?.get("pages")?.as_object()?;
    for page in pages.values() {
        let Some(identity) = wikimedia_page_identity(page) else {
            continue;
        };
        if !used_urls.lock().unwrap().insert(identity.clone()) {
            continue;
        }
        // An empty candidate list deliberately settles as Unavailable below,
        // releasing the reservation for pages with missing/empty imageinfo.
        let candidates = wikimedia_image_candidates(page);
        let outcome = first_unused_renderable_image_src(client, candidates, used_urls).await;
        if let Some(src) = settle_provider_identity(used_urls, &identity, outcome) {
            return Some(src);
        }
    }
    None
}

pub fn wikimedia_page_identity(page: &serde_json::Value) -> Option<String> {
    if let Some(page_id) = page.get("pageid") {
        if page_id.is_number() || page_id.is_string() {
            return Some(format!("wikimedia:{page_id}"));
        }
    }
    page.get("title")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(|title| format!("wikimedia-title:{title}"))
}

pub fn wikimedia_image_candidates(page: &serde_json::Value) -> Vec<String> {
    let mut candidates = Vec::new();
    if let Some(info) = page
        .get("imageinfo")
        .and_then(serde_json::Value::as_array)
        .and_then(|items| items.first())
    {
        if !wikimedia_info_is_image(page, info) {
            return candidates;
        }
        push_candidate_url(
            &mut candidates,
            info.get("thumburl").and_then(serde_json::Value::as_str),
        );
        push_candidate_url(
            &mut candidates,
            info.get("url").and_then(serde_json::Value::as_str),
        );
    }
    candidates
}

fn push_candidate_url(candidates: &mut Vec<String>, url: Option<&str>) {
    let Some(url) = url.map(str::trim).filter(|url| !url.is_empty()) else {
        return;
    };
    if !candidates.iter().any(|candidate| candidate == url) {
        candidates.push(url.to_string());
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ImageCandidateClaim {
    /// A renderable image won the session-wide content claim.
    Claimed(String),
    /// A renderable download matched content another provider result already
    /// owns. The provider identity stays used because its artwork is known to
    /// be a duplicate even though this request cannot return it.
    Duplicate,
    /// No candidate URL produced a renderable image. The provider reservation
    /// must be released so a later request can retry a transient failure.
    Unavailable,
}

pub fn settle_provider_identity(
    used_urls: &Mutex<HashSet<String>>,
    identity: &str,
    outcome: ImageCandidateClaim,
) -> Option<String> {
    match outcome {
        ImageCandidateClaim::Claimed(src) => Some(src),
        ImageCandidateClaim::Duplicate => None,
        ImageCandidateClaim::Unavailable => {
            used_urls.lock().unwrap().remove(identity);
            None
        }
    }
}

pub async fn first_unused_renderable_image_src(
    client: &reqwest::Client,
    candidates: Vec<String>,
    used_urls: &Mutex<HashSet<String>>,
) -> ImageCandidateClaim {
    let mut found_duplicate = false;
    for candidate in candidates {
        if let Some(src) = fetch_image_data_url(client, &candidate).await {
            // Claim the embedded result under one lock. Different queries run
            // concurrently and may resolve to the same underlying image; a
            // snapshot-then-insert check lets both win. The atomic claim lets
            // the loser continue to another candidate/fallback instead.
            if claim_unused_image_src(used_urls, &src) {
                return ImageCandidateClaim::Claimed(src);
            }
            found_duplicate = true;
        }
    }
    if found_duplicate {
        ImageCandidateClaim::Duplicate
    } else {
        ImageCandidateClaim::Unavailable
    }
}

pub fn claim_unused_image_src(used_urls: &Mutex<HashSet<String>>, src: &str) -> bool {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut hasher);
    used_urls
        .lock()
        .unwrap()
        .insert(format!("content:{:016x}", hasher.finish()))
}

pub async fn fetch_image_data_url(client: &reqwest::Client, url: &str) -> Option<String> {
    let (mime, bytes) =
        crate::net::providers::fetch_image_bytes(client, url, MAX_EMBEDDED_IMAGE_BYTES).await?;
    renderable_image_data_url(&mime, &bytes)
}

/// Data URL for a web-fetched image, restricted to payloads the exact
/// renderer can decode. PNG / JPEG embed as-is (down-scaled when
/// oversized); every other container (WebP thumbnails, GIF, …) must
/// transcode through [`crate::net::downscale::reencode_for_renderer`]
/// or the candidate is rejected with `None` so the caller falls through
/// to the next URL. Unlike [`image_bytes_to_data_url`] (the permissive
/// user-import path), this never embeds bytes that draw as a blank
/// placeholder in the committed document.
pub fn renderable_image_data_url(mime: &str, bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    let mime = normalize_image_mime_header(mime)?;
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine as _;
    if mime == "image/png" || mime == "image/jpeg" {
        if let Some((scaled_mime, scaled)) = crate::net::downscale::maybe_downscale(bytes) {
            return Some(format!("data:{scaled_mime};base64,{}", B64.encode(&scaled)));
        }
        return Some(format!("data:{mime};base64,{}", B64.encode(bytes)));
    }
    let (out_mime, out) = crate::net::downscale::reencode_for_renderer(bytes)?;
    Some(format!("data:{out_mime};base64,{}", B64.encode(&out)))
}

pub fn image_bytes_to_data_url(mime: &str, bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    let mime = normalize_image_mime_header(mime)?;
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine as _;
    // Shrink an oversized fetched image before it enters the document —
    // same rationale as the file-pick path (see `image_downscale`).
    if let Some((scaled_mime, scaled)) = crate::net::downscale::maybe_downscale(bytes) {
        return Some(format!("data:{scaled_mime};base64,{}", B64.encode(&scaled)));
    }
    Some(format!("data:{mime};base64,{}", B64.encode(bytes)))
}
