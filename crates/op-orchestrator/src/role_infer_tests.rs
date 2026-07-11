use super::*;
use crate::role_defaults::{RoleCtx, Theme};
use op_editor_core::PenNodeExt;

/// Root context for the tree-walk tests (desktop width, light theme).
fn ctx() -> RoleCtx {
    RoleCtx::root(1200.0, Theme::Light)
}

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
        ("Bottom Navigation", "bottom-tab-bar"),
        ("Bottom Tab Bar", "bottom-tab-bar"),
        ("Tab Bar", "tab-bar"),
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
    resolve_tree_roles(&mut n, &ctx());
    assert_eq!(role_of(&n).as_deref(), Some("custom-thing"));
}

#[test]
fn inferred_role_is_written_back() {
    let mut n = frame("Navbar");
    resolve_tree_roles(&mut n, &ctx());
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
    resolve_tree_roles(&mut card, &ctx());
    // "Pricing Card" → `card` (the `\bcard\b` pattern wins over `pricing`).
    assert_eq!(role_of(&card).as_deref(), Some("card"));
    let inner = &card.children().unwrap()[0];
    assert_eq!(role_of(inner), None, "inner Header must not become navbar");
}

#[test]
fn page_chrome_role_kept_outside_card_parent() {
    // The same "Header" at the top level (no card parent) DOES become navbar.
    let mut n = frame("Header");
    resolve_tree_roles(&mut n, &ctx());
    assert_eq!(role_of(&n).as_deref(), Some("navbar"));
}

#[test]
fn card_role_stripped_on_tiny_node() {
    // 6×6 "Status Dot"-style node tripping /\bstat/ must not become stat-card.
    let mut tiny = node(serde_json::json!({
        "type":"frame","id":"d","name":"Stat Dot","width":6,"height":6,"children":[]
    }));
    resolve_tree_roles(&mut tiny, &ctx());
    assert_eq!(role_of(&tiny), None);
    // A normally-sized stat card keeps the role. ("Stat Panel", not "Stat
    // Card", so the `stat` pattern wins — `\bcard\b` would override to `card`.)
    let mut big = node(serde_json::json!({
        "type":"frame","id":"c","name":"Stat Panel","width":200,"height":120,"children":[]
    }));
    resolve_tree_roles(&mut big, &ctx());
    assert_eq!(role_of(&big).as_deref(), Some("stat-card"));
}

#[test]
fn section_wrapper_feature_card_role_stripped() {
    // tt5: "Categories & Featured Banner" hits `\bfeature` → feature-card, but
    // it's a top-level section WRAPPER holding 2 sub-frames (categories +
    // banner). Must drop the role so no card fill/border/radius boxes the
    // section (the "mysterious background" the user flagged).
    let mut sec = node(serde_json::json!({
        "type":"frame","id":"sec","name":"Categories & Featured Banner","children":[
            {"type":"frame","id":"chips","name":"Category Chips","children":[]},
            {"type":"frame","id":"banner","name":"Promo Banner","children":[]}
        ]
    }));
    resolve_tree_roles(&mut sec, &ctx());
    assert_eq!(
        role_of(&sec),
        None,
        "multi-block section wrapper must not be a feature-card"
    );
}

#[test]
fn section_wrapper_search_bar_role_stripped() {
    // tt5: "Location & Search" hits `\bsearch` → search-bar, but it's a header
    // wrapper (a Location sub-frame + the real search input). Drop the role so
    // the bg-deep fill + border + radius don't box the whole header.
    let mut sec = node(serde_json::json!({
        "type":"frame","id":"sec","name":"Location & Search","children":[
            {"type":"frame","id":"loc","name":"Location & Actions","children":[]},
            {"type":"text_input","id":"si","placeholder":"Search for food"}
        ]
    }));
    resolve_tree_roles(&mut sec, &ctx());
    assert_eq!(
        role_of(&sec),
        None,
        "header wrapper must not be a search-bar"
    );
}

#[test]
fn real_feature_card_with_leaf_content_keeps_role() {
    // Negative control: a genuine feature-card groups LEAF content (icon/title/
    // text), not sub-frames — the guard must leave its role + card fill intact.
    let mut card = node(serde_json::json!({
        "type":"frame","id":"fc","name":"Feature Highlight","children":[
            {"type":"icon_font","id":"ic","iconFontName":"zap"},
            {"type":"text","id":"t","name":"Title","content":"Fast"},
            {"type":"text","id":"d","name":"Desc","content":"Delivered hot"}
        ]
    }));
    resolve_tree_roles(&mut card, &ctx());
    assert_eq!(role_of(&card).as_deref(), Some("feature-card"));
}

#[test]
fn real_search_bar_with_icon_and_input_keeps_role() {
    // Negative control: a real search bar holds an icon + input (no sub-frame) —
    // keep search-bar so it still gets its pill fill.
    let mut bar = node(serde_json::json!({
        "type":"frame","id":"sb","name":"Search","children":[
            {"type":"icon_font","id":"m","iconFontName":"search"},
            {"type":"text_input","id":"si","placeholder":"Search"}
        ]
    }));
    resolve_tree_roles(&mut bar, &ctx());
    assert_eq!(role_of(&bar).as_deref(), Some("search-bar"));
}

#[test]
fn explicit_tiny_card_role_is_stripped() {
    // An LLM-emitted role:"stat-card" on a 6×6 frame must be STRIPPED (not just
    // inferred ones), so the I2 defaults pass never inflates a dot into a card.
    let mut tiny = node(serde_json::json!({
        "type":"frame","id":"d","name":"Indicator","role":"stat-card",
        "width":6,"height":6,"children":[]
    }));
    resolve_tree_roles(&mut tiny, &ctx());
    assert_eq!(
        role_of(&tiny),
        None,
        "explicit tiny card role must be stripped"
    );
    // …but an explicit non-card role on a tiny node is left alone.
    let mut dot = node(serde_json::json!({
        "type":"frame","id":"x","name":"Dot","role":"badge","width":6,"height":6,"children":[]
    }));
    resolve_tree_roles(&mut dot, &ctx());
    assert_eq!(role_of(&dot).as_deref(), Some("badge"));
}

#[test]
fn forest_helper_resolves_each_root() {
    let mut forest = vec![frame("Navbar"), frame("Footer")];
    resolve_forest_roles(&mut forest, 1200.0, Theme::Light);
    assert_eq!(role_of(&forest[0]).as_deref(), Some("navbar"));
    assert_eq!(role_of(&forest[1]).as_deref(), Some("footer"));
}

#[test]
fn chart_names_infer_the_chart_role_including_cjk() {
    // The chart role is the STRUCTURAL signal downstream guards key off
    // (chart marks must not get card borders) — it must fire for English
    // and CJK chart names alike, and NOT swallow chart-card surfaces.
    let mut forest = vec![
        frame("Weekly Workout Chart"),
        frame("Revenue Graph"),
        frame("心率走势图"),
        frame("Heart Rate Chart Card"),
    ];
    resolve_forest_roles(&mut forest, 1200.0, Theme::Light);
    assert_eq!(role_of(&forest[0]).as_deref(), Some("chart"));
    assert_eq!(role_of(&forest[1]).as_deref(), Some("chart"));
    assert_eq!(role_of(&forest[2]).as_deref(), Some("chart"));
    assert_eq!(
        role_of(&forest[3]).as_deref(),
        Some("card"),
        "a chart CARD is a card surface first (guards still see 'chart' in its name)"
    );
}
