use super::*;

fn png_header(width: u32, height: u32, payload_bytes: usize) -> Vec<u8> {
    let mut bytes = vec![0; payload_bytes.max(24)];
    bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
    bytes[8..12].copy_from_slice(&13_u32.to_be_bytes());
    bytes[12..16].copy_from_slice(b"IHDR");
    bytes[16..20].copy_from_slice(&width.to_be_bytes());
    bytes[20..24].copy_from_slice(&height.to_be_bytes());
    bytes
}

fn fetch(registry: &mut AvatarRegistry, key: &str, url: &str) -> CollabAvatarFetchRequest {
    assert!(registry.lookup(key, url).is_none());
    registry.take_requests(1).pop().expect("request queued")
}

#[test]
fn success_is_cached_and_failures_keep_the_fallback() {
    let mut registry = AvatarRegistry::with_limits(1_024, 4, 4);
    let ok = fetch(&mut registry, "p1", "https://cdn.example/a.png");
    assert!(registry.complete(&ok, Some(png_header(16, 16, 48))));
    let image = registry
        .lookup("p1", "https://cdn.example/a.png")
        .expect("valid response is ready");
    assert_eq!(image.image_id, ok.image_id);

    let failed = fetch(&mut registry, "p2", "https://cdn.example/b.png");
    assert!(registry.complete(&failed, None));
    assert!(registry.lookup("p2", "https://cdn.example/b.png").is_none());
    assert!(registry.take_requests(1).is_empty());
}

#[test]
fn encoded_bytes_dimensions_and_queue_are_bounded() {
    let mut registry = AvatarRegistry::with_limits(2_048, 64, 1);
    let large_body = fetch(&mut registry, "p1", "https://cdn.example/large.png");
    assert!(registry.complete(&large_body, Some(vec![0; MAX_AVATAR_ENCODED_BYTES + 1])));
    assert!(registry
        .lookup("p1", "https://cdn.example/large.png")
        .is_none());

    let large_raster = fetch(&mut registry, "p2", "https://cdn.example/wide.png");
    assert!(registry.complete(
        &large_raster,
        Some(png_header(MAX_AVATAR_SOURCE_EDGE_PX + 1, 1, 24))
    ));
    assert!(registry
        .lookup("p2", "https://cdn.example/wide.png")
        .is_none());

    assert!(registry.lookup("p3", "https://cdn.example/c.png").is_none());
    assert!(registry.lookup("p4", "https://cdn.example/d.png").is_none());
    assert_eq!(registry.take_requests(8).len(), 1);
}

#[test]
fn lru_eviction_and_generation_switch_drop_stale_results() {
    let mut registry = AvatarRegistry::with_limits(80, 2, 4);
    let first = fetch(&mut registry, "p1", "https://cdn.example/one.png");
    assert!(registry.complete(&first, Some(png_header(2, 2, 40))));
    let second = fetch(&mut registry, "p2", "https://cdn.example/two.png");
    assert!(registry.complete(&second, Some(png_header(2, 2, 40))));
    assert!(registry
        .lookup("p1", "https://cdn.example/one.png")
        .is_some());
    let third = fetch(&mut registry, "p3", "https://cdn.example/three.png");
    assert!(registry.complete(&third, Some(png_header(2, 2, 40))));
    assert!(
        !registry.slots.contains_key("p2"),
        "least recently used entry is evicted"
    );

    let old = fetch(&mut registry, "p4", "https://cdn.example/old.png");
    assert!(registry
        .lookup("p4", "https://cdn.example/new.png")
        .is_none());
    let new = registry.take_requests(1).pop().expect("new generation");
    assert!(!registry.complete(&old, Some(png_header(2, 2, 32))));
    assert!(registry
        .lookup("p4", "https://cdn.example/new.png")
        .is_none());
    assert!(registry.complete(&new, Some(png_header(2, 2, 32))));
    assert_eq!(
        registry
            .lookup("p4", "https://cdn.example/new.png")
            .expect("new generation wins")
            .image_id,
        new.image_id
    );
}

#[test]
fn session_generation_rotation_clears_cache_and_rejects_late_workers() {
    let mut registry = AvatarRegistry::with_limits(1_024, 4, 4);
    assert!(registry.begin_session_generation(7));
    let stale = fetch(
        &mut registry,
        "same-participant",
        "https://cdn.example/same.png",
    );
    assert!(registry.begin_session_generation(8));
    assert!(registry.slots.is_empty());
    assert!(registry.pending.is_empty());
    assert!(!registry.complete(&stale, Some(png_header(16, 16, 32))));

    let current = fetch(
        &mut registry,
        "same-participant",
        "https://cdn.example/same.png",
    );
    assert_ne!(stale.image_id, current.image_id);
    assert!(
        registry.begin_session_generation(8),
        "a new runtime may reuse the numeric generation and must still reset"
    );
    assert!(!registry.complete(&current, Some(png_header(16, 16, 32))));
    let refreshed = fetch(
        &mut registry,
        "same-participant",
        "https://cdn.example/same.png",
    );
    assert_ne!(current.image_id, refreshed.image_id);
    assert!(registry.complete(&refreshed, Some(png_header(16, 16, 32))));
    assert!(registry
        .lookup("same-participant", "https://cdn.example/same.png")
        .is_some());
}

