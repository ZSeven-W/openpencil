//! Provider fetches (Openverse + Wikimedia fallback), result ranking and
//! the data-URL embed. Carved out of the `image_search_session.rs` spine
//! to keep it under the 800-line cap; pure code motion.
//!
//! The judged ladder (`fetch_first_image_url_with_judge`) runs: primary
//! intent-refined Openverse query → up to two retrieval fallbacks (including
//! the original query when refined) through
//! the same judged Openverse path → the judged Wikimedia fallback ladder.
//! The no-judge ladder lives in `resolve.rs`; both ladders fence their
//! provider lists through the same subject/photo/brand relevance contract.

use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::net::providers::{
    fetch_image_and_judge_thumbnail_for_view, normalize_image_mime_header, retrieval_plan,
    simplify_search_query, wikimedia_info_is_image, RawHit, WebOpenverseCredentials,
    MAX_EMBEDDED_IMAGE_BYTES,
};
use crate::net::resolve::{fetch_first_image_resolved, fetch_relevant_openverse_list_with_aspect};
pub use crate::net::resolve::{fetch_first_image_resolved_blocking, ResolvedImage};
use crate::net::search_trace::trace_line;
use crate::net::{format_judge_log, ImageRelevanceJudge};
use crate::ImageAspectRatio;

#[path = "fetch_judged.rs"]
mod judged;
use judged::{before_deadline, claim_ranked_image, JudgeSlot, JudgedImage, WeakFallback};

#[path = "fetch_aspect.rs"]
mod aspect;
use aspect::{
    earlier_deadline, hit_identity, judging_deadline, search_original_aspects, select_and_record,
    unseen_hits,
};

pub fn fetch_first_image_url_blocking(
    query: &str,
    aspect_ratio: Option<ImageAspectRatio>,
    credentials: Option<&WebOpenverseCredentials>,
    used_urls: &Mutex<HashSet<String>>,
) -> Option<String> {
    fetch_first_image_resolved_blocking(query, "", aspect_ratio, credentials, used_urls)
        .map(|resolved| resolved.src)
}

pub async fn fetch_first_image_url(
    query: &str,
    aspect_ratio: Option<ImageAspectRatio>,
    credentials: Option<&WebOpenverseCredentials>,
    used_urls: &Mutex<HashSet<String>>,
) -> Option<String> {
    fetch_first_image_resolved(query, "", aspect_ratio, credentials, used_urls)
        .await
        .map(|resolved| resolved.src)
}

#[path = "fetch_entry.rs"]
mod entry;
pub use entry::*;

#[cfg(test)]
#[path = "fetch_view_tests.rs"]
mod view_tests;

