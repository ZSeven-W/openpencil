use super::rewrite_queries;

#[test]
fn editorial_product_intent_keeps_the_authored_subject() {
    let rewrites = rewrite_queries(
        "cleansing oil bottle",
        "editorial product photography of an elegant amber glass cleansing oil bottle, soft light, warm minimal backdrop",
    );
    assert_eq!(rewrites, ["glass cleansing oil bottle"]);
}

#[test]
fn long_art_direction_prefix_cannot_replace_product_subjects() {
    for prefix in [
        "sculptural gallery-grade luminous sculpted forms and dramatic editorial product photography of",
        "avant-garde luxurious elegant abstract composition with custom layered framing around",
    ] {
        let intent = format!("{prefix} ceramic table lamp, soft light, warm minimal backdrop");
        // Three concrete subject words: no shorter retry (0930 rule), and
        // the intent rewrite equals the query itself.
        assert!(rewrite_queries("ceramic table lamp", &intent).is_empty());
    }
}

#[test]
fn intervening_portrait_modifiers_do_not_displace_the_profession() {
    assert_eq!(
        rewrite_queries(
            "female dermatologist portrait",
            "editorial photography of a female experienced accomplished dermatologist in a clinic, warm rim lighting",
        ),
        ["female dermatologist", "female dermatologist clinic"]
    );
}

#[test]
fn intervening_fitness_modifiers_do_not_displace_subject_anchors() {
    assert_eq!(
        rewrite_queries(
            "fitness model gym workout",
            "cinematic photography of a fitness athletic confident model training in a gym, dramatic light",
        ),
        ["fitness model gym"]
    );
}

#[test]
fn intent_with_no_subject_overlap_does_not_create_a_new_subject() {
    assert!(
        rewrite_queries("ceramic vase", "editorial portrait of a musician on stage").is_empty()
    );
}

#[test]
fn one_generic_word_cannot_redirect_a_multiword_subject() {
    assert!(rewrite_queries("cleansing oil bottle", "wine bottle on a wooden table").is_empty());
    assert!(rewrite_queries(
        "fitness model gym",
        "architectural model of an apartment building"
    )
    .is_empty());
}

#[test]
fn nearby_literal_color_and_material_refine_the_subject_without_replacing_it() {
    assert_eq!(
        rewrite_queries(
            "luxury face cream jar",
            "editorial product photography of a premium white face cream jar, soft natural light, clean minimal ivory setting",
        ),
        ["white face cream jar"]
    );
    assert_eq!(
        rewrite_queries(
            "cleansing oil bottle",
            "editorial photo of an amber cleansing oil bottle"
        ),
        ["amber cleansing oil bottle"]
    );
}

#[test]
fn abstract_praise_is_not_borrowed_as_a_subject_modifier() {
    for adjective in [
        "premium",
        "elegant",
        "editorial",
        "exquisite",
        "astonishing",
    ] {
        // The only candidate rewrite is the query itself: nothing to retry.
        assert!(rewrite_queries(
            "face cream jar",
            &format!("photo of an {adjective} face cream jar")
        )
        .is_empty());
    }
}

#[test]
fn trailing_praise_cannot_fill_a_portrait_or_fitness_subject() {
    for adjective in ["premium", "elegant", "editorial", "confident"] {
        assert_eq!(
            rewrite_queries(
                "female dermatologist portrait",
                &format!("photo of a female dermatologist {adjective}")
            ),
            ["female dermatologist"]
        );
        assert_eq!(
            rewrite_queries(
                "fitness model workout",
                &format!("photo of a fitness model {adjective}")
            ),
            ["fitness model"]
        );
    }
}

#[test]
fn a_described_location_cannot_turn_into_a_praise_keyword() {
    for adjective in ["premium", "elegant", "editorial"] {
        for (query, subject, expected) in [
            (
                "female dermatologist portrait",
                "female dermatologist",
                vec!["female dermatologist"],
            ),
            (
                "fitness model workout",
                "fitness model",
                vec!["fitness model"],
            ),
            ("ceramic vase product", "ceramic vase", vec!["ceramic vase"]),
        ] {
            assert_eq!(
                rewrite_queries(
                    query,
                    &format!("photo of a {subject} in a {adjective} studio")
                ),
                expected,
                "query={query}, adjective={adjective}"
            );
        }
    }
}

