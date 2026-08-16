use serde_json::json;

use super::*;

#[test]
fn generated_nodes_reject_missing_progress_ring_without_auto_drawing() {
    let nodes: Vec<PenNode> = serde_json::from_value(json!([{
        "type": "frame", "id": "steps-ring", "name": "Steps Ring",
        "width": 124, "height": 124, "layout": "vertical",
        "alignItems": "center", "justifyContent": "center", "children": [
            {"type": "text", "id": "value", "content": "8,432"},
            {"type": "text", "id": "label", "content": "steps"}
        ]
    }]))
    .expect("parse nodes");

    let report = check_generated_nodes(&nodes, 390.0);

    assert!(report.has_fatal(), "missing ring must trigger retry");
    assert!(
        report.failure_message().contains("missing-progress-ring"),
        "{report:?}"
    );
}
#[test]
fn flags_mobile_product_row_that_will_clip_cards() {
    let nodes = json!([
        {
            "type": "frame",
            "id": "popular",
            "name": "Popular Now",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "horizontal",
            "gap": 20,
            "children": [
                {"type": "frame", "id": "card-1", "role": "card", "width": 170, "height": 220,
                 "children": [{"type": "image", "id": "img-1", "width": 170, "height": 120}, {"type": "text", "id": "t1", "content": "Truffle Carbonara"}]},
                {"type": "frame", "id": "card-2", "role": "card", "width": 170, "height": 220,
                 "children": [{"type": "image", "id": "img-2", "width": 170, "height": 120}, {"type": "text", "id": "t2", "content": "Smash Deluxe"}]},
                {"type": "frame", "id": "card-3", "role": "card", "width": 170, "height": 220,
                 "children": [{"type": "image", "id": "img-3", "width": 170, "height": 120}, {"type": "text", "id": "t3", "content": "Poke Salmon"}]}
            ]
        }
    ]);

    let report = check_value_forest(&nodes, 390.0);

    assert!(
        report.has_fatal(),
        "fixed-width mobile product row should be rejected before insertion"
    );
    assert!(
        report
            .failure_message()
            .contains("mobile-product-row-overflow"),
        "{report:?}"
    );
}

#[test]
fn clipping_scroll_row_is_not_flagged_as_overflow() {
    let nodes = json!([{
        "type": "frame", "id": "popular-scroll", "name": "Popular Scroll",
        "width": "fill_container", "height": "fit_content",
        "layout": "horizontal", "clipContent": true, "gap": 16,
        "children": [
            {"type": "frame", "id": "card-1", "role": "card", "width": 144, "height": 212,
             "children": [{"type": "image", "id": "img-1", "src": "", "width": 144, "height": 112}, {"type": "text", "id": "t1", "content": "Alpha"}]},
            {"type": "frame", "id": "card-2", "role": "card", "width": 144, "height": 212,
             "children": [{"type": "image", "id": "img-2", "src": "", "width": 144, "height": 112}, {"type": "text", "id": "t2", "content": "Beta"}]},
            {"type": "frame", "id": "card-3", "role": "card", "width": 144, "height": 212,
             "children": [{"type": "image", "id": "img-3", "src": "", "width": 144, "height": 112}, {"type": "text", "id": "t3", "content": "Gamma"}]},
            {"type": "frame", "id": "card-4", "role": "card", "width": 144, "height": 212,
             "children": [{"type": "image", "id": "img-4", "src": "", "width": 144, "height": 112}, {"type": "text", "id": "t4", "content": "Delta"}]}
        ]
    }]);

    assert!(!is_mobile_product_row_overflow(&nodes[0], 375.0));
    let parsed: Vec<PenNode> = serde_json::from_value(nodes).expect("parse nodes");
    let report = check_generated_nodes(&parsed, 375.0);
    assert!(
        !report.has_fatal(),
        "clipped scroll row should pass: {report:?}"
    );
}

