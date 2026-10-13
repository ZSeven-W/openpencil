//! Background search and generation workers, plus their single-flight publication.

use super::*;

// The parameter list mirrors the session state the spawned worker needs;
// grouping it into a context struct would move nine fields around for one
// call site, so the lint is allowed here.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_job(
    target: ImageSearchTarget,
    credentials: Option<OpenverseCredentials>,
    used_urls: Arc<Mutex<HashSet<String>>>,
    search_memo: Arc<Mutex<HashMap<SearchIntentKey, SearchMemoEntry>>>,
    request_id: u64,
    judge: Arc<dyn ImageRelevanceJudge>,
    judge_enabled: bool,
    intent: String,
    deadline: Option<Instant>,
) -> ImageSearchJob {
    let (tx, rx) = mpsc::channel();
    let node_id = target.node_id.clone();
    let aspect_ratio = target.aspect_ratio;
    let key = search_intent_key(&target.query, aspect_ratio);
    let intent = Some(intent);
    let judge_intent = target
        .prompt
        .as_deref()
        .filter(|prompt| !prompt.trim().is_empty())
        .unwrap_or(target.query.as_str())
        .to_string();
    // One full search intent, one in-flight request and one session result.
    // Rebuilt nodes subscribe to the same pending request instead of racing
    // duplicate searches; completed intents return from the memo.
    {
        let mut memo = search_memo.lock().unwrap();
        match memo.get_mut(&key) {
            Some(SearchMemoEntry::Ready(url)) => {
                let _ = tx.send(JobOutcome::Search(Some(url.clone())));
                return ImageSearchJob {
                    node_id,
                    deadline,
                    intent,
                    fallback_target: None,
                    rx,
                };
            }
            Some(SearchMemoEntry::Pending { waiters, .. }) => {
                waiters.push(tx);
                return ImageSearchJob {
                    node_id,
                    deadline,
                    intent,
                    fallback_target: None,
                    rx,
                };
            }
            None => {
                memo.insert(
                    key.clone(),
                    SearchMemoEntry::Pending {
                        request_id,
                        waiters: vec![tx],
                    },
                );
            }
        }
    }
    std::thread::spawn(move || {
        let url = if judge_enabled {
            fetch_first_image_url_blocking_with_judge_and_deadline(
                &target.query,
                aspect_ratio,
                credentials.as_ref().map(OpenverseCredentials::as_web),
                &used_urls,
                judge.as_ref(),
                &judge_intent,
                deadline,
            )
        } else {
            // No configured judge: the unjudged ladder, still fenced for
            // relevance. The slot's image prompt is passed as intent so a
            // product-photo slot also rejects artwork hits (a tattoo shot and
            // a graffiti mural once filled skincare product cards).
            op_image_enrich::net::resolve::fetch_first_image_resolved_blocking(
                &target.query,
                &judge_intent,
                aspect_ratio,
                credentials.as_ref().map(OpenverseCredentials::as_web),
                &used_urls,
            )
            .map(|resolved| resolved.src)
        };
        publish_search_result(&search_memo, key, request_id, url);
    });
    ImageSearchJob {
        node_id,
        deadline,
        intent,
        fallback_target: None,
        rx,
    }
}

/// Publish only into the exact Pending entry that launched this request. A
/// reset may remove it and a new document may insert the same key before the
/// old thread returns; the request id keeps that old result isolated.
pub(super) fn publish_search_result(
    search_memo: &Arc<Mutex<HashMap<SearchIntentKey, SearchMemoEntry>>>,
    key: SearchIntentKey,
    request_id: u64,
    url: Option<String>,
) -> bool {
    let waiters = {
        let mut memo = search_memo.lock().unwrap();
        let matches_request = matches!(
            memo.get(&key),
            Some(SearchMemoEntry::Pending {
                request_id: pending_id,
                ..
            }) if *pending_id == request_id
        );
        if !matches_request {
            return false;
        }
        let Some(SearchMemoEntry::Pending { waiters, .. }) = memo.remove(&key) else {
            unreachable!("request identity was checked under the same lock")
        };
        if let Some(found) = url.as_ref() {
            memo.insert(key, SearchMemoEntry::Ready(found.clone()));
        }
        waiters
    };
    for waiter in waiters {
        let _ = waiter.send(JobOutcome::Search(url.clone()));
    }
    true
}

/// Enrich via the configured image-GEN model instead of stock search. Prefers the
/// node's bound `image_prompt`; falls back to the search keyword so a node that
/// only carries `image_search_query` still generates. Same `ImageSearchJob`
/// channel as search, so `poll_into` applies the resulting src identically.
pub(super) fn spawn_gen_job(
    target: ImageSearchTarget,
    profile: ImageGenProfile,
    deadline: Option<Instant>,
) -> ImageSearchJob {
    let (tx, rx) = mpsc::channel();
    let node_id = target.node_id.clone();
    let intent = Some(intent_fingerprint(&target, Some(&profile)));
    let prompt = target
        .prompt
        .clone()
        .filter(|p: &String| !p.trim().is_empty())
        .unwrap_or_else(|| target.query.clone());
    let width = target.width;
    let height = target.height;
    std::thread::spawn(move || {
        let result =
            crate::image_generate_host::run_generate_blocking(&prompt, &profile, width, height);
        let _ = tx.send(JobOutcome::Generate(result));
    });
    ImageSearchJob {
        node_id,
        deadline,
        intent,
        // Kept so a degradable failure can re-run the slot as a search.
        fallback_target: Some(target),
        rx,
    }
}

pub(super) fn spawn_unavailable_gen_job(
    target: ImageSearchTarget,
    deadline: Option<Instant>,
) -> ImageSearchJob {
    let (tx, rx) = mpsc::channel();
    let node_id = target.node_id.clone();
    let intent = Some(intent_fingerprint(&target, None));
    // No profile is a CONFIG fault, not a runtime one: the failure stays
    // visible (placeholder) and must never degrade into a stock search.
    let _ = tx.send(JobOutcome::Search(None));
    ImageSearchJob {
        node_id,
        deadline,
        intent,
        fallback_target: None,
        rx,
    }
}
