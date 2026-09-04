//! Relevance fence and ranking tests over `retain_relevant_hits`. Split
//! out of `web_image_search_tests.rs` at the 800-line cap; pure code motion.

use super::*;

#[test]
fn relevance_fence_rejects_holiday_hit_and_accepts_armchair_title_or_tag() {
    let json = serde_json::json!({
        "results": [
            {
                "id": "holiday",
                "thumbnail": "https://x/holiday.jpg",
                "title": "Christmas technology celebration",
                "tags": [{"name": "computer"}, {"name": "holiday"}]
            },
            {
                "id": "title",
                "thumbnail": "https://x/title.jpg",
                "title": "Boucle armchair in a quiet room"
            },
            {
                "id": "tag",
                "thumbnail": "https://x/tag.jpg",
                "title": "Neutral furniture study",
                "tags": [{"name": "armchair"}]
            }
        ]
    });
    assert_eq!(
        core_query_words("warm modern armchair"),
        vec!["armchair".to_string()]
    );

    let hits = retain_relevant_hits(parse_openverse_results(&json), "warm modern armchair");
    let ids: Vec<&str> = hits.iter().map(|hit| hit.id.as_str()).collect();

    assert_eq!(ids, ["title", "tag"]);
    assert!(!ids.contains(&"holiday"));
}

#[test]
fn relevance_fence_returns_empty_when_no_hit_mentions_the_subject() {
    let hits = vec![RawHit {
        id: "holiday".to_string(),
        thumb_url: "https://x/holiday.jpg".to_string(),
        attribution: "By X".to_string(),
        title: "Christmas technology celebration".to_string(),
        relevance_metadata: "Christmas technology celebration computer".to_string(),
    }];

    assert!(retain_relevant_hits(hits, "warm modern armchair").is_empty());
}

#[test]
fn relevance_fence_ranks_more_complete_matches_stably() {
    let hits = [
        ("material-only", "Natural wool textile"),
        ("complete-first", "Knitted wool sweater"),
        ("complete-second", "Wool sweater, hand knitted"),
        ("subject-only", "Winter sweater"),
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

    let ranked = retain_relevant_hits(hits, "minimal knitted wool sweater");
    let ids: Vec<&str> = ranked.iter().map(|hit| hit.id.as_str()).collect();

    assert_eq!(
        ids,
        ["complete-first", "complete-second"],
        "multi-word products require more than one concrete subject match while equal overlap preserves provider order"
    );
}

#[test]
fn multi_word_product_query_rejects_one_generic_match() {
    let hits = [
        ("setup", "Photography lamp setup"),
        ("table-lamp", "Modern table lamp in a studio"),
        ("ceramic-lamp", "Ceramic lamp product photograph"),
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

    let ranked = retain_relevant_hits(hits, "ceramic table lamp studio photo");
    let ids: Vec<&str> = ranked.iter().map(|hit| hit.id.as_str()).collect();

    assert_eq!(ids, ["table-lamp", "ceramic-lamp"]);
}

#[test]
fn single_word_product_query_keeps_one_exact_subject_match() {
    let hits = vec![RawHit {
        id: "armchair".to_string(),
        thumb_url: "https://x/armchair.jpg".to_string(),
        attribution: String::new(),
        title: "Studio armchair photograph".to_string(),
        relevance_metadata: "Studio armchair photograph".to_string(),
    }];

    let ranked = retain_relevant_hits(hits, "armchair studio photo");

    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].id, "armchair");
}

#[test]
fn product_photo_query_rejects_room_scenes_unless_the_query_requests_one() {
    let hits = [
        (
            "scene",
            "Cream armchair in a living room",
            "Cream armchair furniture in a living room interior",
        ),
        (
            "product",
            "Cream armchair",
            "Cream armchair isolated product photograph",
        ),
    ]
    .into_iter()
    .map(|(id, title, metadata)| RawHit {
        id: id.to_string(),
        thumb_url: format!("https://x/{id}.jpg"),
        attribution: String::new(),
        title: title.to_string(),
        relevance_metadata: metadata.to_string(),
    })
    .collect();

    let ranked = retain_relevant_hits(hits, "cream armchair studio photo");
    let ids: Vec<&str> = ranked.iter().map(|hit| hit.id.as_str()).collect();
    assert_eq!(ids, ["product"]);

    let requested_scene = vec![RawHit {
        id: "scene".to_string(),
        thumb_url: "https://x/scene.jpg".to_string(),
        attribution: String::new(),
        title: "Cream armchair in a living room".to_string(),
        relevance_metadata: "Cream armchair furniture in a living room interior".to_string(),
    }];
    assert_eq!(
        retain_relevant_hits(requested_scene, "cream armchair living room photo").len(),
        1,
        "an explicit room request opts into scene-heavy results"
    );
}