#[test]
fn auto_fixes_mobile_product_row_overflow() {
    let mut nodes: Vec<PenNode> = serde_json::from_value(json!([
        {
            "type": "frame",
            "id": "popular",
            "name": "Popular Now",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "horizontal",
            "gap": 20,
            "children": [
                {"type": "frame", "id": "card-1", "role": "card", "width": 170, "height": 220,
                 "children": [{"type": "image", "id": "img-1", "src": "", "width": 170, "height": 120}, {"type": "text", "id": "t1", "content": "Truffle Carbonara"}]},
                {"type": "frame", "id": "card-2", "role": "card", "width": 170, "height": 220,
                 "children": [{"type": "image", "id": "img-2", "src": "", "width": 170, "height": 120}, {"type": "text", "id": "t2", "content": "Smash Deluxe"}]}
            ]
        }
    ]))
    .expect("parse nodes");

    let before = check_generated_nodes(&nodes, 390.0);
    assert!(
        before.has_fatal(),
        "fixed-width mobile product row should fail before auto-fix"
    );

    let fixed = auto_fix_fixable_issues(&mut nodes, 390.0);

    assert!(fixed, "overflowing product row should be auto-fixed");
    let fixed_json = serde_json::to_value(&nodes).expect("serialize nodes");
    let row = &fixed_json[0];
    assert_eq!(row["gap"].as_f64(), Some(12.0));
    assert_eq!(row["children"][0]["width"], json!("fill_container"));
    assert_eq!(row["children"][1]["width"], json!("fill_container"));
    let after = check_generated_nodes(&nodes, 390.0);
    assert!(
        !after.has_fatal(),
        "auto-fixed product row should pass: {after:?}"
    );
}

#[test]
fn accepts_two_equal_fill_product_cards() {
    let nodes = json!([
        {
            "type": "frame",
            "id": "popular",
            "name": "Popular Now",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "horizontal",
            "gap": 12,
            "children": [
                {"type": "frame", "id": "card-1", "role": "card", "width": "fill_container", "height": "fit_content",
                 "children": [{"type": "image", "id": "img-1", "width": "fill_container", "height": 120}, {"type": "text", "id": "t1", "content": "Truffle Carbonara"}]},
                {"type": "frame", "id": "card-2", "role": "card", "width": "fill_container", "height": "fit_content",
                 "children": [{"type": "image", "id": "img-2", "width": "fill_container", "height": 120}, {"type": "text", "id": "t2", "content": "Smash Deluxe"}]}
            ]
        }
    ]);

    let report = check_value_forest(&nodes, 390.0);

    assert!(
        !report.has_fatal(),
        "valid two-card mobile grid should pass: {report:?}"
    );
}

#[test]
fn allows_mobile_category_row_with_space_between() {
    // space_between is now the desired way to spread an under-filling chip
    // set across the row — it must NOT be flagged (user direction 2026-06-23).
    let nodes = json!([
        {
            "type": "frame",
            "id": "categories",
            "name": "Categories Section",
            "layout": "vertical",
            "children": [
                {"type": "text", "id": "heading", "content": "Categories"},
                {
                    "type": "frame",
                    "id": "options-row",
                    "name": "Options Row",
                    "layout": "horizontal",
                    "width": "fill_container",
                    "height": 96,
                    "justifyContent": "space_between",
                    "children": [
                        {"type": "frame", "id": "pizza", "layout": "vertical",
                         "children": [{"type": "icon_font", "iconFontName": "pizza"}, {"type": "text", "content": "Pizza"}]},
                        {"type": "frame", "id": "burger", "layout": "vertical",
                         "children": [{"type": "icon_font", "iconFontName": "hamburger"}, {"type": "text", "content": "Burger"}]}
                    ]
                }
            ]
        }
    ]);

    let report = check_value_forest(&nodes, 390.0);

    assert!(
        !report.has_fatal(),
        "space_between on a category row must be allowed now: {report:?}"
    );
}

#[test]
fn flags_mobile_category_row_that_overflows_or_is_too_tall() {
    // Genuinely broken spacing is still fatal: an over-tall row.
    let nodes = json!([
        {
            "type": "frame",
            "id": "categories",
            "name": "Categories Section",
            "layout": "vertical",
            "children": [
                {"type": "text", "id": "heading", "content": "Categories"},
                {
                    "type": "frame",
                    "id": "options-row",
                    "name": "Options Row",
                    "layout": "horizontal",
                    "width": "fill_container",
                    "height": 160,
                    "justifyContent": "start",
                    "children": [
                        {"type": "frame", "id": "pizza", "layout": "vertical",
                         "children": [{"type": "icon_font", "iconFontName": "pizza"}, {"type": "text", "content": "Pizza"}]},
                        {"type": "frame", "id": "burger", "layout": "vertical",
                         "children": [{"type": "icon_font", "iconFontName": "hamburger"}, {"type": "text", "content": "Burger"}]}
                    ]
                }
            ]
        }
    ]);

    let report = check_value_forest(&nodes, 390.0);

    assert!(
        report.has_fatal(),
        "an over-tall category row should still be rejected"
    );
    assert!(
        report
            .failure_message()
            .contains("mobile-category-row-loose-spacing"),
        "{report:?}"
    );
}

