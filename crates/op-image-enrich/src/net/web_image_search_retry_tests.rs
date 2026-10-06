//! Provider retry-ladder tests (`fetch_relevant_*_list_with`) plus the
//! remaining relevance fallthrough fences. Split out of
//! `web_image_search_tests.rs` at the 800-line cap; pure code motion.

use super::*;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn isolated_retry_preserves_the_original_isolation_contract() {
    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let fetch_calls = calls.clone();
    let fetcher = move |query: String| {
        fetch_calls.lock().expect("calls lock").push(query.clone());
        async move {
            match query.as_str() {
                "modern sofa isolated photo" => Some(Vec::new()),
                "sofa" => Some(vec![
                    RawHit {
                        id: "bare-studio".to_string(),
                        thumb_url: "https://x/bare-studio.jpg".to_string(),
                        attribution: String::new(),
                        title: "Modern sofa studio photograph".to_string(),
                        relevance_metadata: "Modern sofa studio photograph".to_string(),
                    },
                    RawHit {
                        id: "cutout".to_string(),
                        thumb_url: "https://x/cutout.jpg".to_string(),
                        attribution: String::new(),
                        title: "Modern sofa".to_string(),
                        relevance_metadata: "Modern sofa cut out on white background".to_string(),
                    },
                ]),
                unexpected => panic!("unexpected Openverse query: {unexpected}"),
            }
        }
    };

    let hits = crate::net::block_on_image_runtime(fetch_relevant_openverse_list_with(
        "modern sofa isolated photo",
        fetcher,
    ))
    .expect("provider requests succeed");

    assert_eq!(
        calls.lock().expect("calls lock").as_slice(),
        ["modern sofa isolated photo", "sofa"]
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "cutout");
}

#[test]
fn relevance_filtered_openverse_results_retry_the_concrete_subject_once() {
    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let fetch_calls = calls.clone();
    let fetcher = move |query: String| {
        fetch_calls.lock().expect("calls lock").push(query.clone());
        async move {
            match query.as_str() {
                "wool sweater studio photo" => Some(vec![RawHit {
                    id: "catalog".to_string(),
                    thumb_url: "https://x/catalog.jpg".to_string(),
                    attribution: String::new(),
                    title: "Knitted wool sweater catalogue illustration".to_string(),
                    relevance_metadata: "Knitted wool sweater catalogue illustration".to_string(),
                }]),
                "wool sweater" => Some(vec![
                    RawHit {
                        id: "holiday".to_string(),
                        thumb_url: "https://x/holiday.jpg".to_string(),
                        attribution: String::new(),
                        title: "Christmas holiday lights".to_string(),
                        relevance_metadata: "Christmas holiday lights".to_string(),
                    },
                    RawHit {
                        id: "sweater".to_string(),
                        thumb_url: "https://x/sweater.jpg".to_string(),
                        attribution: String::new(),
                        title: "Knitted wool sweater product photo".to_string(),
                        relevance_metadata: "Knitted wool sweater product photo".to_string(),
                    },
                ]),
                unexpected => panic!("unexpected Openverse query: {unexpected}"),
            }
        }
    };

    let hits = crate::net::block_on_image_runtime(fetch_relevant_openverse_list_with(
        "wool sweater studio photo",
        fetcher,
    ))
    .expect("provider requests succeed");

    assert_eq!(
        calls.lock().expect("calls lock").as_slice(),
        ["wool sweater studio photo", "wool sweater"],
        "a filtered non-empty reply gets exactly one concrete retry"
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "sweater");
}

