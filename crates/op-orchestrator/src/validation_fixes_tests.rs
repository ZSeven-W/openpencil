//! Tests for `validation_fixes` — split out to honour the 800-line ceiling.
//!
//! Loaded via `#[cfg(test)] #[path = "validation_fixes_tests.rs"] mod tests;`
//! in `validation_fixes.rs`, so this stays nested as `validation_fixes::tests`.

use super::*;
use serde_json::json;

// ── B1 Step 1: all 17 properties present with right kind ─────────────────

fn find_kind(name: &str) -> Option<ValueKind> {
    property_kind(name)
}

#[test]
fn safe_fix_properties_has_17_entries() {
    assert_eq!(
        SAFE_FIX_PROPERTIES.len(),
        17,
        "expected exactly 17 whitelisted properties"
    );
}

#[test]
fn all_17_property_names_present() {
    let expected = [
        "width",
        "height",
        "padding",
        "gap",
        "fontSize",
        "fontWeight",
        "letterSpacing",
        "lineHeight",
        "cornerRadius",
        "opacity",
        "fillColor",
        "strokeColor",
        "strokeWidth",
        "textAlign",
        "textGrowth",
        "alignItems",
        "justifyContent",
    ];
    for name in &expected {
        assert!(
            find_kind(name).is_some(),
            "property '{}' missing from SAFE_FIX_PROPERTIES",
            name
        );
    }
}

#[test]
fn property_kinds_match_ts() {
    assert_eq!(find_kind("width"), Some(ValueKind::Sizing));
    assert_eq!(find_kind("height"), Some(ValueKind::Sizing));
    assert_eq!(find_kind("padding"), Some(ValueKind::NumberOrArray));
    assert_eq!(find_kind("gap"), Some(ValueKind::Number));
    assert_eq!(find_kind("fontSize"), Some(ValueKind::Number));
    assert_eq!(find_kind("fontWeight"), Some(ValueKind::FontWeight));
    assert_eq!(find_kind("letterSpacing"), Some(ValueKind::Number));
    assert_eq!(find_kind("lineHeight"), Some(ValueKind::Number));
    assert_eq!(find_kind("cornerRadius"), Some(ValueKind::Number));
    assert_eq!(find_kind("opacity"), Some(ValueKind::Number));
    assert_eq!(find_kind("fillColor"), Some(ValueKind::Color));
    assert_eq!(find_kind("strokeColor"), Some(ValueKind::Color));
    assert_eq!(find_kind("strokeWidth"), Some(ValueKind::Number));
    assert_eq!(find_kind("textAlign"), Some(ValueKind::EnumTextAlign));
    assert_eq!(find_kind("textGrowth"), Some(ValueKind::EnumTextGrowth));
    assert_eq!(find_kind("alignItems"), Some(ValueKind::EnumAlign));
    assert_eq!(find_kind("justifyContent"), Some(ValueKind::EnumJustify));
}

#[test]
fn unknown_property_returns_none_kind() {
    assert_eq!(find_kind("notAProperty"), None);
    assert_eq!(find_kind(""), None);
}

// ── B1 Step 1: is_valid_fix_value — Number kind ───────────────────────────

#[test]
fn number_kind_accepts_positive() {
    assert!(is_valid_fix_value("gap", &json!(16)));
    assert!(is_valid_fix_value("fontSize", &json!(14.5)));
    assert!(is_valid_fix_value("letterSpacing", &json!(1)));
    assert!(is_valid_fix_value("lineHeight", &json!(1.5)));
    assert!(is_valid_fix_value("cornerRadius", &json!(8)));
    assert!(is_valid_fix_value("opacity", &json!(0.5)));
    assert!(is_valid_fix_value("strokeWidth", &json!(2)));
}

#[test]
fn number_kind_accepts_zero() {
    assert!(is_valid_fix_value("gap", &json!(0)));
}

#[test]
fn number_kind_rejects_string() {
    assert!(!is_valid_fix_value("gap", &json!("16")));
    assert!(!is_valid_fix_value("fontSize", &json!("14px")));
}

#[test]
fn number_kind_rejects_null_and_bool() {
    assert!(!is_valid_fix_value("gap", &json!(null)));
    assert!(!is_valid_fix_value("opacity", &json!(true)));
}

// ── B1 Step 1: is_valid_fix_value — Sizing kind ───────────────────────────

#[test]
fn sizing_kind_accepts_positive_number() {
    assert!(is_valid_fix_value("width", &json!(100)));
    assert!(is_valid_fix_value("height", &json!(48.5)));
}

#[test]
fn sizing_kind_accepts_zero() {
    assert!(is_valid_fix_value("width", &json!(0)));
}