#[test]
fn a_background_color_is_not_mistaken_for_an_adjacent_subject_modifier() {
    // The only candidate rewrite is the query itself: nothing to retry.
    assert!(rewrite_queries(
        "face cream jar",
        "white background photography of a face cream jar"
    )
    .is_empty());
}

#[test]
fn a_modifier_never_displaces_a_four_word_subject() {
    assert_eq!(
        rewrite_queries("ceramic table lamp", "photo of a white ceramic table lamp"),
        ["white ceramic table lamp"]
    );
    // Four subject words with no framing noun: no shorter retry at all.
    assert!(rewrite_queries(
        "ceramic reading table lamp",
        "photo of a white ceramic reading table lamp"
    )
    .is_empty());
}

#[test]
fn subject_matching_handles_case_and_existing_plural_normalization() {
    // The intent rewrite normalizes "bottles" back onto the query subject,
    // which equals the query and is dropped.
    assert!(rewrite_queries(
        "cleansing oil bottle",
        "Editorial Product photography of CLEANSING OIL BOTTLES, soft light",
    )
    .is_empty());
}

#[test]
fn hyphenated_exercise_subject_survives_rich_intent_rewrite() {
    assert_eq!(
        rewrite_queries(
            "athlete push-up exercise",
            "cinematic photo of an athlete performing a controlled push-up exercise, dramatic light",
        ),
        ["athlete push-up"]
    );
}

#[test]
fn lighting_clause_is_not_used_to_pad_a_short_subject() {
    assert_eq!(
        rewrite_queries(
            "female dermatologist portrait",
            "editorial photo of a female dermatologist, soft light, warm backdrop",
        ),
        ["female dermatologist"]
    );
}

#[test]
fn two_keyword_rewrite_precedes_the_intent_rewrite() {
    assert_eq!(
        rewrite_queries(
            "female dermatologist portrait",
            "professional photo of a female dermatologist in a clinic",
        ),
        vec![
            "female dermatologist".to_string(),
            "female dermatologist clinic".to_string(),
        ]
    );
}

#[test]
fn intent_rewrite_strips_style_words() {
    assert_eq!(
        rewrite_queries(
            "fitness model gym workout",
            "professional photo of a fitness model in a gym, cinematic lighting, 4k hd",
        ),
        vec!["fitness model gym".to_string(),]
    );
}

#[test]
fn empty_intent_never_invents_a_rewrite() {
    // Two words: the two-keyword rewrite equals the query and is dropped.
    assert!(rewrite_queries("desk lamp", "").is_empty());
    // Three subject words: no shorter retry exists (0930 rule).
    assert!(rewrite_queries("oak desk lamp", "").is_empty());
}

#[test]
fn rewrites_matching_the_query_or_each_other_are_dropped() {
    // Both rewrites resolve to the primary query itself: nothing to retry.
    assert!(rewrite_queries(
        "dermatologist portrait",
        "photo of a dermatologist portrait"
    )
    .is_empty());
    assert_eq!(
        rewrite_queries(
            "dermatologist clinic portrait",
            "professional photo clinic portrait",
        ),
        vec![
            "dermatologist clinic".to_string(),
            "clinic portrait".to_string(),
        ]
    );
    // The intent rewrite duplicates the two-keyword rewrite: deduped.
    assert_eq!(
        rewrite_queries(
            "female dermatologist portrait",
            "professional photo of a female dermatologist",
        ),
        vec!["female dermatologist".to_string()]
    );
}

#[test]
fn framing_words_leave_but_three_word_subjects_never_shorten() {
    // "workout" is framing, not the subject: the retry keeps the domain word.
    assert_eq!(
        rewrite_queries("kettlebell swing workout", ""),
        ["kettlebell swing"]
    );
    // Dropping the leading word of a three-word subject loses the domain
    // (0930: "rose serum bottle" → 1960s medical serum): no retry instead.
    assert!(rewrite_queries("rose serum bottle", "").is_empty());
}
