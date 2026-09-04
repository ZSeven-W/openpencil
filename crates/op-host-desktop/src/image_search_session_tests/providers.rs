//! Provider-side tests — Openverse/Wikimedia request shaping, result
//! ranking, image mime handling and the content claim. Split out of the
//! flat `image_search_session_tests.rs` to keep every file under the
//! 800-line cap; pure code motion.

use super::super::*;
use super::*;

#[test]
fn openverse_search_url_includes_aspect_ratio() {
    let url = openverse_search_url("burger fries", Some(ImageAspectRatio::Square))
        .expect("valid openverse url");

    assert_eq!(
        url.query_pairs()
            .find(|(key, _)| key == "aspect_ratio")
            .map(|(_, value)| value.into_owned()),
        Some("square".to_string())
    );
}

#[test]
fn openverse_search_url_simplifies_verbose_ai_prompt_like_ts() {
    let url = openverse_search_url("a beautiful photo of the sunset on the beach", None)
        .expect("valid openverse url");

    assert_eq!(
        url.query_pairs()
            .find(|(key, _)| key == "q")
            .map(|(_, value)| value.into_owned()),
        Some("beautiful photo sunset beach".to_string())
    );
}

#[test]
fn openverse_search_url_limits_simplified_query_to_four_keywords_like_ts() {
    let url = openverse_search_url(
        "modern office workspace natural lighting wooden desk plants",
        None,
    )
    .expect("valid openverse url");

    assert_eq!(
        url.query_pairs()
            .find(|(key, _)| key == "q")
            .map(|(_, value)| value.into_owned()),
        Some("modern office workspace natural".to_string())
    );
}

#[test]
fn openverse_credentials_require_both_fields() {
    let mut state = EditorState::default();
    assert!(OpenverseCredentials::from_state(&state).is_none());

    state.editor_ui.agent_settings.openverse_client_id = " client ".into();
    assert!(OpenverseCredentials::from_state(&state).is_none());

    state.editor_ui.agent_settings.openverse_client_secret = " secret ".into();
    let credentials = OpenverseCredentials::from_state(&state).expect("complete credentials");
    assert_eq!(credentials.as_web().client_id, "client");
    assert_eq!(credentials.as_web().client_secret, "secret");
}

#[test]
fn image_bytes_to_data_url_encodes_canvas_renderable_src() {
    let src = image_bytes_to_data_url("image/png; charset=binary", b"ABC")
        .expect("png bytes should encode");

    assert_eq!(src, "data:image/png;base64,QUJD");
}

#[test]
fn image_bytes_to_data_url_normalizes_jpg_mime_alias() {
    let src = image_bytes_to_data_url("image/jpg", b"ABC").expect("jpg alias should encode");

    assert_eq!(src, "data:image/jpeg;base64,QUJD");
}

#[test]
fn image_bytes_to_data_url_rejects_svg_payloads() {
    assert!(image_bytes_to_data_url("image/svg+xml", b"<svg></svg>").is_none());
}

#[test]
fn sniff_image_mime_detects_common_raster_formats() {
    assert_eq!(
        sniff_image_mime(b"\x89PNG\r\n\x1A\nrest"),
        Some("image/png")
    );
    assert_eq!(sniff_image_mime(b"\xFF\xD8\xFFrest"), Some("image/jpeg"));
    assert_eq!(sniff_image_mime(b"GIF89arest"), Some("image/gif"));
    assert_eq!(sniff_image_mime(b"RIFFxxxxWEBPrest"), Some("image/webp"));
    assert_eq!(sniff_image_mime(b"<svg></svg>"), None);
}

