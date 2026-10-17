use super::*;
use serde_json::json;

fn task(elements: &str) -> Subtask {
    serde_json::from_value(json!({
        "id":"items","label":"Exercise List","elements":elements,
        "region":{"width":375,"height":400}
    }))
    .unwrap()
}

#[test]
fn completeness_recognizes_number_words_only_in_explicit_list_count_syntax() {
    for (words, count) in [("two", 2), ("six", 6), ("twelve", 12)] {
        assert_eq!(
            expected_item_count(&task(&format!("scrollable list of {words} exercise rows"))),
            Some(count)
        );
    }
    for elements in [
        "Hero with a title 'list of six exercises'",
        "meta row (list count: six exercises)",
        "sixteen exercise rows",
        "list of sixteen exercise rows",
        "six reps and twelve minutes",
    ] {
        assert_eq!(expected_item_count(&task(elements)), None, "{elements}");
    }
    assert_eq!(
        expected_item_count(&task("Heading 'Exercises (12)', list of six exercise rows")),
        Some(6)
    );
}
