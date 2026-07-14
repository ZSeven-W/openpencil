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
