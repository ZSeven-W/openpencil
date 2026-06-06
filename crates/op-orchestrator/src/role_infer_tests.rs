use super::*;
use op_editor_core::PenNodeExt;

/// Build a frame `PenNode` from a JSON spec (easiest faithful construction).
fn node(value: serde_json::Value) -> PenNode {
    serde_json::from_value(value).expect("valid PenNode")
}

fn frame(name: &str) -> PenNode {
    node(serde_json::json!({"type":"frame","id":"x","name":name,"children":[]}))
}

fn role_of(n: &PenNode) -> Option<String> {
    n.base().role.clone()
}

// ── infer_role_from_name: exact map ───────────────────────────────────────

#[test]
fn exact_map_infers_role() {
    for (name, expected) in [
        ("Header", "navbar"),
        ("Navigation Bar", "navbar"),
        ("Footer", "footer"),
        ("Hero Section", "hero"),
        ("Search", "search-bar"),
        ("Avatar", "avatar"),
        ("Divider", "divider"),
        ("Table", "table"),
    ] {
        assert_eq!(
            infer_role_from_name(&frame(name)),
            Some(expected),
            "name {name:?}"
        );
    }
}

#[test]
fn exact_map_is_case_insensitive() {
    assert_eq!(infer_role_from_name(&frame("NAVBAR")), Some("navbar"));
    assert_eq!(infer_role_from_name(&frame("  footer  ")), Some("footer"));
}

// ── infer_role_from_name: pattern map ─────────────────────────────────────

#[test]
fn pattern_map_infers_role() {
    for (name, expected) in [
        ("Submit Button", "button"),
        // `\bcard\b` is matched BEFORE `pricing`/`stat`/`feature` (TS order),
        // so any name containing the word "card" resolves to plain `card`.
        ("Pricing Card", "card"),
        ("Pricing Plan", "pricing-card"),
        ("Email Input", "input"),
        ("Text Field", "input"),
        ("Signup Form", "form-group"),
        ("Stat Block", "stat-card"),
        ("Feature Highlight", "feature-card"),
        ("Customer Testimonial", "testimonial"),
    ] {
        assert_eq!(
            infer_role_from_name(&frame(name)),
            Some(expected),
            "name {name:?}"
        );
    }
}

#[test]
fn container_suffix_is_not_an_instance() {
    // "Button Group" / "Card List" are containers, not a button / card.
    assert_eq!(infer_role_from_name(&frame("Button Group")), None);
    assert_eq!(infer_role_from_name(&frame("Card List")), None);
}

#[test]
fn role_part_word_after_keyword_is_a_piece() {
    // "Card Header" is a piece of a card, not a card.
    assert_eq!(infer_role_from_name(&frame("Card Header")), None);
    assert_eq!(infer_role_from_name(&frame("Button Label")), None);
    // …but a part word BEFORE the keyword keeps the role ("Icon Button").
    assert_eq!(infer_role_from_name(&frame("Icon Button")), Some("button"));
    // …and a prepositional variant keeps it ("Card with Icon").
    assert_eq!(infer_role_from_name(&frame("Card with Icon")), Some("card"));
    // numeric index between keyword and part word still detected.
    assert_eq!(infer_role_from_name(&frame("Card 1 Content")), None);
}

#[test]
fn non_frame_and_unnamed_infer_nothing() {
    let text = node(serde_json::json!({"type":"text","id":"t","name":"Header","content":"x"}));
    assert_eq!(infer_role_from_name(&text), None);
    let unnamed = node(serde_json::json!({"type":"frame","id":"f","children":[]}));
    assert_eq!(infer_role_from_name(&unnamed), None);
}

// ── resolve_tree_roles: write-back + guards ───────────────────────────────

#[test]
fn explicit_role_is_never_overwritten() {
    let mut n = node(serde_json::json!({
        "type":"frame","id":"x","name":"Header","role":"custom-thing","children":[]
    }));
    resolve_tree_roles(&mut n, None);
    assert_eq!(role_of(&n).as_deref(), Some("custom-thing"));
}

#[test]
fn inferred_role_is_written_back() {
    let mut n = frame("Navbar");
    resolve_tree_roles(&mut n, None);
    assert_eq!(role_of(&n).as_deref(), Some("navbar"));
}

#[test]
fn page_chrome_role_stripped_inside_card_parent() {
    // A pricing-card whose inner section is named "Header" must NOT become a
    // navbar (else cleanup paints a nav fill on a card's title row).
    let mut card = node(serde_json::json!({
        "type":"frame","id":"card","name":"Pricing Card","children":[
            {"type":"frame","id":"hd","name":"Header","children":[]}
        ]
    }));
    resolve_tree_roles(&mut card, None);
    // "Pricing Card" → `card` (the `\bcard\b` pattern wins over `pricing`).
    assert_eq!(role_of(&card).as_deref(), Some("card"));
    let inner = &card.children().unwrap()[0];
    assert_eq!(role_of(inner), None, "inner Header must not become navbar");
}

#[test]
fn page_chrome_role_kept_outside_card_parent() {
    // The same "Header" at the top level (no card parent) DOES become navbar.
    let mut n = frame("Header");
    resolve_tree_roles(&mut n, None);
    assert_eq!(role_of(&n).as_deref(), Some("navbar"));
}

#[test]
fn card_role_stripped_on_tiny_node() {
    // 6×6 "Status Dot"-style node tripping /\bstat/ must not become stat-card.
    let mut tiny = node(serde_json::json!({
        "type":"frame","id":"d","name":"Stat Dot","width":6,"height":6,"children":[]
    }));
    resolve_tree_roles(&mut tiny, None);
    assert_eq!(role_of(&tiny), None);
    // A normally-sized stat card keeps the role. ("Stat Panel", not "Stat
    // Card", so the `stat` pattern wins — `\bcard\b` would override to `card`.)
    let mut big = node(serde_json::json!({
        "type":"frame","id":"c","name":"Stat Panel","width":200,"height":120,"children":[]
    }));
    resolve_tree_roles(&mut big, None);
    assert_eq!(role_of(&big).as_deref(), Some("stat-card"));
}

#[test]
fn forest_helper_resolves_each_root() {
    let mut forest = vec![frame("Navbar"), frame("Footer")];
    resolve_forest_roles(&mut forest);
    assert_eq!(role_of(&forest[0]).as_deref(), Some("navbar"));
    assert_eq!(role_of(&forest[1]).as_deref(), Some("footer"));
}