#[allow(clippy::too_many_arguments)]
async fn fetch_first_image_url_with_judge(
    query: &str,
    aspect_ratio: Option<ImageAspectRatio>,
    credentials: Option<&WebOpenverseCredentials>,
    used_urls: &Mutex<HashSet<String>>,
    judge: &dyn ImageRelevanceJudge,
    intent: &str,
    deadline: Option<Instant>,
    view: Option<crate::ImageJudgeView>,
) -> Option<(ResolvedImage, Option<Instant>)> {
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .timeout(Duration::from_secs(8))
        .user_agent(concat!("openpencil-desktop/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;
    let slot = JudgeSlot {
        query,
        intent,
        judge,
        view,
        deadline: judging_deadline(judge, deadline),
    };
    let (query, rewrites) = retrieval_plan(&simplify_search_query(query), intent);
    let mut fallback = WeakFallback::new(used_urls, deadline);
    let seen = Mutex::new(HashSet::new());
    trace_line(format_args!(
        "ladder=judged query=\"{query}\" aspect={aspect_ratio:?} judge_view={view:?}"
    ));
    let original = search_original_aspects(
        &query,
        aspect_ratio,
        judge,
        deadline,
        &mut fallback,
        |aspect, deadline, allow_weak| {
            let client = &client;
            let query = &query;
            let seen = &seen;
            let slot = JudgeSlot {
                deadline: judging_deadline(judge, deadline),
                ..slot
            };
            async move {
                let hits = fetch_relevant_openverse_list_with_aspect(
                    client,
                    query,
                    intent,
                    aspect,
                    credentials,
                )
                .await?
                .hits;
                Some(
                    fetch_judged_openverse(
                        client,
                        hits,
                        &slot,
                        used_urls,
                        seen,
                        allow_weak,
                        "openverse",
                    )
                    .await,
                )
            }
        },
    )
    .await;
    let rewrite_aspect = match original {
        Ok((Some(image), _)) => return Some((image, fallback.delivery_deadline())),
        Err(_) => return fallback.finish_with_deadline(),
        Ok((None, aspect)) => aspect,
    };

    // The existing rewrite ladder keeps its two-query / two-batch ceiling.
    // Network deadlines remain active even for a legacy synchronous judge.
    for rewrite in rewrites {
        if !fallback.can_continue(judge) {
            return fallback.finish_with_deadline();
        }
        eprintln!("[ENRICH] judge-rewrite: \"{query}\" → \"{rewrite}\"");
        let tier_deadline = earlier_deadline(deadline, fallback.deadline());
        let slot = JudgeSlot {
            deadline: judging_deadline(judge, tier_deadline),
            ..slot
        };
        let result = before_deadline(tier_deadline, async {
            let hits = fetch_relevant_openverse_list_with_aspect(
                &client,
                &rewrite,
                intent,
                rewrite_aspect,
                credentials,
            )
            .await
            .map(|list| list.hits)
            .unwrap_or_default();
            fetch_judged_openverse(
                &client,
                hits,
                &slot,
                used_urls,
                &seen,
                fallback.is_empty(),
                "openverse-rewrite",
            )
            .await
        })
        .await;
        let Ok(image) = result else {
            return fallback.finish_with_deadline();
        };
        if let Some(image) = image {
            if let Some(image) = fallback.consider("openverse-rewrite", &rewrite, image) {
                return Some((image, fallback.delivery_deadline()));
            }
        }
    }

    // The Wikimedia fallback ladder is judged as well. Until 2026-09-05 it
    // took the first page unseen, so every slot whose Openverse batches were
    // all Off fell open to an unjudged pick — the A/B audit traced its
    // remaining Off picks to exactly those slots. Two all-Off ladders now
    // leave the slot unresolved instead.
    let words: Vec<&str> = query
        .split_whitespace()
        .filter(|word| !word.is_empty())
        .collect();
    if words.len() > 2 {
        let truncated = words[..2].join(" ");
        if !fallback.can_continue(judge) {
            return fallback.finish_with_deadline();
        }
        let tier_deadline = earlier_deadline(deadline, fallback.deadline());
        let slot = JudgeSlot {
            deadline: judging_deadline(judge, tier_deadline),
            ..slot
        };
        let result = before_deadline(
            tier_deadline,
            fetch_judged_wikimedia(&client, &truncated, &slot, used_urls, fallback.is_empty()),
        )
        .await;
        let Ok(image) = result else {
            return fallback.finish_with_deadline();
        };
        if let Some(image) = image {
            if let Some(image) = fallback.consider("wikimedia", &truncated, image) {
                return Some((image, fallback.delivery_deadline()));
            }
        }
    }
    if !fallback.can_continue(judge) {
        return fallback.finish_with_deadline();
    }
    let tier_deadline = earlier_deadline(deadline, fallback.deadline());
    let slot = JudgeSlot {
        deadline: judging_deadline(judge, tier_deadline),
        ..slot
    };
    let result = before_deadline(
        tier_deadline,
        fetch_judged_wikimedia(&client, &query, &slot, used_urls, fallback.is_empty()),
    )
    .await;
    let Ok(image) = result else {
        return fallback.finish_with_deadline();
    };
    if let Some(image) = image {
        if let Some(image) = fallback.consider("wikimedia", &query, image) {
            return Some((image, fallback.delivery_deadline()));
        }
    }
    fallback.finish_with_deadline()
}

struct PreparedJudgeCandidate {
    title: String,
    data_url: String,
    thumb_jpeg: Vec<u8>,
    identity: Option<String>,
    attribution: String,
    original: Option<crate::net::providers::OriginalImageSource>,
}

async fn fetch_judged_openverse(
    client: &reqwest::Client,
    hits: Vec<RawHit>,
    slot: &JudgeSlot<'_>,
    used_urls: &Mutex<HashSet<String>>,
    seen: &Mutex<HashSet<String>>,
    allow_weak: bool,
    source: &str,
) -> Option<JudgedImage> {
    const ROUND_SIZE: usize = 5;
    const MAX_JUDGE_CANDIDATES: usize = ROUND_SIZE * 2;
    let hits = unseen_hits(hits, seen);
    let taken = hits.len().min(MAX_JUDGE_CANDIDATES);
    let mut candidates = Vec::new();
    for hit in hits.into_iter().take(MAX_JUDGE_CANDIDATES) {
        let Some((data_url, thumb_jpeg)) =
            fetch_image_and_judge_thumbnail_for_view(client, &hit.thumb_url, slot.view).await
        else {
            continue;
        };
        candidates.push(PreparedJudgeCandidate {
            identity: Some(hit_identity(&hit)),
            title: hit.title,
            data_url,
            thumb_jpeg,
            attribution: hit.attribution,
            original: hit.original,
        });
    }
    if candidates.len() < taken {
        // Thumbnail downloads that fail are the last silent way to a `k=0`;
        // name them so a slow CDN is not read as an empty catalogue.
        eprintln!(
            "[ENRICH] {source}: \"{}\" thumbnails fetched {}/{taken}",
            slot.query,
            candidates.len()
        );
    }

    let batches: Vec<Vec<Vec<u8>>> = candidates
        .chunks(ROUND_SIZE)
        .map(|batch| {
            batch
                .iter()
                .map(|candidate| candidate.thumb_jpeg.clone())
                .collect()
        })
        .collect();
    let selection = select_and_record(slot, &batches, &candidates, seen);
    eprintln!("[ENRICH] {} source={source}", format_judge_log(&selection));
    claim_ranked_image(&selection, &candidates, used_urls, allow_weak)
}

/// Judged Wikimedia fallback. Search pages arrive in relevance order; each
/// page's preferred image (thumbnail, else original) is downloaded into a
/// judge thumbnail and ranked by the same two-batch judge as the Openverse
/// candidates. A page whose identity another slot already claimed is skipped
/// up front; the winner claims both its page identity and its content digest.
async fn fetch_judged_wikimedia(
    client: &reqwest::Client,
    query: &str,
    slot: &JudgeSlot<'_>,
    used_urls: &Mutex<HashSet<String>>,
    allow_weak: bool,
) -> Option<JudgedImage> {
    const ROUND_SIZE: usize = 5;
    const MAX_JUDGE_CANDIDATES: usize = ROUND_SIZE * 2;
    let url = wikimedia_search_url(query, MAX_JUDGE_CANDIDATES)?;
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    let pages = json.get("query")?.get("pages")?.as_object()?;
    // The pages object is keyed by page id; `index` carries the search rank.
    let mut ordered: Vec<&serde_json::Value> = pages.values().collect();
    ordered.sort_by_key(|page| {
        page.get("index")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(i64::MAX)
    });

    let mut candidates = Vec::new();
    for page in ordered.into_iter().take(MAX_JUDGE_CANDIDATES) {
        let Some(identity) = wikimedia_page_identity(page) else {
            continue;
        };
        if used_urls.lock().unwrap().contains(&identity) {
            continue;
        }
        let Some(candidate_url) = wikimedia_image_candidates(page).into_iter().next() else {
            continue;
        };
        let Some((data_url, thumb_jpeg)) =
            fetch_image_and_judge_thumbnail_for_view(client, &candidate_url, slot.view).await
        else {
            continue;
        };
        let title = page
            .get("title")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string();
        candidates.push(PreparedJudgeCandidate {
            title,
            data_url,
            thumb_jpeg,
            identity: Some(identity),
            attribution: page["imageinfo"][0]["extmetadata"]["LicenseShortName"]["value"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            original: crate::net::providers::original_source(&page["imageinfo"][0], &candidate_url),
        });
    }
    if candidates.is_empty() {
        return None;
    }

    let batches: Vec<Vec<Vec<u8>>> = candidates
        .chunks(ROUND_SIZE)
        .map(|batch| {
            batch
                .iter()
                .map(|candidate| candidate.thumb_jpeg.clone())
                .collect()
        })
        .collect();
    let selection = slot.select(&batches);
    eprintln!("[ENRICH] {} source=wikimedia", format_judge_log(&selection));
    claim_ranked_image(&selection, &candidates, used_urls, allow_weak)
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

fn wikimedia_search_url(query: &str, limit: usize) -> Option<reqwest::Url> {
    let limit = limit.to_string();
    reqwest::Url::parse_with_params(
        "https://commons.wikimedia.org/w/api.php",
        &[
            ("action", "query"),
            ("generator", "search"),
            ("gsrsearch", query),
            ("gsrnamespace", "6"),
            ("gsrlimit", limit.as_str()),
            ("prop", "imageinfo"),
            ("iiprop", "url|size|mime|extmetadata"),
            ("iiurlwidth", "800"),
            ("format", "json"),
            ("origin", "*"),
        ],
    )
    .ok()
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
    used_urls
        .lock()
        .unwrap()
        .insert(image_content_identity(src))
}

fn image_content_identity(src: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut hasher);
    format!("content:{:016x}", hasher.finish())
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

#[cfg(test)]
mod wikimedia_url_tests {
    use super::wikimedia_search_url;

    #[test]
    fn judged_fallback_asks_for_a_full_candidate_page() {
        let url = wikimedia_search_url("kyoto temple", 10).expect("url");
        let pairs: Vec<(String, String)> = url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        assert!(pairs.contains(&("gsrlimit".into(), "10".into())));
        assert!(pairs.contains(&("gsrsearch".into(), "kyoto temple".into())));
    }

    #[test]
    fn unjudged_fallback_keeps_the_single_page_shape() {
        let url = wikimedia_search_url("vase", 1).expect("url");
        assert!(url.query_pairs().any(|(k, v)| k == "gsrlimit" && v == "1"));
    }
}
