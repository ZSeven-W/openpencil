use super::*;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn simplify_search_query_mirrors_the_desktop_adapter() {
    assert_eq!(
        simplify_search_query("A beautiful sunset over the mountains"),
        "beautiful sunset over mountains"
    );
    // Artifact words drop only when aesthetic words remain.
    assert_eq!(
        simplify_search_query("synthwave album cover neon"),
        "synthwave neon"
    );
    assert_eq!(simplify_search_query("logo"), "logo");
    // Empty keyword set falls back to a 30-char prefix.
    assert_eq!(simplify_search_query("の"), "の");
}

#[test]
fn zero_result_retry_keeps_the_core_product_phrase() {
    assert_eq!(
        simplify_search_query("minimalist ceramic vase beige"),
        "minimalist ceramic vase beige",
        "descriptor filtering belongs only to the zero-result retry"
    );
    assert_eq!(
        two_keyword_retry("minimalist ceramic vase beige"),
        Some("ceramic vase".to_string())
    );
    assert_eq!(
        two_keyword_retry("linen cushion neutral tone"),
        Some("linen cushion".to_string())
    );
    assert_eq!(
        two_keyword_retry("oak desk lamp"),
        None,
        "an already-concrete query must not repeat the same request"
    );
    assert_eq!(
        two_keyword_retry("minimal knitted wool sweater"),
        Some("knitted wool sweater".to_string()),
        "the retry must keep the garment subject instead of wasting a slot on staging prose"
    );
    assert_eq!(
        two_keyword_retry("sculptural arc table lamp terracotta"),
        Some("table lamp terracotta".to_string()),
        "the retry must keep a trailing product subject instead of truncating it"
    );
    assert_eq!(
        two_keyword_retry("minimal sneakers product photography studio"),
        Some("sneakers".to_string()),
        "one concrete product noun is a stronger retry than giving up on descriptor-only noise"
    );
    assert_eq!(
        two_keyword_retry("armchair isolated"),
        Some("armchair".to_string()),
        "two-word photo prompts still need a concrete recovery query"
    );
    assert_eq!(
        core_query_words("wooden lamps studio photo"),
        ["wood", "lamp"],
        "wooden is product evidence, while simple plurals canonicalize"
    );
    assert_eq!(
        core_query_words("knitting cardigans isolated"),
        ["knit", "cardigan"]
    );
}