#[test]
fn sizing_kind_accepts_keywords() {
    assert!(is_valid_fix_value("width", &json!("fill_container")));
    assert!(is_valid_fix_value("height", &json!("fit_content")));
    assert!(is_valid_fix_value("width", &json!("fit_content")));
    assert!(is_valid_fix_value("height", &json!("fill_container")));
}

#[test]
fn sizing_kind_rejects_unknown_string() {
    assert!(!is_valid_fix_value("width", &json!("auto")));
    assert!(!is_valid_fix_value("height", &json!("100%")));
}

#[test]
fn sizing_kind_rejects_negative() {
    // TS: `typeof value === 'number'` passes negative — we match faithfully.
    // Actually TS doesn't restrict sign for sizing numbers; let's pass negatives.
    // Negative numbers are JSON numbers, so they pass `is_number()`.
    assert!(is_valid_fix_value("width", &json!(-1)));
}

#[test]
fn sizing_kind_rejects_array() {
    assert!(!is_valid_fix_value("width", &json!([100, 200])));
}

// ── B1 Step 1: is_valid_fix_value — NumberOrArray kind ────────────────────

#[test]
fn number_or_array_accepts_number() {
    assert!(is_valid_fix_value("padding", &json!(16)));
    assert!(is_valid_fix_value("padding", &json!(0)));
}

#[test]
fn number_or_array_accepts_number_array() {
    assert!(is_valid_fix_value("padding", &json!([8, 16])));
    assert!(is_valid_fix_value("padding", &json!([4, 8, 4, 8])));
    assert!(is_valid_fix_value("padding", &json!([0])));
}

#[test]
fn number_or_array_rejects_string_array() {
    assert!(!is_valid_fix_value("padding", &json!(["8px", "16px"])));
}

#[test]
fn number_or_array_rejects_mixed_array() {
    assert!(!is_valid_fix_value("padding", &json!([8, "16px"])));
}

#[test]
fn number_or_array_rejects_string() {
    assert!(!is_valid_fix_value("padding", &json!("16")));
}

#[test]
fn number_or_array_rejects_empty_array() {
    // TS: `value.every(...)` returns `true` for empty arrays → passes.
    assert!(is_valid_fix_value("padding", &json!([])));
}

// ── B1 Step 1: is_valid_fix_value — FontWeight kind ───────────────────────

#[test]
fn font_weight_accepts_valid_multiples() {
    for w in [100u64, 200, 300, 400, 500, 600, 700, 800, 900] {
        assert!(
            is_valid_fix_value("fontWeight", &json!(w)),
            "fontWeight {w} should be valid"
        );
    }
}

#[test]
fn font_weight_rejects_non_multiple() {
    assert!(!is_valid_fix_value("fontWeight", &json!(750)));
    assert!(!is_valid_fix_value("fontWeight", &json!(0)));
    assert!(!is_valid_fix_value("fontWeight", &json!(1000)));
    assert!(!is_valid_fix_value("fontWeight", &json!(150)));
}

#[test]
fn font_weight_rejects_string() {
    assert!(!is_valid_fix_value("fontWeight", &json!("bold")));
    assert!(!is_valid_fix_value("fontWeight", &json!("700")));
}

#[test]
fn font_weight_rejects_float() {
    // JSON floats like 700.5 won't parse as u64, so rejected.
    assert!(!is_valid_fix_value("fontWeight", &json!(700.5)));
}

// ── B1 Step 1: is_valid_fix_value — Color kind ────────────────────────────

#[test]
fn color_accepts_6digit_hex() {
    assert!(is_valid_fix_value("fillColor", &json!("#0F172A")));
    assert!(is_valid_fix_value("strokeColor", &json!("#ffffff")));
    assert!(is_valid_fix_value("fillColor", &json!("#AABBCC")));
}

#[test]
fn color_accepts_3digit_hex() {
    assert!(is_valid_fix_value("fillColor", &json!("#FFF")));
    assert!(is_valid_fix_value("fillColor", &json!("#abc")));
}

#[test]
fn color_accepts_4digit_hex() {
    assert!(is_valid_fix_value("fillColor", &json!("#FFFF")));
    assert!(is_valid_fix_value("fillColor", &json!("#0000")));
}

#[test]
fn color_accepts_8digit_hex() {
    assert!(is_valid_fix_value("fillColor", &json!("#0F172AFF")));
    assert!(is_valid_fix_value("fillColor", &json!("#ffffffff")));
}

#[test]
fn color_rejects_named_color() {
    assert!(!is_valid_fix_value("fillColor", &json!("blue")));
    assert!(!is_valid_fix_value("fillColor", &json!("red")));
    assert!(!is_valid_fix_value("fillColor", &json!("transparent")));
}

