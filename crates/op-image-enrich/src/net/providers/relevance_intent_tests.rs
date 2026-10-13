use super::*;

const NECK: &str =
    "A person stretching shoulders and neck while seated at a desk, calm office light";
const PLANK: &str = "A person doing a plank exercise on a mat in a minimal bright gym";

fn hit(id: &str, title: &str, tags: &str) -> RawHit {
    RawHit {
        id: id.into(),
        title: title.into(),
        relevance_metadata: format!("{title} {tags}"),
        thumb_url: format!("https://example.test/{id}.jpg"),
        attribution: String::new(),
        original: None,
    }
}

#[test]
fn a_generic_training_label_uses_the_authored_exercise_first() {
    let (primary, retries) = retrieval_plan("core training", PLANK);
    assert_eq!(primary, "person plank exercise");
    assert_eq!(retries.first().unwrap(), "core training");
    assert!(retries.len() <= 2);
}

#[test]
fn person_context_disambiguates_neck_stretching_without_inventing_an_action() {
    let (primary, retries) = retrieval_plan("neck stretching", NECK);
    assert_eq!(primary, "person neck stretching");
    assert_eq!(retries.first().unwrap(), "neck stretching");
    assert!(retries.len() <= 2);
    assert_eq!(
        human_activity_query("neck stretch", NECK).unwrap(),
        "person neck stretch"
    );
}

#[test]
fn concrete_product_material_precedes_the_original_broad_query() {
    let intent =
        "editorial product photography of an amber glass serum dropper bottle on ivory linen";
    let (primary, retries) = retrieval_plan("luxury serum bottle", intent);
    assert_eq!(primary, "glass serum bottle");
    assert_eq!(retries.first().unwrap(), "luxury serum bottle");
    assert!(retries.len() <= 2);
}

#[test]
fn unchanged_primary_keeps_the_existing_rewrite_plan() {
    for (q, intent) in [
        ("ceramic vase", ""),
        ("plush toy", "soft toy"),
        ("ceramic table lamp", "photo of a ceramic table lamp"),
    ] {
        let (primary, retries) = retrieval_plan(q, intent);
        assert_eq!(primary, q);
        assert_eq!(retries, rewrite_queries(q, intent));
    }
}

#[test]
fn unrelated_specific_subjects_cannot_be_replaced_by_a_human_brief() {
    for query in [
        "dog training",
        "bird neck stretching",
        "horse exercise",
        "core computer training",
        "ceramic vase",
    ] {
        assert_eq!(retrieval_plan(query, PLANK).0, query);
    }
    assert_eq!(
        retrieval_plan("bird neck stretching", NECK).0,
        "bird neck stretching"
    );
}

#[test]
fn negation_background_people_and_non_photo_subjects_do_not_add_a_person() {
    for intent in [
        "A person not doing a plank exercise",
        "No people doing a plank exercise",
        "An empty gym with a person doing a plank exercise in the background",
        "A sculpture of a person doing a plank exercise",
        "A person doing computer training",
        "A person performing a song on stage",
        "一个人在健身房做平板支撑",
    ] {
        assert_eq!(
            retrieval_plan("core training", intent).0,
            "core training",
            "{intent}"
        );
    }
}

#[test]
fn explicit_subjects_are_not_duplicated_or_given_inferred_demographics() {
    assert_eq!(
        retrieval_plan("person neck stretching", NECK).0,
        "person neck stretching"
    );
    let (q, _) = retrieval_plan("yoga stretching", "A woman doing yoga stretching on a mat");
    assert_eq!(q, "person yoga stretching");
    assert!(
        !q.contains("woman"),
        "retrieval may broaden, the unchanged judge brief checks gender"
    );
}

#[test]
fn compound_action_words_remain_whole() {
    let (q, _) = retrieval_plan("core training", "An athlete doing push-ups on a mat");
    assert_eq!(q, "person push-ups");
    assert_eq!(
        human_activity_query(
            "push-up exercise",
            "A person doing push-ups exercise on a mat"
        )
        .unwrap(),
        "person push-up exercise"
    );
}

#[test]
fn human_tags_win_over_short_animal_titles_without_dropping_sparse_metadata() {
    let ranked = retain_relevant_hits_for_intent(
        vec![
            hit("bird", "Neck Stretch", "swans bird"),
            hit("human", "Gentle neck stretch exercise", "woman fitness"),
            hit("sparse", "Neck stretching", ""),
        ],
        "neck stretching",
        NECK,
    );
    assert_eq!(ranked[0].id, "human");
    assert_eq!(
        ranked.len(),
        3,
        "context is ranking evidence, not a new hard fence"
    );
}

#[test]
fn catalogue_metadata_cannot_opt_a_brand_or_illustration_into_a_photo_slot() {
    let ranked = retain_relevant_hits_for_intent(
        vec![
            hit(
                "drawing",
                "Neck stretch illustration",
                "person woman fitness",
            ),
            hit("brand", "Nike neck stretch", "person woman fitness"),
            hit("photo", "Neck Stretch", "person"),
        ],
        "neck stretching photo",
        "A person stretching neck, photograph",
    );
    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].id, "photo");
}

#[test]
fn physical_subject_context_wins_but_background_color_does_not() {
    let ranked = retain_relevant_hits_for_intent(
        vec![
            hit("medical", "Serum bottle", "vial laboratory"),
            hit("cosmetic", "Serum bottle", "amber glass skincare"),
        ],
        "serum bottle",
        "editorial photography of an amber serum bottle on linen",
    );
    assert_eq!(ranked[0].id, "cosmetic");
    let ranked = retain_relevant_hits_for_intent(
        vec![
            hit("plain", "Serum bottle", ""),
            hit("background", "Serum bottle", "ivory linen"),
        ],
        "serum bottle",
        "photo of a serum bottle on ivory linen",
    );
    assert_eq!(ranked[0].id, "plain");
}

#[test]
fn equal_context_preserves_existing_ranking_and_does_not_count_hashtag_soup() {
    let ranked = retain_relevant_hits_for_intent(
        vec![
            hit("first", "Neck Stretch", ""),
            hit("second", "Neck Stretch", "#person #woman"),
        ],
        "neck stretching",
        NECK,
    );
    assert_eq!(ranked[0].id, "first");
}

#[test]
fn rewrite_budget_stays_bounded_and_original_query_is_always_available() {
    for (query, intent) in [
        ("core training", PLANK),
        ("neck stretching", NECK),
        ("face cream jar", "photo of a white ceramic face cream jar"),
    ] {
        let (primary, retries) = retrieval_plan(query, intent);
        assert!(retries.len() <= 2);
        assert!(primary == query || retries.iter().any(|r| r == query));
        assert!(retries.iter().all(|r| r != &primary));
        let unique: std::collections::HashSet<_> = retries.iter().collect();
        assert_eq!(unique.len(), retries.len());
    }
}