#[test]
fn photo_primary_rejects_tag_only_armchair_then_retries_real_title() {
    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let fetch_calls = calls.clone();
    let fetcher = move |query: String| {
        fetch_calls.lock().expect("calls lock").push(query.clone());
        async move {
            let json = match query.as_str() {
                "armchair studio photo" => serde_json::json!({"results": [{
                    "id": "table",
                    "thumbnail": "https://x/table.jpg",
                    "title": "Earth-Stripe Table",
                    "tags": [
                        {"name": "coffeetable"},
                        {"name": "woodentable"},
                        {"name": "armchair", "accuracy": 0.94696},
                        {"name": "table", "accuracy": 0.98991}
                    ]
                }]}),
                "armchair" => serde_json::json!({"results": [{
                    "id": "exact",
                    "thumbnail": "https://x/armchair.jpg",
                    "title": "The armchair",
                    "tags": [{"name": "armchair", "accuracy": null}]
                }]}),
                unexpected => panic!("unexpected Openverse query: {unexpected}"),
            };
            Some(parse_openverse_results(&json))
        }
    };

    let hits = crate::net::block_on_image_runtime(fetch_relevant_openverse_list_with(
        "armchair studio photo",
        fetcher,
    ))
    .expect("provider requests succeed");

    assert_eq!(
        calls.lock().expect("calls lock").as_slice(),
        ["armchair studio photo", "armchair"]
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "exact");
    assert_eq!(hits[0].title, "The armchair");
}

#[test]
fn cardigan_retry_canonicalizes_knit_and_plural_but_rejects_doll() {
    let fetcher = |query: String| async move {
        let json = match query.as_str() {
            "knitted cardigan studio photo" => serde_json::json!({"results": [{
                "id": "tag-only",
                "thumbnail": "https://x/tag-only.jpg",
                "title": "a better photo of miss henry",
                "tags": ["cardigan", "knit", "knitting", "studio", "sweater", "wool"]
            }]}),
            "knitted cardigan" => serde_json::json!({"results": [
                {
                    "id": "doll",
                    "thumbnail": "https://x/doll.jpg",
                    "title": "My first hand knitted cardigans for 11 cm Obitsu doll body",
                    "tags": ["cardigan", "doll", "dolls", "dollfashion", "handknitted"]
                },
                {
                    "id": "garment",
                    "thumbnail": "https://x/garment.jpg",
                    "title": "#BabyGap NWT knit cardigan. 18-24M. $8 plus ship.",
                    "tags": ["instagramapp"]
                }
            ]}),
            unexpected => panic!("unexpected Openverse query: {unexpected}"),
        };
        Some(parse_openverse_results(&json))
    };

    let hits = crate::net::block_on_image_runtime(fetch_relevant_openverse_list_with(
        "knitted cardigan studio photo",
        fetcher,
    ))
    .expect("provider requests succeed");

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "garment");
}

#[test]
fn wooden_lamp_retry_prefers_concise_product_title() {
    let fetcher = |query: String| async move {
        let json = match query.as_str() {
            "wooden lamp studio photo" => serde_json::json!({"results": [{
                "id": "toy",
                "thumbnail": "https://x/toy.jpg",
                "title": "Only on Bonsoni!",
                "tags": ["lamp", "figurine", "plush", "teddy", "toy", "wool"]
            }]}),
            "wooden lamp" => serde_json::json!({"results": [
                {
                    "id": "framed",
                    "thumbnail": "https://x/framed.jpg",
                    "title": "Zardozi embroidery artwork framed on wooden lamp"
                },
                {
                    "id": "exact",
                    "thumbnail": "https://x/lamp.jpg",
                    "title": "The Wooden Lamp",
                    "tags": ["lamp", "light", "wood"]
                },
                {
                    "id": "rope",
                    "thumbnail": "https://x/rope.jpg",
                    "title": "Rope with Wooden Lamp over a dining table"
                }
            ]}),
            unexpected => panic!("unexpected Openverse query: {unexpected}"),
        };
        Some(parse_openverse_results(&json))
    };

    let hits = crate::net::block_on_image_runtime(fetch_relevant_openverse_list_with(
        "wooden lamp studio photo",
        fetcher,
    ))
    .expect("provider requests succeed");
    let ids: Vec<&str> = hits.iter().map(|hit| hit.id.as_str()).collect();

    assert_eq!(
        two_keyword_retry("wooden lamp studio photo").as_deref(),
        Some("wooden lamp")
    );
    assert_eq!(ids.first(), Some(&"exact"));
    assert!(ids.contains(&"framed"));
    assert!(
        !ids.contains(&"rope"),
        "the original studio-photo intent must reject dining-room retry noise"
    );
}

