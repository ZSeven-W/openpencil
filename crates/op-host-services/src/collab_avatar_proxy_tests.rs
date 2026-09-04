use super::*;
use op_editor_ui::collab_avatar_runtime::{
    begin_collab_avatar_generation, register_collab_avatar_url,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// The avatar registry is process-global, and every test here rotates its
/// generation to start from a clean roster.
static REGISTRY_LOCK: Mutex<()> = Mutex::new(());
static GENERATION: AtomicU64 = AtomicU64::new(1);

fn reset_registry() -> std::sync::MutexGuard<'static, ()> {
    let guard = REGISTRY_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    assert!(begin_collab_avatar_generation(
        GENERATION.fetch_add(1, Ordering::Relaxed)
    ));
    guard
}

/// Smallest byte string `encoded_image_dimensions` accepts as a PNG.
fn png_bytes(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = vec![0; 32];
    bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
    bytes[8..12].copy_from_slice(&13_u32.to_be_bytes());
    bytes[12..16].copy_from_slice(b"IHDR");
    bytes[16..20].copy_from_slice(&width.to_be_bytes());
    bytes[20..24].copy_from_slice(&height.to_be_bytes());
    bytes
}

fn body_for(participant_key: &str) -> String {
    serde_json::json!({ "participantKey": participant_key }).to_string()
}

#[test]
fn a_queued_roster_url_is_fetched_and_returned_as_opaque_bytes() {
    let _guard = reset_registry();
    let url = "https://cdn.example/peer.png?signature=secret-token";
    assert!(register_collab_avatar_url("peer-one", Some(url)));

    let fetched = std::cell::RefCell::new(Vec::new());
    let payload = resolve_avatar(&body_for("peer-one"), |request| {
        fetched.borrow_mut().push(request.url().to_string());
        Some(png_bytes(48, 48))
    })
    .expect("the queued participant resolves after one fetch");

    assert_eq!(fetched.borrow().as_slice(), [url]);
    assert_eq!(payload.participant_key, "peer-one");
    assert_eq!(payload.encoded, png_bytes(48, 48));
    assert_eq!(payload.revision.len(), 16);
    assert!(!payload.revision.contains("secret"));
}

#[test]
fn a_second_request_is_served_from_the_registry_without_refetching() {
    let _guard = reset_registry();
    assert!(register_collab_avatar_url(
        "peer-cached",
        Some("https://cdn.example/cached.png?signature=secret-token"),
    ));
    let first = resolve_avatar(&body_for("peer-cached"), |_| Some(png_bytes(32, 32)))
        .expect("first request fetches");

    // The real handler runs the network fetcher; taking the fast path is what
    // keeps it off the wire, so this also proves the response body carries no
    // trace of the signed source URL.
    let reply = avatar(&body_for("peer-cached"));
    assert_eq!(reply.status, "200 OK");
    assert!(reply.body.contains(&first.revision));
    assert!(!reply.body.contains("secret-token"));
    assert!(!reply.body.contains("cdn.example"));
}

#[test]
fn an_unregistered_participant_reports_no_avatar() {
    let _guard = reset_registry();
    assert_eq!(
        resolve_avatar(&body_for("peer-absent"), |_| Some(png_bytes(16, 16))),
        Err(CollabAvatarProxyError::NotAvailable)
    );
    assert_eq!(avatar(&body_for("peer-absent")).status, "404 Not Found");
}

#[test]
fn a_failed_fetch_drains_the_queue_without_serving_bytes() {
    let _guard = reset_registry();
    assert!(register_collab_avatar_url(
        "peer-fails",
        Some("https://cdn.example/fails.png"),
    ));
    let attempts = std::cell::Cell::new(0_u32);
    assert_eq!(
        resolve_avatar(&body_for("peer-fails"), |_| {
            attempts.set(attempts.get() + 1);
            None
        }),
        Err(CollabAvatarProxyError::NotAvailable)
    );
    assert_eq!(attempts.get(), 1, "the queued request was taken once");

    // The failed request was completed rather than left in flight, so the
    // queue is empty and a retry does not re-run the same fetch.
    assert_eq!(
        resolve_avatar(&body_for("peer-fails"), |_| Some(png_bytes(16, 16))),
        Err(CollabAvatarProxyError::NotAvailable)
    );
    assert_eq!(attempts.get(), 1);
}

#[test]
fn only_the_asked_for_participant_is_served_but_the_others_still_land() {
    let _guard = reset_registry();
    assert!(register_collab_avatar_url(
        "peer-a",
        Some("https://cdn.example/a.png")
    ));
    assert!(register_collab_avatar_url(
        "peer-b",
        Some("https://cdn.example/b.png")
    ));

    let payload = resolve_avatar(&body_for("peer-b"), |_| Some(png_bytes(24, 24)))
        .expect("peer-b resolves in the same drain pass");
    assert_eq!(payload.participant_key, "peer-b");

    // peer-a was drained by the same pass; its bytes are cached, so asking for
    // it now must not need the fetcher at all.
    let peer_a = resolve_avatar(&body_for("peer-a"), |_| {
        panic!("peer-a was already completed by the earlier drain")
    })
    .expect("peer-a is cached");
    assert_ne!(peer_a.revision, payload.revision);
}

#[test]
fn malformed_oversized_and_unsafe_bodies_are_rejected_before_any_fetch() {
    let _guard = reset_registry();
    let never = |_: &CollabAvatarFetchRequest| -> Option<Vec<u8>> {
        panic!("a rejected body must never reach the fetcher")
    };
    let oversized = serde_json::json!({ "participantKey": "x".repeat(MAX_BODY_BYTES) }).to_string();
    for (body, expected) in [
        (oversized, CollabAvatarProxyError::BodyTooLarge),
        (
            "not json".to_string(),
            CollabAvatarProxyError::MalformedRequest,
        ),
        ("{}".to_string(), CollabAvatarProxyError::MalformedRequest),
        (body_for(""), CollabAvatarProxyError::KeyNotAllowed),
        (
            body_for("peer\u{0}one"),
            CollabAvatarProxyError::KeyNotAllowed,
        ),
        (
            body_for(&"k".repeat(MAX_PARTICIPANT_KEY_BYTES + 1)),
            CollabAvatarProxyError::KeyNotAllowed,
        ),
    ] {
        assert_eq!(resolve_avatar(&body, never), Err(expected), "{body:.40}");
    }
    assert_eq!(avatar("not json").status, "400 Bad Request");
}