#[test]
fn parse_search_request_reads_query_and_prefers_request_credentials() {
    let mut state = op_editor_core::EditorState::default();
    state.editor_ui.agent_settings.openverse_client_id = "persisted-id".into();
    state.editor_ui.agent_settings.openverse_client_secret = "persisted-secret".into();
    let (query, cred) = parse_search_request(
        r#"{"query":"cat","openverse":{"client_id":"req-id","client_secret":"req-secret"}}"#,
        &state,
    )
    .expect("parses");
    assert_eq!(query, "cat");
    assert_eq!(cred.expect("cred").client_id, "req-id");
    // No request credential → daemon-persisted fallback.
    let (_, cred) = parse_search_request(r#"{"query":"cat"}"#, &state).expect("parses");
    assert_eq!(cred.expect("cred").client_id, "persisted-id");
    // Neither → anonymous.
    let empty = op_editor_core::EditorState::default();
    let (_, cred) = parse_search_request(r#"{"query":"cat"}"#, &empty).expect("parses");
    assert!(cred.is_none());
}

#[test]
fn parse_search_request_rejects_bad_bodies() {
    let state = op_editor_core::EditorState::default();
    assert!(parse_search_request("", &state).is_err());
    assert!(parse_search_request("{}", &state).is_err());
    assert!(parse_search_request(r#"{"query":"  "}"#, &state).is_err());
}

#[test]
fn parse_openverse_results_maps_thumbnail_license_and_candidate_cap() {
    let mut results = vec![
        serde_json::json!({"id": "a", "thumbnail": "https://x/a.jpg", "attribution": "By A"}),
        serde_json::json!({"id": "b", "url": "https://x/b.jpg", "license": "cc0", "license_version": "1.0"}),
        serde_json::json!({"id": "missing-thumbnail"}),
    ];
    results.extend((0..(SEARCH_CANDIDATE_COUNT + 3)).map(|index| {
        serde_json::json!({
            "id": format!("candidate-{index}"),
            "thumbnail": format!("https://x/candidate-{index}.jpg")
        })
    }));
    let json = serde_json::json!({"results": results});
    let hits = parse_openverse_results(&json);
    assert_eq!(hits.len(), SEARCH_CANDIDATE_COUNT);
    assert_eq!(hits[0].id, "a");
    assert_eq!(hits[0].attribution, "By A");
    assert_eq!(hits[1].thumb_url, "https://x/b.jpg");
    assert_eq!(hits[1].attribution, "cc0 1.0");
}

#[test]
fn parse_wikimedia_results_maps_thumburl_and_license() {
    let json = serde_json::json!({
        "query": {"pages": {
            "1": {"pageid": 1, "title": "File:Forest armchair.jpg", "imageinfo": [{
                "thumburl": "https://c/w1.jpg",
                "extmetadata": {"LicenseShortName": {"value": "CC BY-SA 4.0"}}
            }]},
            "2": {"pageid": 2, "title": "File:Mountain lake.jpg", "imageinfo": [{"url": "https://c/w2.jpg"}]},
            "3": {"pageid": 3}
        }}
    });
    let mut hits = parse_wikimedia_results(&json);
    hits.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].thumb_url, "https://c/w1.jpg");
    assert_eq!(hits[0].attribution, "CC BY-SA 4.0");
    assert_eq!(hits[0].relevance_metadata, "File:Forest armchair.jpg");
    assert_eq!(hits[1].thumb_url, "https://c/w2.jpg");
}

#[test]
fn parse_wikimedia_results_keeps_a_twenty_hit_candidate_pool() {
    let mut pages = serde_json::Map::new();
    for index in 0..(SEARCH_CANDIDATE_COUNT + 4) {
        pages.insert(
            index.to_string(),
            serde_json::json!({
                "pageid": index,
                "title": format!("File:Ceramic vase {index}.jpg"),
                "imageinfo": [{"thumburl": format!("https://x/vase-{index}.jpg")}]
            }),
        );
    }
    let json = serde_json::json!({"query": {"pages": pages}});

    assert_eq!(parse_wikimedia_results(&json).len(), SEARCH_CANDIDATE_COUNT);
}

#[test]
fn parse_wikimedia_results_rejects_pdf_page_thumbnails() {
    let json = serde_json::json!({
        "query": {"pages": {
            "1": {
                "pageid": 1,
                "title": "File:Technical Wool Conference (IA report).pdf",
                "imageinfo": [{
                    "mime": "application/pdf",
                    "thumburl": "https://upload.wikimedia.org/report.pdf/page1-report.pdf.jpg"
                }]
            },
            "2": {
                "pageid": 2,
                "title": "File:Knitted wool sweater.jpg",
                "imageinfo": [{
                    "mime": "image/jpeg",
                    "thumburl": "https://upload.wikimedia.org/sweater.jpg"
                }]
            },
            "3": {
                "pageid": 3,
                "title": "File:Historic wool bulletin.pdf",
                "imageinfo": [{
                    "thumburl": "https://upload.wikimedia.org/bulletin.pdf/page1.jpg"
                }]
            }
        }}
    });

    let hits = parse_wikimedia_results(&json);

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "2");
    assert_eq!(
        hits[0].thumb_url,
        "https://upload.wikimedia.org/sweater.jpg"
    );

    let pdf_page = &json["query"]["pages"]["1"];
    assert!(
        crate::net::fetch::wikimedia_image_candidates(pdf_page).is_empty(),
        "the shared desktop provider must reject the same PDF thumbnail"
    );
}

#[test]
fn search_outcome_json_shape() {
    let outcome = WebImageSearchOutcome {
        results: vec![WebImageSearchHit {
            id: "a".into(),
            thumb_data_url: "data:image/png;base64,AA==".into(),
            attribution: "By A".into(),
        }],
        source: Some("openverse"),
    };
    let json: serde_json::Value =
        serde_json::from_str(&search_outcome_to_json(&outcome)).expect("valid json");
    assert_eq!(json["ok"], true);
    assert_eq!(json["source"], "openverse");
    assert_eq!(json["results"][0]["id"], "a");
    assert_eq!(
        json["results"][0]["thumb_data_url"],
        "data:image/png;base64,AA=="
    );
    let empty = WebImageSearchOutcome {
        results: Vec::new(),
        source: None,
    };
    let json: serde_json::Value =
        serde_json::from_str(&search_outcome_to_json(&empty)).expect("valid json");
    assert!(json["source"].is_null());
}

#[test]
fn image_job_slot_caps_concurrency_and_releases_on_drop() {
    let held: Vec<_> = (0..MAX_IN_FLIGHT_IMAGE_JOBS)
        .map(|_| ImageJobSlot::acquire().expect("slot under the cap"))
        .collect();
    assert!(
        ImageJobSlot::acquire().is_none(),
        "cap reached — acquire must fail"
    );
    drop(held);
    assert!(
        ImageJobSlot::acquire().is_some(),
        "drop must release the slots"
    );
}

#[test]
fn sniff_image_mime_recognizes_the_embeddable_formats() {
    assert_eq!(sniff_image_mime(b"\x89PNG\r\n\x1A\nxx"), Some("image/png"));
    assert_eq!(sniff_image_mime(b"\xFF\xD8\xFFxx"), Some("image/jpeg"));
    assert_eq!(sniff_image_mime(b"GIF89a"), Some("image/gif"));
    assert_eq!(
        sniff_image_mime(b"RIFF\0\0\0\0WEBPVP8 "),
        Some("image/webp")
    );
    assert_eq!(sniff_image_mime(b"<svg>"), None);
    assert_eq!(
        normalize_image_mime_header("image/jpg"),
        Some("image/jpeg".into())
    );
    assert_eq!(normalize_image_mime_header("image/svg+xml"), None);
    assert_eq!(normalize_image_mime_header("text/html"), None);
}

#[test]
fn provider_timeout_helper_cancels_the_whole_slow_future() {
    let result = run_with_timeout(Duration::from_millis(20), async {
        tokio::time::sleep(Duration::from_millis(250)).await;
        7_u8
    });

    assert_eq!(
        result, None,
        "the provider ladder must not outlive its budget"
    );
    assert_eq!(
        run_with_timeout(Duration::ZERO, async { 9_u8 }),
        None,
        "an exhausted overall deadline must not start the ladder"
    );
}

#[test]
fn first_thumbnail_materializer_stops_after_first_success() {
    let calls = Arc::new(AtomicUsize::new(0));
    let fetch_calls = calls.clone();
    let fetcher = move |url: String| {
        let fetch_calls = fetch_calls.clone();
        async move {
            let call = fetch_calls.fetch_add(1, Ordering::SeqCst);
            (call == 1).then(|| format!("data:image/png;base64,{url}"))
        }
    };
    let hits = ["first", "second", "third"]
        .into_iter()
        .map(|id| RawHit {
            id: id.to_string(),
            thumb_url: id.to_string(),
            attribution: format!("by {id}"),
            title: id.to_string(),
            relevance_metadata: id.to_string(),
        })
        .collect();

    let hit = crate::net::block_on_image_runtime(materialize_first_thumb(hits, &fetcher))
        .expect("the second thumbnail succeeds");

    assert_eq!(hit.id, "second");
    assert_eq!(hit.thumb_data_url, "data:image/png;base64,second");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "third hit must not download"
    );
}

