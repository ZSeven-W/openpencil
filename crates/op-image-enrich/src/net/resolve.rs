//! The unjudged single-image ladder (desktop session default, mobile FFI,
//! headless op-smoke fill) and the relevance-fenced Openverse list shared
//! with the judged ladder in `fetch.rs`.
//!
//! Until 2026-09-27 the unjudged ladder never ran the relevance fence: it
//! picked the Openverse result with the most raw title-token overlap and fell
//! back to the first result at zero overlap, retried with the query's FIRST
//! two words (dropping the head noun: "serum oil bottle" → "serum oil"), and
//! took Wikimedia's first page unseen. A skincare landing page shipped a
//! street-graffiti "snake oil serum" mural and an Instagram tattoo shot
//! ("… disposable tubes … cleanser #tattoo") as product photos that way.
//! Both lists now pass the same subject/photo/brand fence as the judged
//! ladder, with the photo contract read from the slot's image prompt too.

use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Duration;

use crate::net::fetch::{first_unused_renderable_image_src, settle_provider_identity};
use crate::net::providers::{
    fetch_openverse_list_with_aspect, fetch_relevant_wikimedia_list_for_intent,
    retain_relevant_hits_for_intent, simplify_search_query, two_keyword_retry, RawHit,
    WebOpenverseCredentials,
};
use crate::net::search_trace::{short_title, trace_hits, trace_line};
use crate::ImageAspectRatio;

/// The image a ladder settled on, with enough provenance for a one-line
/// diagnostic (`[IMAGE] <node> query=… -> <provider> "<title>"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedImage {
    /// Renderable `data:` URL written into the slot.
    pub src: String,
    /// Ladder stage that produced it (`openverse`, `openverse-retry`,
    /// `openverse-rewrite`, `wikimedia`).
    pub provider: &'static str,
    /// The query actually sent to that provider.
    pub provider_query: String,
    /// Catalogue title of the chosen hit.
    pub title: String,
}

impl ResolvedImage {
    /// One-line, URL-free provenance: `openverse "Title" (via "q")`.
    pub fn describe(&self) -> String {
        format!(
            "{} \"{}\" (via \"{}\")",
            self.provider,
            short_title(&self.title, 80),
            short_title(&self.provider_query, 60)
        )
    }
}

/// A relevance-fenced provider list plus the stage/query that produced it.
pub(crate) struct FencedList {
    pub(crate) hits: Vec<RawHit>,
    pub(crate) provider: &'static str,
    pub(crate) provider_query: String,
}

pub fn fetch_first_image_resolved_blocking(
    query: &str,
    intent: &str,
    aspect_ratio: Option<ImageAspectRatio>,
    credentials: Option<&WebOpenverseCredentials>,
    used_urls: &Mutex<HashSet<String>>,
) -> Option<ResolvedImage> {
    let query = query.trim();
    if query.is_empty() {
        return None;
    }
    // Shared runtime bridge: a private per-call runtime aborts with "Cannot
    // start a runtime from within a runtime" once this search is reached from
    // a tokio worker (design-loop / MCP driven runs).
    crate::net::block_on_image_runtime(fetch_first_image_resolved(
        query,
        intent,
        aspect_ratio,
        credentials,
        used_urls,
    ))
}