#[test]
fn lounge_chair_does_not_opt_into_scenes_but_explicit_scene_phrases_do() {
    assert!(!query_requests_scene("lounge chair studio photo"));
    assert!(query_requests_scene("lounge chair in a living room photo"));
    assert!(query_requests_scene("lounge chair in a hotel lounge photo"));
    assert!(query_requests_scene("lounge chair interior photo"));

    let hits = [
        (
            "hotel-scene",
            "Modern lounge chair",
            "Modern lounge chair in a hotel lounge interior",
        ),
        (
            "product",
            "Modern lounge chair",
            "Modern lounge chair isolated product photograph",
        ),
    ]
    .into_iter()
    .map(|(id, title, metadata)| RawHit {
        id: id.to_string(),
        thumb_url: format!("https://x/{id}.jpg"),
        attribution: String::new(),
        title: title.to_string(),
        relevance_metadata: metadata.to_string(),
    })
    .collect();

    let ranked = retain_relevant_hits(hits, "lounge chair studio photo");
    let ids: Vec<&str> = ranked.iter().map(|hit| hit.id.as_str()).collect();
    assert_eq!(ids, ["product"]);
}

#[test]
fn product_photo_scene_fence_covers_common_room_metadata() {
    for marker in SCENE_HEAVY_RESULT_WORDS {
        assert!(
            metadata_is_scene_heavy(&format!("table lamp {marker}")),
            "{marker} must be recognized as scene-heavy metadata"
        );
    }
}

#[test]
fn concise_subject_dense_vase_title_beats_long_kylix_metadata() {
    let hits = [
        (
            "kylix",
            "Ancient Greek kylix drinking cup ceramic vase studio collection",
            "Ancient Greek kylix drinking cup ceramic vase studio pottery collection",
        ),
        (
            "vase",
            "Vase",
            "Vase ceramic pottery isolated product photograph",
        ),
    ]
    .into_iter()
    .map(|(id, title, metadata)| RawHit {
        id: id.to_string(),
        thumb_url: format!("https://x/{id}.jpg"),
        attribution: String::new(),
        title: title.to_string(),
        relevance_metadata: metadata.to_string(),
    })
    .collect();

    let ranked = retain_relevant_hits(hits, "ceramic vase studio photo");
    let ids: Vec<&str> = ranked.iter().map(|hit| hit.id.as_str()).collect();

    assert_eq!(ids, ["vase", "kylix"]);
}

#[test]
fn photo_query_rejects_explicit_illustration_and_catalog_hits() {
    let hits = [
        (
            "studio",
            "Ceramic tableware isolated studio product photograph",
        ),
        ("engraving", "Vintage ceramic tableware engraving"),
        ("catalog", "Ceramic tableware catalogue page"),
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

    let ranked = retain_relevant_hits(
        hits,
        "ceramic tableware isolated studio product photography",
    );
    let ids: Vec<&str> = ranked.iter().map(|hit| hit.id.as_str()).collect();

    assert_eq!(ids, ["studio"]);
}

#[test]
fn isolated_query_requires_positive_isolation_metadata() {
    let hits = [
        ("bare-studio", "Modern lounge chair studio photograph"),
        (
            "isolated",
            "Modern lounge chair isolated product photograph",
        ),
        ("cutout", "Modern lounge chair cutout product photograph"),
        (
            "white-background",
            "Modern lounge chair on a white background",
        ),
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

    let ranked = retain_relevant_hits(hits, "lounge chair isolated photo");
    let ids: Vec<&str> = ranked.iter().map(|hit| hit.id.as_str()).collect();

    assert_eq!(ids, ["isolated", "white-background", "cutout"]);
}