#[test]
fn openverse_selection_skips_junk_and_prefers_query_overlap() {
    use serde_json::json;
    let results = vec![
        json!({"title": "File Not Found", "url": "https://x/1.jpg"}),
        json!({"title": "Sunset over green hills", "url": "https://x/2.jpg"}),
        json!({"title": "Midnight city neon lights", "url": "https://x/3.jpg"}),
    ];
    let empty = std::collections::HashSet::new();
    let picked =
        select_openverse_result(&results, "midnight city neon", &empty).expect("a result survives");
    assert_eq!(
        picked["url"], "https://x/3.jpg",
        "query-overlapping title wins"
    );

    let all_junk = vec![
        json!({"title": "404 error page", "url": "https://x/1.jpg"}),
        json!({"title": "image not found placeholder", "url": "https://x/2.jpg"}),
    ];
    assert!(
        select_openverse_result(&all_junk, "midnight city", &empty).is_none(),
        "all-junk result sets leave the slot empty"
    );

    let no_overlap = vec![json!({"title": "Sunset over hills", "url": "https://x/9.jpg"})];
    let fallback =
        select_openverse_result(&no_overlap, "midnight city", &empty).expect("non-junk fallback");
    assert_eq!(fallback["url"], "https://x/9.jpg");

    // Session dedup: a URL already used by another card is skipped, so
    // near-identical queries stop filling every card with the same photo.
    let mut used = std::collections::HashSet::new();
    used.insert("https://x/3.jpg".to_string());
    let second =
        select_openverse_result(&results, "midnight city neon", &used).expect("a different result");
    assert_ne!(second["url"], "https://x/3.jpg", "used URL is skipped");
}

#[test]
fn openverse_selection_ranks_by_complete_token_overlap_stably() {
    use serde_json::json;
    let empty = std::collections::HashSet::new();

    let ranked = vec![
        json!({"title": "Limestone architecture", "url": "https://x/one.jpg"}),
        json!({"title": "Limestone lounge chair in a quiet studio", "url": "https://x/many.jpg"}),
    ];
    let picked = select_openverse_result(
        &ranked,
        "limestone lounge chair editorial furniture",
        &empty,
    )
    .expect("ranked result");
    assert_eq!(
        picked["url"], "https://x/many.jpg",
        "more complete query tokens beat an earlier one-token match"
    );

    let substring = vec![
        json!({"title": "Cathedral facade", "url": "https://x/substring.jpg"}),
        json!({"title": "Minimal lounge interior", "url": "https://x/exact.jpg"}),
    ];
    let picked = select_openverse_result(&substring, "cat lounge", &empty).expect("exact result");
    assert_eq!(
        picked["url"], "https://x/exact.jpg",
        "a query token inside a longer title word is not an overlap"
    );

    let tied = vec![
        json!({"title": "Oak lounge", "url": "https://x/first.jpg"}),
        json!({"title": "Lounge lighting", "url": "https://x/second.jpg"}),
    ];
    let picked =
        select_openverse_result(&tied, "lounge chair", &empty).expect("stable tied result");
    assert_eq!(
        picked["url"], "https://x/first.jpg",
        "provider order breaks equal-overlap ties"
    );
}

#[test]
fn openverse_selection_still_excludes_used_and_junk_before_ranking() {
    use serde_json::json;
    let results = vec![
        json!({"id": "junk", "title": "Limestone lounge chair placeholder", "url": "https://x/junk.jpg"}),
        json!({"id": "used", "title": "Limestone lounge chair studio", "url": "https://x/used.jpg"}),
        json!({"id": "available", "title": "Limestone chair", "url": "https://x/available.jpg"}),
    ];
    let used = std::collections::HashSet::from(["openverse:used".to_string()]);

    let picked = select_openverse_result(&results, "limestone lounge chair", &used)
        .expect("unused non-junk result");

    assert_eq!(picked["url"], "https://x/available.jpg");
}

#[test]
fn simplify_strips_design_artifact_words_but_never_to_empty() {
    // "synthwave album cover neon" → the corpus has no album covers, but it
    // has plenty of synthwave/neon photography.
    assert_eq!(
        simplify_search_query("synthwave album cover neon"),
        "synthwave neon"
    );
    assert_eq!(
        simplify_search_query("playlist cover daily mix"),
        "daily mix"
    );
    // All-artifact queries keep their words rather than going empty.
    assert_eq!(simplify_search_query("album cover"), "album cover");
    // Concrete-subject queries are untouched.
    assert_eq!(
        simplify_search_query("kyoto temple cherry blossom"),
        "kyoto temple cherry blossom"
    );
}

