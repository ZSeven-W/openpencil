//! Search-memo + dedup tests (single-flight, repeat queries, generation
//! detachment across resets). Split out of the flat
//! `image_search_session_tests.rs` to keep every file under the 800-line
//! cap; pure code motion.

use super::super::*;
use super::*;

#[test]
fn a_repeat_query_gets_the_same_photo_back_not_a_dedup_downgrade() {
    use super::super::{
        search_intent_key, spawn_job, ImageSearchTarget, SearchIntentKey, SearchMemoEntry,
    };
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex};

    let used_urls: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let resolved: Arc<Mutex<HashMap<SearchIntentKey, SearchMemoEntry>>> =
        Arc::new(Mutex::new(HashMap::new()));
    // The first search already answered "Bali Indonesia" and marked its photo used.
    let good = "https://example.org/bali-temple.jpg".to_string();
    resolved.lock().unwrap().insert(
        search_intent_key("Bali, Indonesia", None, None, None),
        SearchMemoEntry::Ready(good.clone()),
    );
    used_urls.lock().unwrap().insert(good.clone());

    // The rebuilt card asks again — differently spelled, same subject.
    let target = ImageSearchTarget {
        node_id: op_editor_core::NodeId::new("n99".to_string()),
        query: "  bali indonesia  ".to_string(),
        prompt: None,
        page_context: None,
        generation_subject: None,
        mode: ImageRequestMode::Search,
        aspect_ratio: None,
        width: None,
        height: None,
        judge_view: None,
    };
    let job = spawn_job(
        target,
        None,
        Arc::clone(&used_urls),
        Arc::clone(&resolved),
        1,
        Arc::new(NoJudge),
        false,
        format!(
            "search|{}|{:?}",
            "kitten photo".to_lowercase(),
            Option::<op_image_enrich::ImageAspectRatio>::None
        ),
        None,
    );
    let answer = job
        .rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("the memo answers without touching the network");
    assert_eq!(
        answer,
        JobOutcome::Search(Some(good)),
        "the rebuilt card gets ITS photo back, not the next-best junk result"
    );
}

#[test]
fn a_pending_search_intent_is_singleflight() {
    use super::super::{
        search_intent_key, spawn_job, ImageSearchTarget, SearchIntentKey, SearchMemoEntry,
    };
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex};

    let key = search_intent_key("Bali Indonesia", None, None, None);
    let memo: Arc<Mutex<HashMap<SearchIntentKey, SearchMemoEntry>>> =
        Arc::new(Mutex::new(HashMap::from([(
            key.clone(),
            SearchMemoEntry::Pending {
                request_id: 7,
                waiters: Vec::new(),
            },
        )])));
    let target = ImageSearchTarget {
        node_id: NodeId::new("n100"),
        query: "Bali, Indonesia".into(),
        prompt: None,
        page_context: None,
        generation_subject: None,
        mode: ImageRequestMode::Search,
        aspect_ratio: None,
        width: None,
        height: None,
        judge_view: None,
    };
    let job = spawn_job(
        target,
        None,
        Arc::new(Mutex::new(HashSet::new())),
        Arc::clone(&memo),
        8,
        Arc::new(NoJudge),
        false,
        format!(
            "search|{}|{:?}",
            "bali, indonesia",
            Option::<op_image_enrich::ImageAspectRatio>::None
        ),
        None,
    );

    let waiters = match memo.lock().unwrap().remove(&key) {
        Some(SearchMemoEntry::Pending { waiters, .. }) => waiters,
        _ => panic!("the second caller joins the pending intent"),
    };
    assert_eq!(waiters.len(), 1, "no second fetch thread was created");
    waiters[0]
        .send(JobOutcome::Search(Some(
            "https://example.org/bali.jpg".into(),
        )))
        .unwrap();
    assert_eq!(
        job.rx.recv_timeout(Duration::from_secs(1)).unwrap(),
        JobOutcome::Search(Some("https://example.org/bali.jpg".into()))
    );
}