#[test]
fn explicit_toy_query_keeps_toy_results_but_product_query_rejects_them() {
    let toy = RawHit {
        id: "toy".to_string(),
        thumb_url: "https://x/toy.jpg".to_string(),
        attribution: String::new(),
        title: "Handmade plush toy".to_string(),
        relevance_metadata: "Handmade plush teddy toy stuffed doll".to_string(),
    };

    assert_eq!(
        retain_relevant_hits(vec![toy], "plush toy studio photo").len(),
        1,
        "an explicit toy request opts into the toy result group"
    );

    let toy = RawHit {
        id: "toy".to_string(),
        thumb_url: "https://x/toy.jpg".to_string(),
        attribution: String::new(),
        title: "Wooden lamp toy".to_string(),
        relevance_metadata: "Wooden lamp plush teddy toy".to_string(),
    };
    assert!(retain_relevant_hits(vec![toy], "wooden lamp").is_empty());
}

#[test]
fn wikimedia_filtered_primary_retries_concrete_subject() {
    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let fetch_calls = calls.clone();
    let fetcher = move |query: String| {
        fetch_calls.lock().expect("calls lock").push(query.clone());
        async move {
            match query.as_str() {
                "wooden lamp studio photo" => vec![RawHit {
                    id: "montage".to_string(),
                    thumb_url: "https://x/montage.jpg".to_string(),
                    attribution: String::new(),
                    title: "Wooden lamp montage".to_string(),
                    relevance_metadata: "Wooden lamp montage collage".to_string(),
                }],
                "wooden lamp" => vec![
                    RawHit {
                        id: "dining-room".to_string(),
                        thumb_url: "https://x/dining-room.jpg".to_string(),
                        attribution: String::new(),
                        title: "Wooden lamp over a dining table".to_string(),
                        relevance_metadata: "Wooden lamp in dining room interior".to_string(),
                    },
                    RawHit {
                        id: "lamp".to_string(),
                        thumb_url: "https://x/lamp.jpg".to_string(),
                        attribution: String::new(),
                        title: "The Wooden Lamp".to_string(),
                        relevance_metadata: "The Wooden Lamp".to_string(),
                    },
                ],
                unexpected => panic!("unexpected Wikimedia query: {unexpected}"),
            }
        }
    };

    let hits = crate::net::block_on_image_runtime(fetch_relevant_wikimedia_list_with(
        "wooden lamp studio photo",
        fetcher,
    ));

    assert_eq!(
        calls.lock().expect("calls lock").as_slice(),
        ["wooden lamp studio photo", "wooden lamp"]
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "lamp");
}

#[test]
fn zero_openverse_results_use_the_same_single_retry_path() {
    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let fetch_calls = calls.clone();
    let fetcher = move |query: String| {
        fetch_calls.lock().expect("calls lock").push(query.clone());
        async move {
            match query.as_str() {
                "ceramic vase studio photo" => Some(Vec::new()),
                "ceramic vase" => Some(vec![
                    RawHit {
                        id: "hotel".to_string(),
                        thumb_url: "https://x/hotel.jpg".to_string(),
                        attribution: String::new(),
                        title: "Ceramic vase in a hotel lounge".to_string(),
                        relevance_metadata: "Ceramic vase in a hotel lounge interior".to_string(),
                    },
                    RawHit {
                        id: "catalog".to_string(),
                        thumb_url: "https://x/catalog.jpg".to_string(),
                        attribution: String::new(),
                        title: "Ceramic vase catalogue illustration".to_string(),
                        relevance_metadata: "Ceramic vase catalogue illustration".to_string(),
                    },
                    RawHit {
                        id: "vase".to_string(),
                        thumb_url: "https://x/vase.jpg".to_string(),
                        attribution: String::new(),
                        title: "Ceramic vase isolated on white".to_string(),
                        relevance_metadata: "Ceramic vase isolated on white".to_string(),
                    },
                ]),
                unexpected => panic!("unexpected Openverse query: {unexpected}"),
            }
        }
    };

    let hits = crate::net::block_on_image_runtime(fetch_relevant_openverse_list_with(
        "ceramic vase studio photo",
        fetcher,
    ))
    .expect("provider requests succeed");

    assert_eq!(
        calls.lock().expect("calls lock").as_slice(),
        ["ceramic vase studio photo", "ceramic vase"]
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "vase");
}