#[test]
fn image_result_claim_is_atomic_across_queries() {
    let used = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
    let winners = (0..8)
        .map(|_| {
            let used = std::sync::Arc::clone(&used);
            std::thread::spawn(move || claim_unused_image_src(&used, "data:image/png;base64,SAME"))
        })
        .map(|thread| thread.join().expect("claim thread"))
        .filter(|claimed| *claimed)
        .count();

    assert_eq!(winners, 1, "only one concurrent query may claim an image");
    assert!(
        used.lock()
            .unwrap()
            .iter()
            .all(|key| key.starts_with("content:") && !key.contains("base64")),
        "dedup stores compact digests, not multi-megabyte data URIs"
    );
}

#[test]
fn provider_identity_reservation_releases_only_unavailable_downloads() {
    let used = std::sync::Mutex::new(std::collections::HashSet::from([
        "openverse:unavailable".to_string(),
        "openverse:claimed".to_string(),
        "openverse:duplicate".to_string(),
    ]));

    assert_eq!(
        settle_provider_identity(
            &used,
            "openverse:unavailable",
            ImageCandidateClaim::Unavailable,
        ),
        None
    );
    assert_eq!(
        settle_provider_identity(
            &used,
            "openverse:claimed",
            ImageCandidateClaim::Claimed("data:image/png;base64,OK".into()),
        ),
        Some("data:image/png;base64,OK".into())
    );
    assert_eq!(
        settle_provider_identity(&used, "openverse:duplicate", ImageCandidateClaim::Duplicate,),
        None
    );

    let used = used.lock().unwrap();
    assert!(!used.contains("openverse:unavailable"));
    assert!(used.contains("openverse:claimed"));
    assert!(
        used.contains("openverse:duplicate"),
        "a successfully downloaded duplicate remains excluded"
    );
}

#[test]
fn wikimedia_missing_or_empty_imageinfo_releases_the_page_reservation() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .build()
        .expect("build test rustls client");
    for page in [
        serde_json::json!({"pageid": 41, "title": "Missing imageinfo"}),
        serde_json::json!({"pageid": 42, "title": "Empty imageinfo", "imageinfo": []}),
    ] {
        let identity = wikimedia_page_identity(&page).expect("page identity");
        let used = std::sync::Mutex::new(std::collections::HashSet::from([identity.clone()]));
        let candidates = wikimedia_image_candidates(&page);
        assert!(candidates.is_empty());

        let outcome = runtime.block_on(first_unused_renderable_image_src(
            &client, candidates, &used,
        ));
        assert_eq!(outcome, ImageCandidateClaim::Unavailable);
        assert_eq!(settle_provider_identity(&used, &identity, outcome), None);
        assert!(
            !used.lock().unwrap().contains(&identity),
            "an unusable page must remain retryable"
        );
    }
}

#[test]
fn openverse_claim_uses_artwork_identity_not_thumbnail_variant() {
    let used = std::sync::Mutex::new(std::collections::HashSet::new());
    let first = vec![serde_json::json!({
        "id":"art-42", "title":"Kyoto temple", "thumbnail":"https://x/thumb-400.jpg",
        "url":"https://x/full.jpg"
    })];
    let resized = vec![serde_json::json!({
        "id":"art-42", "title":"Kyoto temple", "thumbnail":"https://x/thumb-800.jpg",
        "url":"https://x/full.jpg"
    })];

    assert!(claim_openverse_result(&first, "Kyoto temple", &used).is_some());
    assert!(
        claim_openverse_result(&resized, "Kyoto temple", &used).is_none(),
        "one artwork id owns all thumbnail/full-size URL variants"
    );
}