#[test]
fn multi_thumbnail_materializer_caps_public_results_at_five() {
    let calls = Arc::new(AtomicUsize::new(0));
    let fetch_calls = calls.clone();
    let fetcher = move |url: String| {
        let fetch_calls = fetch_calls.clone();
        async move {
            fetch_calls.fetch_add(1, Ordering::SeqCst);
            Some(format!("data:image/png;base64,{url}"))
        }
    };
    let hits = (0..SEARCH_CANDIDATE_COUNT)
        .map(|index| RawHit {
            id: index.to_string(),
            thumb_url: index.to_string(),
            attribution: String::new(),
            title: format!("Product {index}"),
            relevance_metadata: format!("Product {index}"),
        })
        .collect();

    let results = crate::net::block_on_image_runtime(materialize_thumbs(hits, &fetcher));

    assert_eq!(results.len(), SEARCH_RESULT_COUNT);
    assert_eq!(calls.load(Ordering::SeqCst), SEARCH_RESULT_COUNT);
}

#[test]
fn renderable_image_data_url_is_restricted_to_renderer_codecs() {
    use crate::net::fetch::renderable_image_data_url;
    use skia_safe::{surfaces, EncodedImageFormat};
    let mut surface = surfaces::raster_n32_premul((8, 8)).expect("raster surface");
    surface.canvas().clear(skia_safe::Color::BLUE);
    let snapshot = surface.image_snapshot();
    let png = snapshot
        .encode(None, EncodedImageFormat::PNG, 100)
        .expect("encode png")
        .as_bytes()
        .to_vec();

    // PNG within budget embeds untouched.
    let url = renderable_image_data_url("image/png", &png).expect("png embeds");
    assert!(url.starts_with("data:image/png;base64,"));

    // A WebP payload must never survive as image/webp — it is either
    // transcoded to a renderer codec or rejected so the caller can try
    // the next candidate URL. (The encoder may be absent from this skia
    // build; the invariant holds either way.)
    if let Some(webp) = snapshot.encode(None, EncodedImageFormat::WEBP, 100) {
        if let Some(url) = renderable_image_data_url("image/webp", webp.as_bytes()) {
            assert!(
                url.starts_with("data:image/png;base64,")
                    || url.starts_with("data:image/jpeg;base64,"),
                "webp transcodes to a renderer codec, got {}",
                &url[..40.min(url.len())]
            );
        }
    }
    assert!(
        renderable_image_data_url("image/webp", b"RIFF\0\0\0\0WEBPVP8 junk").is_none(),
        "undecodable webp is rejected"
    );
    assert!(
        renderable_image_data_url("image/gif", b"GIF89a junk").is_none(),
        "gif is rejected rather than flattened"
    );
}