/// Unjudged ladder: fenced Openverse (primary, then one concrete retry) →
/// fenced Wikimedia. The first fenced hit, in fence rank order, whose
/// thumbnail renders and is not already used by another slot wins. When
/// every list is fenced empty the slot stays unresolved (fallback tile)
/// rather than filling with an unrelated picture.
pub async fn fetch_first_image_resolved(
    query: &str,
    intent: &str,
    aspect_ratio: Option<ImageAspectRatio>,
    credentials: Option<&WebOpenverseCredentials>,
    used_urls: &Mutex<HashSet<String>>,
) -> Option<ResolvedImage> {
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .timeout(Duration::from_secs(8))
        .user_agent(concat!("openpencil-desktop/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;
    let query = simplify_search_query(query);
    trace_line(format_args!(
        "ladder=unjudged query=\"{query}\" intent=\"{}\" aspect={}",
        short_title(intent, 90),
        aspect_ratio.map_or("any", ImageAspectRatio::as_openverse_param)
    ));
    // The aspect filter is a preference (the slot crops its fill), not part
    // of the subject contract: the square filter answered zero hits for both
    // incident queries while the unfiltered catalogue had fenced product
    // shots. Relax it once before leaving Openverse.
    let aspect_attempts = if aspect_ratio.is_some() {
        vec![aspect_ratio, None]
    } else {
        vec![None]
    };
    for aspect in aspect_attempts {
        if aspect.is_none() && aspect_ratio.is_some() {
            trace_line("openverse: aspect filter fenced empty, retrying without it");
        }
        let Some(list) =
            fetch_relevant_openverse_list_with_aspect(&client, &query, intent, aspect, credentials)
                .await
        else {
            // Request-level failure (429 / network): don't hammer the API.
            break;
        };
        if let Some(resolved) = claim_first_fenced_hit(&client, list, "openverse:", used_urls).await
        {
            return Some(resolved);
        }
    }
    let wiki = FencedList {
        hits: fetch_relevant_wikimedia_list_for_intent(&client, &query, intent).await,
        provider: "wikimedia",
        provider_query: query.clone(),
    };
    let resolved = claim_first_fenced_hit(&client, wiki, "wikimedia:", used_urls).await;
    if resolved.is_none() {
        trace_line(format_args!(
            "chosen: none for \"{query}\" (every provider list fenced empty or unusable)"
        ));
    }
    resolved
}

/// Walk a fenced list in rank order and claim the first hit whose provider
/// identity and rendered content are both unused.
async fn claim_first_fenced_hit(
    client: &reqwest::Client,
    list: FencedList,
    identity_prefix: &str,
    used_urls: &Mutex<HashSet<String>>,
) -> Option<ResolvedImage> {
    for hit in list.hits {
        let identity = if hit.id.trim().is_empty() {
            format!("{identity_prefix}url:{}", hit.thumb_url)
        } else {
            format!("{identity_prefix}{}", hit.id)
        };
        if !used_urls.lock().unwrap().insert(identity.clone()) {
            trace_line(format_args!(
                "skip \"{}\": already used by another slot",
                short_title(&hit.title, 80)
            ));
            continue;
        }
        let outcome =
            first_unused_renderable_image_src(client, vec![hit.thumb_url.clone()], used_urls).await;
        let Some(src) = settle_provider_identity(used_urls, &identity, outcome) else {
            trace_line(format_args!(
                "skip \"{}\": thumbnail unusable or duplicate content",
                short_title(&hit.title, 80)
            ));
            continue;
        };
        let resolved = ResolvedImage {
            src,
            provider: list.provider,
            provider_query: list.provider_query.clone(),
            title: hit.title,
        };
        trace_line(format_args!(
            "chosen: {} — highest-ranked fenced hit whose thumbnail rendered",
            resolved.describe()
        ));
        return Some(resolved);
    }
    None
}

/// Fetch and relevance-fence one Openverse list, retrying once with the
/// concrete tail of the query when the primary answer is empty or fully
/// fenced. The fence always follows the authored `query` + `intent`, so a
/// shorter provider query never loosens the subject or photo contract.
pub(crate) async fn fetch_relevant_openverse_list_with_aspect(
    client: &reqwest::Client,
    query: &str,
    intent: &str,
    aspect_ratio: Option<ImageAspectRatio>,
    credentials: Option<&WebOpenverseCredentials>,
) -> Option<FencedList> {
    fenced_openverse_list_with(query, intent, |candidate| {
        let client = client.clone();
        let credentials = credentials.cloned();
        async move {
            fetch_openverse_list_with_aspect(
                &client,
                &candidate,
                aspect_ratio,
                credentials.as_ref(),
            )
            .await
        }
    })
    .await
}

/// Transport-free core of [`fetch_relevant_openverse_list_with_aspect`]:
/// `fetch_list` answers one provider query (`None` = request failure).
pub(crate) async fn fenced_openverse_list_with<F, Fut>(
    query: &str,
    intent: &str,
    mut fetch_list: F,
) -> Option<FencedList>
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = Option<Vec<RawHit>>>,
{
    let hits = fetch_list(query.to_string()).await?;
    trace_hits("openverse", query, (query, intent), &hits);
    let relevant = retain_relevant_hits_for_intent(hits, query, intent);
    if !relevant.is_empty() {
        return Some(FencedList {
            hits: relevant,
            provider: "openverse",
            provider_query: query.to_string(),
        });
    }

    let Some(retry_query) = two_keyword_retry(query) else {
        return Some(FencedList {
            hits: relevant,
            provider: "openverse",
            provider_query: query.to_string(),
        });
    };
    let retry = fetch_list(retry_query.clone()).await?;
    trace_hits("openverse-retry", &retry_query, (query, intent), &retry);
    let retry_total = retry.len();
    let relevant = retain_relevant_hits_for_intent(retry, query, intent);
    if relevant.is_empty() && retry_total > 0 {
        eprintln!(
            "[ENRICH] openverse: \"{query}\" {retry_total} hits for \"{retry_query}\", none lexically relevant"
        );
    }
    Some(FencedList {
        hits: relevant,
        provider: "openverse-retry",
        provider_query: retry_query,
    })
}

#[cfg(test)]
#[path = "resolve_tests.rs"]
mod tests;