#[test]
fn flags_mobile_featured_food_card_with_blank_split() {
    let nodes = json!([
        {
            "type": "frame",
            "id": "featured-card",
            "name": "Featured Dish Card",
            "role": "card",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "horizontal",
            "gap": 0,
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {
                    "type": "frame",
                    "id": "featured-left",
                    "width": 122,
                    "height": "fit_content",
                    "layout": "vertical",
                    "children": [
                        {"type": "text", "id": "badge", "content": "Free delivery"},
                        {"type": "text", "id": "title", "content": "Truffle Tagliatelle"},
                        {"type": "text", "id": "price", "content": "$18.50"}
                    ]
                },
                {
                    "type": "image",
                    "id": "featured-photo",
                    "width": 170,
                    "height": 136,
                    "imageSearchQuery": "pasta plate"
                },
                {
                    "type": "frame",
                    "id": "plus-action",
                    "name": "Plus Action Button",
                    "role": "button",
                    "width": 64,
                    "height": 64,
                    "children": [{"type": "icon_font", "iconFontName": "plus"}]
                }
            ]
        }
    ]);

    let report = check_value_forest(&nodes, 390.0);

    assert!(
        report.has_fatal(),
        "bad split featured food card should be rejected before insertion"
    );
    assert!(
        report
            .failure_message()
            .contains("mobile-featured-card-bad-split"),
        "{report:?}"
    );
}

#[test]
fn accepts_mobile_featured_food_card_with_image_top() {
    let nodes = json!([
        {
            "type": "frame",
            "id": "featured-card",
            "name": "Featured Dish Card",
            "role": "card",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "vertical",
            "gap": 12,
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {
                    "type": "image",
                    "id": "featured-photo",
                    "width": "fill_container",
                    "height": 148,
                    "imageSearchQuery": "dumpling plate"
                },
                {
                    "type": "frame",
                    "id": "body",
                    "layout": "horizontal",
                    "children": [
                        {"type": "text", "id": "title", "content": "Dumpling House"},
                        {"type": "text", "id": "price", "content": "$18.50"}
                    ]
                }
            ]
        }
    ]);

    let report = check_value_forest(&nodes, 390.0);

    assert!(
        !report.has_fatal(),
        "valid image-top featured food card should pass: {report:?}"
    );
}

// ── Structural drift echo (DS P1.5) ──────────────────────────────────────────

/// The 0815 v4-pro card lesion: five "法则" items, five different internal
/// structures. No structure holds a 2/3 majority, so the family must be
/// echoed (never auto-fixed — structure is intent).
#[test]
fn five_same_stem_sections_with_five_structures_are_echoed() {
    let nodes = json!([
        {
            "type": "frame", "id": "card", "name": "知识卡片", "width": 1080, "height": 1440,
            "layout": "vertical",
            "children": [
                { "type": "frame", "id": "r1", "name": "法则 01", "layout": "vertical",
                  "children": [{ "type": "text", "id": "r1t", "content": "第一条" }] },
                { "type": "frame", "id": "r2", "name": "法则 02", "layout": "horizontal",
                  "children": [{ "type": "text", "id": "r2t", "content": "第二条" },
                               { "type": "frame", "id": "r2b", "layout": "vertical",
                                 "children": [{ "type": "text", "id": "r2bt", "content": "行" }] }] },
                { "type": "frame", "id": "r3", "name": "法则 03", "layout": "vertical",
                  "children": [{ "type": "rectangle", "id": "r3o" },
                               { "type": "text", "id": "r3t", "content": "第三条" }] },
                { "type": "frame", "id": "r4", "name": "法则 04", "layout": "vertical",
                  "children": [{ "type": "image", "id": "r4i" },
                               { "type": "text", "id": "r4t", "content": "第四条" }] },
                { "type": "frame", "id": "r5", "name": "法则 05", "layout": "vertical",
                  "children": [{ "type": "text", "id": "r5t", "content": "第五条" },
                               { "type": "text", "id": "r5b", "content": "尾注" }] }
            ]
        }
    ]);

    let report = check_value_forest(&nodes, 1080.0);

    assert!(
        report.has_fatal(),
        "five structures under one family must be echoed: {report:?}"
    );
    let message = report.failure_message();
    assert!(
        message.contains("section-structure-drift"),
        "echo must carry the drift code: {message}"
    );
    assert!(
        message.contains("unify them on ONE structure template"),
        "echo must demand one template: {message}"
    );
}