#[test]
fn openverse_network_failure_falls_through_without_retrying() {
    let calls = Arc::new(AtomicUsize::new(0));
    let fetch_calls = calls.clone();
    let fetcher = move |_query: String| {
        fetch_calls.fetch_add(1, Ordering::SeqCst);
        async move { None }
    };

    let hits = crate::net::block_on_image_runtime(fetch_relevant_openverse_list_with(
        "ceramic vase studio photo",
        fetcher,
    ));

    assert!(
        hits.is_none(),
        "request failure must retain fallback semantics"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "network failure must not trigger another Openverse request"
    );
}

#[test]
fn non_photo_query_keeps_relevant_illustrations() {
    let hits = vec![RawHit {
        id: "drawing".to_string(),
        thumb_url: "https://x/drawing.jpg".to_string(),
        attribution: String::new(),
        title: "Ceramic tableware illustration".to_string(),
        relevance_metadata: "Ceramic tableware illustration".to_string(),
    }];

    let ranked = retain_relevant_hits(hits, "ceramic tableware illustration");

    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].id, "drawing");
}

#[test]
fn descriptor_only_studio_photo_query_preserves_unclassified_hits() {
    let hits = vec![RawHit {
        id: "unclassified".to_string(),
        thumb_url: "https://x/unclassified.jpg".to_string(),
        attribution: String::new(),
        title: String::new(),
        relevance_metadata: String::new(),
    }];

    let ranked = retain_relevant_hits(hits, "studio product photo");

    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].id, "unclassified");
}

#[test]
fn descriptor_only_isolated_query_still_requires_positive_evidence() {
    let hits = [
        ("unclassified", ""),
        ("isolated", "Product cutout on a white background"),
    ]
    .into_iter()
    .map(|(id, metadata)| RawHit {
        id: id.to_string(),
        thumb_url: format!("https://x/{id}.jpg"),
        attribution: String::new(),
        title: metadata.to_string(),
        relevance_metadata: metadata.to_string(),
    })
    .collect();

    let ranked = retain_relevant_hits(hits, "isolated studio product photo");

    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].id, "isolated");
}

#[test]
fn failed_relevant_thumbnail_never_falls_through_to_downloadable_holiday_hit() {
    let hits = vec![
        RawHit {
            id: "chair".to_string(),
            thumb_url: "https://x/chair.jpg".to_string(),
            attribution: "By Chair".to_string(),
            title: "Modern armchair".to_string(),
            relevance_metadata: "Modern armchair".to_string(),
        },
        RawHit {
            id: "holiday".to_string(),
            thumb_url: "https://x/holiday.jpg".to_string(),
            attribution: "By Holiday".to_string(),
            title: "Christmas technology celebration".to_string(),
            relevance_metadata: "Christmas technology celebration".to_string(),
        },
    ];
    let relevant = retain_relevant_hits(hits, "warm modern armchair");
    let calls = Arc::new(AtomicUsize::new(0));
    let fetch_calls = calls.clone();
    let fetcher = move |url: String| {
        let fetch_calls = fetch_calls.clone();
        async move {
            fetch_calls.fetch_add(1, Ordering::SeqCst);
            (url == "https://x/holiday.jpg").then(|| "data:image/jpeg;base64,HOLIDAY".to_string())
        }
    };

    let result = crate::net::block_on_image_runtime(materialize_first_thumb(relevant, &fetcher));

    assert!(
        result.is_none(),
        "unrelated downloadable hit must be fenced out"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1, "only armchair was tried");
}