#[test]
fn color_rejects_no_hash() {
    assert!(!is_valid_fix_value("fillColor", &json!("0F172A")));
    assert!(!is_valid_fix_value("fillColor", &json!("FFFFFF")));
}

#[test]
fn color_rejects_wrong_length() {
    // 5 digits (not 3/4/6/8)
    assert!(!is_valid_fix_value("fillColor", &json!("#FFFFF")));
    // 7 digits
    assert!(!is_valid_fix_value("fillColor", &json!("#FFFFFFF")));
}

#[test]
fn color_rejects_non_hex_chars() {
    assert!(!is_valid_fix_value("fillColor", &json!("#GGHHII")));
    assert!(!is_valid_fix_value("fillColor", &json!("#XYZ")));
}

#[test]
fn color_rejects_number() {
    assert!(!is_valid_fix_value("fillColor", &json!(0xffffff)));
}

// ── B1 Step 1: is_valid_fix_value — EnumTextAlign kind ────────────────────

#[test]
fn text_align_accepts_valid() {
    assert!(is_valid_fix_value("textAlign", &json!("left")));
    assert!(is_valid_fix_value("textAlign", &json!("center")));
    assert!(is_valid_fix_value("textAlign", &json!("right")));
}

#[test]
fn text_align_rejects_invalid() {
    assert!(!is_valid_fix_value("textAlign", &json!("justify")));
    assert!(!is_valid_fix_value("textAlign", &json!("start")));
    assert!(!is_valid_fix_value("textAlign", &json!("end")));
    assert!(!is_valid_fix_value("textAlign", &json!("")));
}

// ── B1 Step 1: is_valid_fix_value — EnumTextGrowth kind ───────────────────

#[test]
fn text_growth_accepts_valid() {
    assert!(is_valid_fix_value("textGrowth", &json!("auto")));
    assert!(is_valid_fix_value("textGrowth", &json!("fixed-width")));
    assert!(is_valid_fix_value(
        "textGrowth",
        &json!("fixed-width-height")
    ));
}

#[test]
fn text_growth_rejects_invalid() {
    assert!(!is_valid_fix_value("textGrowth", &json!("fixed")));
    assert!(!is_valid_fix_value("textGrowth", &json!("fill")));
    assert!(!is_valid_fix_value("textGrowth", &json!("")));
}

// ── B1 Step 1: is_valid_fix_value — EnumAlign kind ────────────────────────

#[test]
fn align_items_accepts_valid() {
    assert!(is_valid_fix_value("alignItems", &json!("start")));
    assert!(is_valid_fix_value("alignItems", &json!("center")));
    assert!(is_valid_fix_value("alignItems", &json!("end")));
}

#[test]
fn align_items_rejects_invalid() {
    assert!(!is_valid_fix_value("alignItems", &json!("left")));
    assert!(!is_valid_fix_value("alignItems", &json!("stretch")));
    assert!(!is_valid_fix_value("alignItems", &json!("space_between")));
    assert!(!is_valid_fix_value("alignItems", &json!("")));
}

// ── B1 Step 1: is_valid_fix_value — EnumJustify kind ──────────────────────

#[test]
fn justify_content_accepts_valid() {
    assert!(is_valid_fix_value("justifyContent", &json!("start")));
    assert!(is_valid_fix_value("justifyContent", &json!("center")));
    assert!(is_valid_fix_value("justifyContent", &json!("end")));
    assert!(is_valid_fix_value(
        "justifyContent",
        &json!("space_between")
    ));
    assert!(is_valid_fix_value("justifyContent", &json!("space_around")));
}

#[test]
fn justify_content_rejects_invalid() {
    assert!(!is_valid_fix_value("justifyContent", &json!("flex-start")));
    assert!(!is_valid_fix_value(
        "justifyContent",
        &json!("space-between")
    ));
    assert!(!is_valid_fix_value("justifyContent", &json!("stretch")));
    assert!(!is_valid_fix_value("justifyContent", &json!("")));
}

// ── B1 Step 1: is_valid_fix_value — unknown property ──────────────────────

#[test]
fn unknown_property_always_returns_false() {
    assert!(!is_valid_fix_value("notAProperty", &json!(100)));
    assert!(!is_valid_fix_value("background", &json!("#FFF")));
    assert!(!is_valid_fix_value("", &json!(0)));
}

// ── B1 Step 1: is_valid_structural_fix — addChild ─────────────────────────

#[test]
fn structural_fix_add_child_valid_frame() {
    let fix = json!({
        "action": "addChild",
        "parentId": "frame-abc",
        "node": { "type": "frame", "name": "Card" }
    });
    assert!(is_valid_structural_fix(&fix));
}