/// Drift detection also keys on a shared role when the names disagree
/// entirely (the second 0815 naming style: mixed 法则条目 / Section Rule /
/// Rule 03 Row names under one role).
#[test]
fn same_role_sections_with_drifted_names_are_echoed_by_role() {
    let nodes = json!([
        {
            "type": "frame", "id": "card", "name": "知识卡片", "width": 1080, "height": 1440,
            "layout": "vertical",
            "children": [
                { "type": "frame", "id": "r1", "name": "法则条目", "role": "section-rule",
                  "layout": "vertical",
                  "children": [{ "type": "text", "id": "r1t", "content": "A" }] },
                { "type": "frame", "id": "r2", "name": "Section Rule", "role": "section-rule",
                  "layout": "vertical",
                  "children": [{ "type": "text", "id": "r2t", "content": "B" },
                               { "type": "frame", "id": "r2b",
                                 "children": [{ "type": "text", "id": "r2bt", "content": "行" }] }] },
                { "type": "frame", "id": "r3", "name": "Rule 03 Row", "role": "section-rule",
                  "layout": "vertical",
                  "children": [{ "type": "rectangle", "id": "r3o" },
                               { "type": "text", "id": "r3t", "content": "C" }] }
            ]
        }
    ]);

    let report = check_value_forest(&nodes, 1080.0);
    assert!(
        report.has_fatal(),
        "role-grouped drift must be echoed: {report:?}"
    );
}

/// Isomorphic siblings (the family norm) are not drift — nothing to echo.
#[test]
fn isomorphic_sections_are_not_echoed() {
    let items: Vec<serde_json::Value> = [1, 2, 3, 4]
        .iter()
        .map(|i| {
            json!({
                "type": "frame", "id": format!("r{i}"), "name": format!("法则 0{i}"),
                "layout": "vertical",
                "children": [
                    { "type": "text", "id": format!("r{i}t"), "content": format!("第{i}条") }
                ]
            })
        })
        .collect();
    let nodes = json!([
        {
            "type": "frame", "id": "card", "name": "知识卡片", "width": 1080, "height": 1440,
            "layout": "vertical",
            "children": items
        }
    ]);

    let report = check_value_forest(&nodes, 1080.0);
    assert!(
        !report.has_fatal(),
        "one shared structure is the norm, not drift: {report:?}"
    );
}

/// The P1-a hero exemption: one differently-structured hero among a >= 2/3
/// isomorphic family is a deliberate first item, not drift.
#[test]
fn a_hero_among_isomorphic_siblings_is_not_echoed() {
    let nodes = json!([
        {
            "type": "frame", "id": "card", "name": "知识卡片", "width": 1080, "height": 1440,
            "layout": "vertical",
            "children": [
                { "type": "frame", "id": "hero", "name": "法则 00",
                  "layout": "vertical",
                  "children": [
                      { "type": "image", "id": "hero-img" },
                      { "type": "text", "id": "hero-t", "content": "总纲" },
                      { "type": "text", "id": "hero-b", "content": "导语" }
                  ] },
                { "type": "frame", "id": "r1", "name": "法则 01", "layout": "vertical",
                  "children": [{ "type": "text", "id": "r1t", "content": "一" }] },
                { "type": "frame", "id": "r2", "name": "法则 02", "layout": "vertical",
                  "children": [{ "type": "text", "id": "r2t", "content": "二" }] },
                { "type": "frame", "id": "r3", "name": "法则 03", "layout": "vertical",
                  "children": [{ "type": "text", "id": "r3t", "content": "三" }] },
                { "type": "frame", "id": "r4", "name": "法则 04", "layout": "vertical",
                  "children": [{ "type": "text", "id": "r4t", "content": "四" }] }
            ]
        }
    ]);

    let report = check_value_forest(&nodes, 1080.0);
    assert!(
        !report.has_fatal(),
        "a hero among a majority-consistent family is exempt: {report:?}"
    );
}
