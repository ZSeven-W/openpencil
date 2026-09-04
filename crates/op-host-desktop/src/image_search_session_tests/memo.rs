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
        search_intent_key("Bali, Indonesia", None),
        SearchMemoEntry::Ready(good.clone()),
    );
    used_urls.lock().unwrap().insert(good.clone());

    // The rebuilt card asks again — differently spelled, same subject.
    let target = ImageSearchTarget {
        node_id: op_editor_core::NodeId::new("n99".to_string()),
        query: "  bali indonesia  ".to_string(),
        prompt: None,
        mode: ImageRequestMode::Search,
        aspect_ratio: None,
        width: None,
        height: None,
    };
    let job = spawn_job(
        target,
        None,
        Arc::clone(&used_urls),
        Arc::clone(&resolved),
        1,
        Arc::new(NoJudge),
        false,
    );
    let answer = job
        .rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("the memo answers without touching the network");
    assert_eq!(
        answer,
        Some(good),
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

    let key = search_intent_key("Bali Indonesia", None);
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
        mode: ImageRequestMode::Search,
        aspect_ratio: None,
        width: None,
        height: None,
    };
    let job = spawn_job(
        target,
        None,
        Arc::new(Mutex::new(HashSet::new())),
        Arc::clone(&memo),
        8,
        Arc::new(NoJudge),
        false,
    );

    let waiters = match memo.lock().unwrap().remove(&key) {
        Some(SearchMemoEntry::Pending { waiters, .. }) => waiters,
        _ => panic!("the second caller joins the pending intent"),
    };
    assert_eq!(waiters.len(), 1, "no second fetch thread was created");
    waiters[0]
        .send(Some("https://example.org/bali.jpg".into()))
        .unwrap();
    assert_eq!(
        job.rx.recv_timeout(Duration::from_secs(1)).unwrap(),
        Some("https://example.org/bali.jpg".into())
    );
}

#[test]
fn stale_pre_reset_request_cannot_publish_into_same_key_in_new_session() {
    use super::super::{
        publish_search_result, search_intent_key, SearchIntentKey, SearchMemoEntry,
    };
    use std::collections::HashMap;
    use std::sync::{mpsc, Arc, Mutex};

    let key = search_intent_key("Bali Indonesia", None);
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
        Some("https://new.example/bali.jpg".into())
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
        search_intent_key("Bali, Indonesia", Some(ImageAspectRatio::Square)),
        search_intent_key("bali indonesia", Some(ImageAspectRatio::Square)),
        "fetch-equivalent spelling shares one intent"
    );
    assert_ne!(
        search_intent_key("Bali Indonesia", Some(ImageAspectRatio::Square)),
        search_intent_key("Bali Indonesia", Some(ImageAspectRatio::Wide)),
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
        search_intent_key(album, Some(ImageAspectRatio::Square)),
        search_intent_key(playlist, Some(ImageAspectRatio::Square)),
        "provider adaptation must not merge authored image identity"
    );

    let dome = "santorini greece white buildings blue dome";
    let beach = "santorini greece white buildings sunset beach";
    assert_eq!(simplify_search_query(dome), simplify_search_query(beach));
    assert_ne!(
        search_intent_key(dome, Some(ImageAspectRatio::Wide)),
        search_intent_key(beach, Some(ImageAspectRatio::Wide)),
        "words beyond the provider's four-keyword cap remain part of identity"
    );
}