#[test]
fn structural_fix_add_child_valid_all_node_types() {
    for node_type in ["frame", "text", "path", "rectangle", "ellipse"] {
        let fix = json!({
            "action": "addChild",
            "parentId": "parent-1",
            "node": { "type": node_type }
        });
        assert!(
            is_valid_structural_fix(&fix),
            "addChild with node.type='{}' should be valid",
            node_type
        );
    }
}

#[test]
fn structural_fix_add_child_with_optional_index() {
    let fix = json!({
        "action": "addChild",
        "parentId": "frame-abc",
        "index": 2,
        "node": { "type": "text", "content": "Hello" }
    });
    assert!(is_valid_structural_fix(&fix));
}

#[test]
fn structural_fix_add_child_rejects_empty_parent_id() {
    let fix = json!({
        "action": "addChild",
        "parentId": "",
        "node": { "type": "frame" }
    });
    assert!(!is_valid_structural_fix(&fix));
}

#[test]
fn structural_fix_add_child_rejects_missing_parent_id() {
    let fix = json!({
        "action": "addChild",
        "node": { "type": "frame" }
    });
    assert!(!is_valid_structural_fix(&fix));
}

#[test]
fn structural_fix_add_child_rejects_invalid_node_type() {
    let fix = json!({
        "action": "addChild",
        "parentId": "frame-abc",
        "node": { "type": "circle" }
    });
    assert!(!is_valid_structural_fix(&fix));
}

#[test]
fn structural_fix_add_child_rejects_missing_node() {
    let fix = json!({
        "action": "addChild",
        "parentId": "frame-abc"
    });
    assert!(!is_valid_structural_fix(&fix));
}

#[test]
fn structural_fix_add_child_rejects_node_without_type() {
    let fix = json!({
        "action": "addChild",
        "parentId": "frame-abc",
        "node": { "name": "Card" }
    });
    assert!(!is_valid_structural_fix(&fix));
}

// ── B1 Step 1: is_valid_structural_fix — removeNode ───────────────────────

#[test]
fn structural_fix_remove_node_valid() {
    let fix = json!({
        "action": "removeNode",
        "nodeId": "node-xyz"
    });
    assert!(is_valid_structural_fix(&fix));
}

#[test]
fn structural_fix_remove_node_rejects_empty_id() {
    let fix = json!({
        "action": "removeNode",
        "nodeId": ""
    });
    assert!(!is_valid_structural_fix(&fix));
}

#[test]
fn structural_fix_remove_node_rejects_missing_id() {
    let fix = json!({
        "action": "removeNode"
    });
    assert!(!is_valid_structural_fix(&fix));
}

// ── B1 Step 1: is_valid_structural_fix — unknown action / non-object ──────

#[test]
fn structural_fix_rejects_unknown_action() {
    let fix = json!({ "action": "moveNode", "nodeId": "abc" });
    assert!(!is_valid_structural_fix(&fix));
}

#[test]
fn structural_fix_rejects_missing_action() {
    let fix = json!({ "nodeId": "abc" });
    assert!(!is_valid_structural_fix(&fix));
}

#[test]
fn structural_fix_rejects_non_object() {
    assert!(!is_valid_structural_fix(&json!("addChild")));
    assert!(!is_valid_structural_fix(&json!(null)));
    assert!(!is_valid_structural_fix(&json!(42)));
    assert!(!is_valid_structural_fix(&json!([])));
}

// ── Hex color helper standalone tests ─────────────────────────────────────

#[test]
fn hex_color_3_digits() {
    assert!(is_valid_hex_color("#FFF"));
    assert!(is_valid_hex_color("#abc"));
    assert!(is_valid_hex_color("#000"));
}

#[test]
fn hex_color_4_digits() {
    assert!(is_valid_hex_color("#FFFF"));
    assert!(is_valid_hex_color("#abcd"));
}

#[test]
fn hex_color_6_digits() {
    assert!(is_valid_hex_color("#FFFFFF"));
    assert!(is_valid_hex_color("#0f172a"));
}

#[test]
fn hex_color_8_digits() {
    assert!(is_valid_hex_color("#FFFFFFFF"));
    assert!(is_valid_hex_color("#0f172aff"));
}

#[test]
fn hex_color_rejects_5_digits() {
    assert!(!is_valid_hex_color("#FFFFF"));
}

#[test]
fn hex_color_rejects_7_digits() {
    assert!(!is_valid_hex_color("#FFFFFFF"));
}

#[test]
fn hex_color_rejects_no_hash() {
    assert!(!is_valid_hex_color("FFFFFF"));
}

#[test]
fn hex_color_rejects_non_hex() {
    assert!(!is_valid_hex_color("#GGHHII"));
    assert!(!is_valid_hex_color("#zzzzzz"));
}

#[test]
fn hex_color_rejects_empty() {
    assert!(!is_valid_hex_color(""));
    assert!(!is_valid_hex_color("#"));
}