#[test]
fn stale_pre_reset_request_cannot_publish_into_same_key_in_new_session() {
    use super::super::{
        publish_search_result, search_intent_key, SearchIntentKey, SearchMemoEntry,
    };
    use std::collections::HashMap;
    use std::sync::{mpsc, Arc, Mutex};

    let key = search_intent_key("Bali Indonesia", None, None, None);
    let memo: Arc<Mutex<HashMap<SearchIntentKey, SearchMemoEntry>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let (old_tx, _old_rx) = mpsc::channel();
    memo.lock().unwrap().insert(
        key.clone(),
        SearchMemoEntry::Pending {
            request_id: 10,
            waiters: vec![old_tx],
        },
    );

    // reset(), followed by a new document asking for the same intent.
    memo.lock().unwrap().clear();
    let (new_tx, new_rx) = mpsc::channel();
    memo.lock().unwrap().insert(
        key.clone(),
        SearchMemoEntry::Pending {
            request_id: 11,
            waiters: vec![new_tx],
        },
    );

    assert!(!publish_search_result(
        &memo,
        key.clone(),
        10,
        Some("https://old.example/bali.jpg".into())
    ));
    assert!(matches!(new_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
    assert!(publish_search_result(
        &memo,
        key,
        11,
        Some("https://new.example/bali.jpg".into())
    ));
    assert_eq!(
        new_rx.recv_timeout(Duration::from_secs(1)).unwrap(),
        JobOutcome::Search(Some("https://new.example/bali.jpg".into()))
    );
}

#[test]
fn reset_detaches_dedup_and_memo_generations_from_old_threads() {
    let mut session = ImageSearchSession::default();
    let old_used = std::sync::Arc::clone(&session.used_urls);
    let old_memo = std::sync::Arc::clone(&session.search_memo);
    old_used.lock().unwrap().insert("openverse:old".into());

    session.reset();

    assert!(!std::sync::Arc::ptr_eq(&old_used, &session.used_urls));
    assert!(!std::sync::Arc::ptr_eq(&old_memo, &session.search_memo));
    old_used
        .lock()
        .unwrap()
        .insert("openverse:late-old-thread".into());
    assert!(
        session.used_urls.lock().unwrap().is_empty(),
        "late old-document claims stay in the detached generation"
    );
}

#[test]
fn search_memo_separates_aspect_ratio_intents() {
    use super::super::{search_intent_key, ImageAspectRatio};

    assert_eq!(
        search_intent_key(
            "Bali, Indonesia",
            Some(ImageAspectRatio::Square),
            None,
            None
        ),
        search_intent_key("bali indonesia", Some(ImageAspectRatio::Square), None, None),
        "fetch-equivalent spelling shares one intent"
    );
    assert_ne!(
        search_intent_key("Bali Indonesia", Some(ImageAspectRatio::Square), None, None),
        search_intent_key("Bali Indonesia", Some(ImageAspectRatio::Wide), None, None),
        "a cover and hero must not share a cached crop"
    );
}

#[test]
fn memo_identity_keeps_lossy_provider_query_collisions_separate() {
    let album = "album cover neon lights night";
    let playlist = "playlist artwork neon lights night";
    assert_eq!(
        simplify_search_query(album),
        simplify_search_query(playlist),
        "both authored intents intentionally adapt to the same photo-corpus query"
    );
    assert_ne!(
        search_intent_key(album, Some(ImageAspectRatio::Square), None, None),
        search_intent_key(playlist, Some(ImageAspectRatio::Square), None, None),
        "provider adaptation must not merge authored image identity"
    );

    let dome = "santorini greece white buildings blue dome";
    let beach = "santorini greece white buildings sunset beach";
    assert_eq!(simplify_search_query(dome), simplify_search_query(beach));
    assert_ne!(
        search_intent_key(dome, Some(ImageAspectRatio::Wide), None, None),
        search_intent_key(beach, Some(ImageAspectRatio::Wide), None, None),
        "words beyond the provider's four-keyword cap remain part of identity"
    );
}

fn directed_stock_target(prompt: Option<&str>) -> ImageSearchTarget {
    ImageSearchTarget {
        node_id: NodeId::new("directed-photo"),
        query: "ceramic vase".into(),
        prompt: prompt.map(str::to_string),
        page_context: None,
        generation_subject: None,
        mode: ImageRequestMode::Search,
        aspect_ratio: Some(ImageAspectRatio::Square),
        width: Some(240.0),
        height: Some(240.0),
        judge_view: None,
    }
}

#[test]
fn ready_memo_keeps_different_art_directions_separate() {
    let warm = directed_stock_target(Some("Warm studio photograph on an ivory background"));
    let cool = directed_stock_target(Some("Cool outdoor photograph on dark stone"));
    let memo = Arc::new(Mutex::new(HashMap::new()));
    let used_urls = Arc::new(Mutex::new(HashSet::new()));
    // Seed the legacy key too: a regression must return the wrong cached image,
    // not launch an external request during this test.
    memo.lock().unwrap().insert(
        search_intent_key(&warm.query, warm.aspect_ratio, None, warm.judge_view),
        SearchMemoEntry::Ready("legacy-query-only.jpg".into()),
    );
    for (target, url) in [(&warm, "warm.jpg"), (&cool, "cool.jpg")] {
        memo.lock().unwrap().insert(
            search_intent_key(
                &target.query,
                target.aspect_ratio,
                target.prompt.as_deref(),
                target.judge_view,
            ),
            SearchMemoEntry::Ready(url.into()),
        );
    }

    for (target, url) in [(warm, "warm.jpg"), (cool, "cool.jpg")] {
        let intent = intent_fingerprint(&target, None);
        let job = spawn_job(
            target,
            None,
            Arc::clone(&used_urls),
            Arc::clone(&memo),
            1,
            Arc::new(NoJudge),
            false,
            intent,
            None,
        );
        assert_eq!(
            job.rx.recv_timeout(Duration::from_secs(1)).unwrap(),
            JobOutcome::Search(Some(url.into()))
        );
    }
}

#[test]
fn pending_art_direction_does_not_join_another_directions_search() {
    for old_ready in [true, false] {
        let warm = directed_stock_target(Some("Warm studio photograph on an ivory background"));
        let cool = directed_stock_target(Some("Cool outdoor photograph on dark stone"));
        let warm_key = search_intent_key(
            &warm.query,
            warm.aspect_ratio,
            warm.prompt.as_deref(),
            warm.judge_view,
        );
        let cool_key = search_intent_key(
            &cool.query,
            cool.aspect_ratio,
            cool.prompt.as_deref(),
            cool.judge_view,
        );
        assert_ne!(warm_key, cool_key);
        let memo = Arc::new(Mutex::new(HashMap::from([
            (
                search_intent_key(&warm.query, warm.aspect_ratio, None, warm.judge_view),
                SearchMemoEntry::Ready("legacy-query-only.jpg".into()),
            ),
            (
                warm_key.clone(),
                if old_ready {
                    SearchMemoEntry::Ready("warm.jpg".into())
                } else {
                    SearchMemoEntry::Pending {
                        request_id: 7,
                        waiters: Vec::new(),
                    }
                },
            ),
            (
                cool_key.clone(),
                SearchMemoEntry::Pending {
                    request_id: 8,
                    waiters: Vec::new(),
                },
            ),
        ])));
        let intent = intent_fingerprint(&cool, None);
        let job = spawn_job(
            cool,
            None,
            Arc::new(Mutex::new(HashSet::new())),
            Arc::clone(&memo),
            9,
            Arc::new(NoJudge),
            false,
            intent,
            None,
        );
        assert!(matches!(job.rx.try_recv(), Err(TryRecvError::Empty)));
        {
            let entries = memo.lock().unwrap();
            let Some(SearchMemoEntry::Pending { waiters, .. }) = entries.get(&cool_key) else {
                panic!("the requested direction stays pending")
            };
            assert_eq!(waiters.len(), 1);
        }
        if !old_ready {
            assert!(publish_search_result(
                &memo,
                warm_key,
                7,
                Some("warm.jpg".into())
            ));
            assert!(matches!(job.rx.try_recv(), Err(TryRecvError::Empty)));
        }
        assert!(publish_search_result(
            &memo,
            cool_key,
            8,
            Some("cool.jpg".into())
        ));
        assert_eq!(
            job.rx.recv_timeout(Duration::from_secs(1)).unwrap(),
            JobOutcome::Search(Some("cool.jpg".into()))
        );
    }
}

#[test]
fn blank_prompts_keep_query_only_memo_and_fingerprint_behavior() {
    let original = directed_stock_target(None);
    let key = search_intent_key(
        &original.query,
        original.aspect_ratio,
        None,
        original.judge_view,
    );
    let expected = intent_fingerprint(&original, None);
    assert_eq!(expected, "search|ceramic vase|Some(Square)");
    let memo = Arc::new(Mutex::new(HashMap::from([(
        key.clone(),
        SearchMemoEntry::Ready("query-only.jpg".into()),
    )])));
    for prompt in [None, Some(""), Some(" \n\t ")] {
        let target = directed_stock_target(prompt);
        assert_eq!(
            search_intent_key(
                &target.query,
                target.aspect_ratio,
                target.prompt.as_deref(),
                target.judge_view
            ),
            key
        );
        assert_eq!(intent_fingerprint(&target, None), expected);
        let job = spawn_job(
            target,
            None,
            Arc::new(Mutex::new(HashSet::new())),
            Arc::clone(&memo),
            1,
            Arc::new(NoJudge),
            false,
            expected.clone(),
            None,
        );
        assert_eq!(
            job.rx.recv_timeout(Duration::from_secs(1)).unwrap(),
            JobOutcome::Search(Some("query-only.jpg".into()))
        );
    }
    assert_eq!(
        search_intent_key(
            &original.query,
            original.aspect_ratio,
            Some(" warm studio "),
            original.judge_view
        ),
        search_intent_key(
            &original.query,
            original.aspect_ratio,
            Some("warm studio"),
            original.judge_view
        )
    );
}

#[test]
fn generation_fingerprint_still_tracks_prompt_and_size_independent_of_stock_query() {
    let mut target = directed_stock_target(Some("Warm studio photograph"));
    target.mode = ImageRequestMode::Generate;
    let original = intent_fingerprint(&target, None);
    target.query = "vase product".into();
    assert_eq!(intent_fingerprint(&target, None), original);
    target.prompt = Some("Cool outdoor photograph".into());
    assert_ne!(intent_fingerprint(&target, None), original);
    target.prompt = Some("Warm studio photograph".into());
    target.width = Some(480.0);
    assert_ne!(intent_fingerprint(&target, None), original);
}
