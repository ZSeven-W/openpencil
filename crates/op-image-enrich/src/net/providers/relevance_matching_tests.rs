use super::super::{fence_rejection, retain_relevant_hits_for_intent, rewrite_queries, RawHit};
use super::{action_alias, evidence};

fn hit(title: &str, tags: &str) -> RawHit {
    RawHit {
        id: title.to_string(),
        thumb_url: "https://example.test/image.jpg".to_string(),
        attribution: String::new(),
        title: title.to_string(),
        relevance_metadata: format!("{title} {tags}"),
        original: None,
    }
}

fn overlap(query: &[&str], text: &str) -> usize {
    evidence(
        &query
            .iter()
            .map(|word| (*word).to_string())
            .collect::<Vec<_>>(),
        text,
    )
    .overlap
}

#[test]
fn hyphenated_catalogue_product_gets_subject_evidence() {
    let candidate = hit("EBE SERUM-bottle+box", "");
    let intent =
        "editorial product photography of an amber glass serum dropper bottle on ivory linen";
    assert_eq!(fence_rejection(&candidate, "serum bottle", intent), None);
    assert_eq!(
        retain_relevant_hits_for_intent(vec![candidate], "serum bottle", intent).len(),
        1,
        "metadata eligibility only; the visual judge still checks the full brief"
    );
    let score = evidence(&["serum".into(), "bottle".into()], "EBE SERUM-bottle+box");
    assert_eq!(score.overlap, 2);
    assert_eq!(score.unmatched, ["ebe", "box"]);
}

#[test]
fn human_tagged_stretch_candidate_survives_inflection_difference() {
    let candidate = hit(
        "Neck Stretch",
        "person people girl stretches shoulderstretch",
    );
    let intent = "A person stretching shoulders and neck while seated at a desk, calm office light";
    assert_eq!(fence_rejection(&candidate, "neck stretching", intent), None);
    assert_eq!(
        retain_relevant_hits_for_intent(vec![candidate], "neck stretching", intent).len(),
        1
    );
    // Lexical matching cannot establish whether the actor is human. This
    // candidate remains subject to the same strict visual judge as before.
    assert_eq!(
        fence_rejection(&hit("Neck stretching", "bird"), "neck stretching", intent),
        None
    );
}

#[test]
fn regular_sibilant_action_forms_match_in_both_directions() {
    for stem in ["stretch", "watch", "search", "wash", "brush", "press"] {
        let forms = [stem.to_string(), format!("{stem}ing"), format!("{stem}es")];
        for query in &forms {
            for candidate in &forms {
                assert_eq!(overlap(&[query], candidate), 1, "{query} / {candidate}");
            }
        }
    }
}

#[test]
fn ing_is_not_a_general_stemming_rule() {
    for (word, clipped) in [
        ("thing", "th"),
        ("spring", "spr"),
        ("string", "str"),
        ("morning", "morn"),
        ("building", "build"),
        ("ceiling", "ceil"),
        ("painting", "paint"),
        ("clothing", "cloth"),
        ("Beijing", "beij"),
        ("Boeing", "boe"),
        ("Sterling", "sterl"),
        ("king", "k"),
        ("ring", "r"),
        ("sing", "s"),
    ] {
        assert_eq!(action_alias(&word.to_lowercase()), None, "{word}");
        assert_eq!(overlap(&[word], word), 1, "{word} retains exact spelling");
        assert_eq!(overlap(&[word], clipped), 0, "{word} / {clipped}");
        assert_eq!(overlap(&[clipped], word), 0, "{clipped} / {word}");
    }
}

#[test]
fn short_stems_and_unimplemented_inflections_remain_exact() {
    for (word, other) in [
        ("aching", "ach"),
        ("jogging", "jog"),
        ("running", "run"),
        ("dancing", "dance"),
        ("hinges", "hing"),
    ] {
        assert_eq!(action_alias(word), None, "{word}");
        assert_eq!(overlap(&[word], other), 0, "{word} / {other}");
    }
}

#[test]
fn query_compounds_require_all_adjacent_parts_and_count_once() {
    for text in ["push-up exercise", "push up exercise", "push-ups exercise"] {
        assert_eq!(overlap(&["push-up", "exercise"], text), 2, "{text}");
    }
    for text in ["up exercise", "push exercise", "push down up exercise"] {
        assert_eq!(overlap(&["push-up", "exercise"], text), 1, "{text}");
        assert!(
            retain_relevant_hits_for_intent(vec![hit(text, "")], "push-up exercise", "").is_empty()
        );
    }
    assert_eq!(overlap(&["push-ups", "exercise"], "push up exercise"), 2);
    assert_eq!(overlap(&["push-up"], "push-up push up push-ups"), 1);
}

#[test]
fn t_shirt_does_not_degrade_to_shirt_alone() {
    for text in ["blue t-shirt", "blue t shirt", "blue t-shirts"] {
        assert_eq!(overlap(&["blue", "t-shirt"], text), 2, "{text}");
    }
    assert_eq!(overlap(&["blue", "t-shirt"], "blue shirt"), 1);
    assert_eq!(overlap(&["t-shirt"], "shirt"), 0);
    assert_eq!(overlap(&["t-shirts"], "t-shirt"), 1);
}

