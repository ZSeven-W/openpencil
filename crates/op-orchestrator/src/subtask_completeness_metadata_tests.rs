use super::*;

// Exact hero elements from the 1018 run-4 full-pipeline planning response.
const WORKOUT_HERO: &str = "ARCHETYPE: image-led — protagonist hero photo with workout title overlaid, first screen ~40%, title/meta owned by this section only, imagery one relevant workout photo. Back button (circular, top-left overlay), edge-to-edge hero photo with bottom gradient scrim, workout title 'Full Body Strength' on scrim, meta row (duration 45 min · 12 exercises · 320 kcal), difficulty badge group (Beginner / Intermediate chips with Intermediate active), pinned inside hero area";

fn two_item_outcome() -> SubtaskOutcome {
    SubtaskOutcome {
        id: "hero".into(),
        node_count: 1,
        inserted_root_ids: vec!["section".into()],
        error: None,
        headline: None,
        subtask: None,
    }
}

#[test]
fn metadata_workout_counter_does_not_retry_two_difficulty_badges_as_twelve_items() {
    let task = subtask("Hero with Title & Meta", Some(WORKOUT_HERO));
    let sink = sink_with_section((0..2).map(card).collect());
    assert_eq!(delivered_item_count(&sink, &["section".into()]), 2);
    assert_eq!(expected_item_count(&task), None);
    assert!(incomplete_attempt(&sink, &task, &two_item_outcome()).is_none());
}

#[test]
fn metadata_parentheses_preserve_real_lists_and_unrelated_parentheses() {
    for elements in [
        "6 exercises, each with thumbnail and sets/reps",
        "meta row (12 exercises); list of 6 exercises with thumbnails",
        "metadata text (12 exercises) and 6 rows below",
        "meta row (list of 6 exercises)",
        "meta row (6 exercises list)",
        "meta row (6 exercises, each with thumbnail)",
        "meta row (show 6 exercises)",
        "exercise list (6 exercises)",
        "(6 exercises)",
        "meta row (12 exercises; list of 6 exercises", // Unclosed is not copy.
        "meta row (summary (12 exercises), 6 exercises)", // Nested is ambiguous.
    ] {
        let task = subtask("Exercise List", Some(elements));
        let expected = if elements.contains("12 exercises;") || elements.contains("summary (") {
            12
        } else {
            6
        };
        assert_eq!(expected_item_count(&task), Some(expected), "{elements}");
        let sink = sink_with_section((0..2).map(card).collect());
        let failure = incomplete_attempt(&sink, &task, &two_item_outcome()).unwrap();
        assert_eq!(failure.expected, expected, "{elements}");
        assert_eq!(failure.delivered, 2);
    }
    assert_eq!(
        expected_item_count(&subtask("Exercise List", Some("12 exercises list"))),
        Some(12)
    );
}

#[test]
fn metadata_copy_requires_a_local_metadata_prefix() {
    for elements in [
        "meta row (45 min · 12 exercises · 320 kcal)",
        "metadata (12 exercises)",
        "Metadata text: (12 exercises, 45 minutes)",
        "信息行（共12项训练 · 45分钟）",
    ] {
        assert_eq!(
            expected_item_count(&subtask("Hero", Some(elements))),
            None,
            "{elements}"
        );
    }
    for elements in [
        "metadata then exercise section (12 exercises)",
        "meta row above; (12 exercises)",
        "meta row (12 exercises with thumbnail and name)",
        "信息行（列出12项训练，每项带图）",
    ] {
        assert_eq!(
            expected_item_count(&subtask("Hero", Some(elements))),
            Some(12),
            "{elements}"
        );
    }
}

// Exact header elements from the 1019 fix-arm app-18 rep-1 planning response:
// the meta row's "6 exercises" is copy describing the list section below.
#[test]
fn metadata_row_with_descriptor_tail_is_copy_not_an_item_promise() {
    let elements = "workout title 'Full-Body Strength Builder' 28px semibold two-line balanced, meta row of three inline items with 16px icons (32 min duration, 6 exercises, ~280 kcal), difficulty badges";
    let task = subtask("Workout Title, Meta and Difficulty Badges", Some(elements));
    assert_eq!(expected_item_count(&task), None);
    for elements in [
        "metadata then exercise section (12 exercises)",
        "meta row of stats; (12 exercises)",
        "meta row with icons (list of 12 exercises)",
    ] {
        assert_eq!(
            expected_item_count(&subtask("Hero", Some(elements))),
            Some(12),
            "{elements}"
        );
    }
}