#[test]
fn collaboration_generation_preserves_and_requeues_the_account_slot() {
    let mut registry = AvatarRegistry::with_limits(1_024, 4, 4);
    let account = fetch(
        &mut registry,
        ACCOUNT_AVATAR_KEY,
        "https://cdn.example/account.png",
    );
    assert!(registry.begin_session_generation(7));
    assert!(!registry.complete(&account, Some(png_header(16, 16, 32))));
    let retried = registry
        .take_requests(1)
        .pop()
        .expect("account request requeued");
    assert_eq!(retried.image_id, account.image_id);
    assert!(registry.complete(&retried, Some(png_header(16, 16, 32))));

    let collab = fetch(
        &mut registry,
        "collab:participant",
        "https://cdn.example/participant.png",
    );
    assert!(registry.complete(&collab, Some(png_header(16, 16, 32))));
    assert!(registry.begin_session_generation(8));
    assert!(registry.ready(ACCOUNT_AVATAR_KEY).is_some());
    assert!(!registry.slots.contains_key("collab:participant"));
}

#[test]
fn failed_account_fetch_retries_with_backoff_and_keeps_the_fallback() {
    let mut registry = AvatarRegistry::with_limits(1_024, 4, 4);
    let failed = fetch(
        &mut registry,
        ACCOUNT_AVATAR_KEY,
        "https://cdn.example/account.png",
    );
    assert!(registry.complete(&failed, None));
    assert!(registry.ready(ACCOUNT_AVATAR_KEY).is_none());
    assert!(registry.has_background_work());
    assert!(registry.pending.is_empty());

    registry
        .slots
        .get_mut(ACCOUNT_AVATAR_KEY)
        .expect("account slot")
        .retry_at = Some(Instant::now());
    assert!(registry.ready(ACCOUNT_AVATAR_KEY).is_none());
    let retried = registry
        .take_requests(1)
        .pop()
        .expect("due account retry queued");
    assert_eq!(retried.image_id, failed.image_id);
    assert!(registry.complete(&retried, Some(png_header(16, 16, 32))));
    assert!(registry.ready(ACCOUNT_AVATAR_KEY).is_some());
}

#[test]
fn account_and_adversarial_participant_keys_cannot_collide() {
    let _guard = lock_collab_avatar_registry_for_tests();
    assert!(register_account_avatar_url(Some(
        "https://cdn.example/account.png"
    )));
    assert!(register_collab_avatar_url(
        ACCOUNT_AVATAR_KEY,
        Some("https://cdn.example/participant.png")
    ));
    let requests = take_collab_avatar_requests(2);
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.is_current_account())
            .count(),
        1
    );
    for request in &requests {
        assert!(complete_collab_avatar_request(
            request,
            Some(png_header(16, 16, 32))
        ));
    }
    let account = account_avatar_image().expect("account image");
    let participant = collab_avatar_image(ACCOUNT_AVATAR_KEY).expect("participant image");
    assert_ne!(account.image_id, participant.image_id);
}

#[test]
fn proxy_install_replaces_only_on_a_valid_new_revision() {
    let _guard = lock_collab_avatar_registry_for_tests();
    assert!(install_account_avatar_bytes(
        "revision-one",
        png_header(16, 16, 32)
    ));
    let first = account_avatar_image().expect("first proxy image");
    assert!(!install_account_avatar_bytes(
        "../unsafe",
        png_header(16, 16, 32)
    ));
    assert_eq!(
        account_avatar_image()
            .expect("invalid revision keeps current")
            .image_id,
        first.image_id
    );
    assert!(install_account_avatar_bytes(
        "revision-two",
        png_header(16, 16, 32)
    ));
    assert_ne!(
        account_avatar_image().expect("new proxy image").image_id,
        first.image_id
    );
}

#[test]
fn request_debug_redacts_url_and_identity() {
    let mut registry = AvatarRegistry::with_limits(100, 1, 1);
    let request = fetch(
        &mut registry,
        "private-participant",
        "https://cdn.example/secret.png",
    );
    let debug = format!("{request:?}");
    assert!(!debug.contains("private-participant"));
    assert!(!debug.contains("cdn.example"));
    assert!(debug.contains("[REDACTED]"));
}