#[test]
fn repeated_aliases_cannot_inflate_a_query_condition() {
    let score = evidence(
        &["neck".into(), "stretching".into()],
        "Neck stretch stretching stretches",
    );
    assert_eq!(score.overlap, 2);
    assert!(score.unmatched.is_empty());
    assert_eq!(overlap(&["stretching"], "stretch stretches stretching"), 1);
    assert_eq!(
        overlap(&["serum", "bottle"], "serum-bottle serum bottle bottles"),
        2
    );
}

#[test]
fn repeated_query_aliases_cannot_replace_a_missing_subject() {
    for (query, title) in [
        ("neck stretch stretching", "stretch"),
        ("serum bottle bottle", "bottle"),
        ("serum bottle bottles", "bottles"),
        ("blue t-shirt shirt", "t-shirt"),
        ("blue shirt t-shirt", "t-shirt"),
    ] {
        assert_eq!(
            fence_rejection(&hit(title, ""), query, ""),
            Some("too few subject words in title/tags"),
            "{query} / {title}"
        );
    }
    assert_eq!(overlap(&["neck", "stretch", "stretching"], "stretch"), 1);
    assert_eq!(overlap(&["serum", "bottle", "bottle"], "bottle"), 1);
    assert_eq!(overlap(&["blue", "t-shirt", "shirt"], "t-shirt"), 1);
    assert_eq!(
        fence_rejection(&hit("Neck stretches", ""), "neck stretch stretching", ""),
        None
    );
    assert_eq!(
        fence_rejection(&hit("Stretch", ""), "stretch stretching", ""),
        None,
        "one distinct subject needs one match, not two copies of its spelling"
    );
}

#[test]
fn existing_plural_wood_and_knit_matches_survive() {
    for (query, candidate) in [
        ("bottle", "bottles"),
        ("cherry", "cherries"),
        ("wood", "wooden"),
        ("wooden", "wood"),
        ("knit", "knitted"),
        ("knitting", "knit"),
        ("knitted", "knitting"),
    ] {
        assert_eq!(overlap(&[query], candidate), 1, "{query} / {candidate}");
    }
}

#[test]
fn compound_aliases_preserve_the_existing_metadata_fences() {
    assert_eq!(
        fence_rejection(
            &hit("Serum-bottle illustration", ""),
            "serum bottle",
            "product photography"
        ),
        Some("photo requested but metadata names a non-photo medium")
    );
    assert_eq!(
        fence_rejection(&hit("IKEA table-lamp", ""), "table lamp", ""),
        Some("off-subject theme or unrequested brand")
    );
    assert_eq!(
        fence_rejection(
            &hit("Table-lamp living room", ""),
            "table lamp studio photo",
            ""
        ),
        Some("photo requested but metadata describes a staged room")
    );
    assert_eq!(
        fence_rejection(&hit("Product image", "#serum-bottle"), "serum bottle", ""),
        Some("too few subject words in title/tags")
    );
}

#[test]
fn hard_fence_markers_inside_compounds_are_not_hidden() {
    for (title, query, intent, reason) in [
        (
            "Serum-bottle-illustration",
            "serum bottle",
            "product photography",
            "photo requested but metadata names a non-photo medium",
        ),
        (
            "IKEA-table-lamp",
            "table lamp",
            "",
            "off-subject theme or unrequested brand",
        ),
        (
            "Table-lamp-collage",
            "table lamp",
            "",
            "off-subject theme or unrequested brand",
        ),
        (
            "Table-lamp-living-room",
            "table lamp studio photo",
            "",
            "photo requested but metadata describes a staged room",
        ),
    ] {
        assert_eq!(
            fence_rejection(&hit(title, ""), query, intent),
            Some(reason),
            "{title}"
        );
    }
}

#[test]
fn explicitly_authored_compound_opt_ins_remain_valid() {
    for (query, title) in [
        ("IKEA-table-lamp", "IKEA-table-lamp"),
        ("plush-toy studio photo", "Plush-toy isolated photograph"),
        ("table lamp living-room photo", "Table-lamp-living-room"),
        (
            "lounge chair studio photo",
            "Lounge-chair isolated photograph",
        ),
    ] {
        assert_eq!(fence_rejection(&hit(title, ""), query, ""), None, "{query}");
    }
}

#[test]
fn compound_matching_does_not_turn_non_isolated_into_isolation_evidence() {
    let candidates = vec![
        hit("Table-lamp-non-isolated", ""),
        hit("Table-lamp not-isolated", ""),
        hit("Table-lamp-living-room", "isolated"),
        hit("Table-lamp isolated on white background", ""),
    ];
    let retained = retain_relevant_hits_for_intent(candidates, "table lamp isolated photo", "");
    assert_eq!(
        retained
            .iter()
            .map(|hit| hit.title.as_str())
            .collect::<Vec<_>>(),
        ["Table-lamp isolated on white background"],
        "strict isolation evidence and the room fence still precede subject matching"
    );
}

#[test]
fn provider_query_generation_is_not_stemmed_or_split() {
    assert_eq!(
        rewrite_queries(
            "athlete push-up exercise",
            "photo of an athlete performing a push-up exercise"
        ),
        ["push-up exercise"]
    );
    assert_eq!(
        rewrite_queries("blue cotton t-shirt", ""),
        ["cotton t-shirt"]
    );
    assert_eq!(
        rewrite_queries(
            "neck stretching",
            "A person stretching shoulders and neck while seated at a desk, calm office light"
        ),
        ["stretching neck"]
    );
}
