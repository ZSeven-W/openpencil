//! Tests for the cross-detector node accessors in `node_util.rs`,
//! carved off into a sibling file to keep both under the 800-line cap.
//! Wired in as `node_util::tests` via `#[path]` so `use super::*` still
//! resolves against `node_util` itself.

use super::*;
use serde_json::json;

/// Deserialize a JSON value into a `PenNode` — doubles as a schema
/// round-trip check for every fixture below.
fn node(value: serde_json::Value) -> PenNode {
    serde_json::from_value(value).expect("fixture must deserialize as PenNode")
}

#[test]
fn node_id_and_kind_accessors() {
    let n = node(json!({"type": "frame", "id": "f1"}));
    assert_eq!(node_id(&n), "f1");
    assert_eq!(node_kind(&n), NodeKind::Frame);
    assert_eq!(node_kind_str(&n), "frame");

    let t = node(json!({"type": "text", "id": "t1", "content": "hi"}));
    assert_eq!(node_kind(&t), NodeKind::Text);
    assert_eq!(node_kind_str(&t), "text");

    let ico = node(json!({"type": "icon_font", "id": "i1", "iconFontName": "star"}));
    assert_eq!(node_kind_str(&ico), "icon_font");
}

#[test]
fn children_returns_slice_for_containers_and_empty_for_leaves() {
    let frame = node(json!({
        "type": "frame",
        "id": "root",
        "children": [
            {"type": "text", "id": "t1", "content": "a"},
            {"type": "text", "id": "t2", "content": "b"}
        ]
    }));
    assert_eq!(children(&frame).len(), 2);
    assert_eq!(node_id(&children(&frame)[0]), "t1");

    // A container with no `children` field → empty slice.
    let empty_frame = node(json!({"type": "frame", "id": "f"}));
    assert!(children(&empty_frame).is_empty());

    // A leaf kind → empty slice.
    let text = node(json!({"type": "text", "id": "t", "content": "x"}));
    assert!(children(&text).is_empty());
}

#[test]
fn role_rotation_opacity_accessors() {
    let n = node(json!({
        "type": "frame", "id": "f", "role": "card",
        "rotation": 12.5, "opacity": 0.5
    }));
    assert_eq!(role(&n), Some("card"));
    assert_eq!(rotation(&n), 12.5);
    assert_eq!(opacity(&n), 0.5);

    // Missing rotation → 0.0; missing opacity → 1.0; missing role → None.
    let bare = node(json!({"type": "frame", "id": "b"}));
    assert_eq!(role(&bare), None);
    assert_eq!(rotation(&bare), 0.0);
    assert_eq!(opacity(&bare), 1.0);

    // An opacity expression resolves to the opaque default.
    let expr = node(json!({"type": "frame", "id": "e", "opacity": "$alpha"}));
    assert_eq!(opacity(&expr), 1.0);
}

#[test]
fn first_fill_color_on_solid_filled_frame() {
    let filled = node(json!({
        "type": "frame", "id": "f",
        "fill": [
            {"type": "solid", "color": "#FF0000"},
            {"type": "solid", "color": "#00FF00"}
        ]
    }));
    // First fill's color wins even though a later fill exists.
    assert_eq!(first_fill_color(&filled), Some("#FF0000"));
}

#[test]
fn first_fill_color_none_for_unfilled_node() {
    // No `fill` field at all.
    let bare = node(json!({"type": "frame", "id": "f"}));
    assert_eq!(first_fill_color(&bare), None);

    // Empty `fill` array.
    let empty = node(json!({"type": "frame", "id": "f", "fill": []}));
    assert_eq!(first_fill_color(&empty), None);

    // First fill is an image fill → no color.
    let img = node(json!({
        "type": "frame", "id": "f",
        "fill": [{"type": "image", "url": "x.png"}]
    }));
    assert_eq!(first_fill_color(&img), None);

    // A solid fill with an empty-string color → None (TS `first.color &&`).
    let empty_color = node(json!({
        "type": "frame", "id": "f",
        "fill": [{"type": "solid", "color": ""}]
    }));
    assert_eq!(first_fill_color(&empty_color), None);
}

#[test]
fn has_stroke_true_for_positive_thickness() {
    let uniform = node(json!({
        "type": "frame", "id": "f",
        "stroke": {"thickness": 2.0}
    }));
    assert!(has_stroke(&uniform));

    let per_side = node(json!({
        "type": "frame", "id": "f",
        "stroke": {"thickness": [0.0, 0.0, 3.0, 0.0]}
    }));
    assert!(has_stroke(&per_side));

    let sided = node(json!({
        "type": "frame", "id": "f",
        "stroke": {"thickness": {"bottom": 1.5}}
    }));
    assert!(has_stroke(&sided));
}

#[test]
fn has_stroke_false_for_missing_or_zero_stroke() {
    let no_stroke = node(json!({"type": "frame", "id": "f"}));
    assert!(!has_stroke(&no_stroke));

    let zero = node(json!({
        "type": "frame", "id": "f",
        "stroke": {"thickness": 0.0}
    }));
    assert!(!has_stroke(&zero));

    // A node kind that cannot carry a stroke (text) is always false.
    let text = node(json!({"type": "text", "id": "t", "content": "x"}));
    assert!(!has_stroke(&text));
}

#[test]
fn is_node_visible_for_visible_false_node() {
    let hidden = node(json!({"type": "frame", "id": "f", "visible": false}));
    assert!(!is_node_visible(&hidden));
}

#[test]
fn is_node_visible_for_enabled_false_node() {
    let disabled = node(json!({"type": "frame", "id": "f", "enabled": false}));
    assert!(!is_node_visible(&disabled));
}

#[test]
fn is_node_visible_for_normal_node() {
    // No visible/enabled fields → visible.
    let bare = node(json!({"type": "frame", "id": "f"}));
    assert!(is_node_visible(&bare));

    // Explicit visible:true, enabled:true → visible.
    let explicit = node(json!({
        "type": "frame", "id": "f", "visible": true, "enabled": true
    }));
    assert!(is_node_visible(&explicit));

    // An `enabled` expression is treated as visible (matches TS `!== false`).
    let expr = node(json!({"type": "frame", "id": "f", "enabled": "$canEdit"}));
    assert!(is_node_visible(&expr));
}

#[test]
fn resolve_color_ref_returns_literal_unchanged() {
    let vars = Variables::new();
    let theme = Theme::new();
    assert_eq!(
        resolve_color_ref("#3366FF", &vars, &theme),
        Some("#3366FF".to_string())
    );
}

#[test]
fn resolve_color_ref_resolves_known_scalar_ref() {
    let mut vars = Variables::new();
    vars.insert(
        "--primary".to_string(),
        serde_json::from_value(json!({"type": "color", "value": "#112233"})).unwrap(),
    );
    let theme = Theme::new();
    assert_eq!(
        resolve_color_ref("$--primary", &vars, &theme),
        Some("#112233".to_string())
    );
}

#[test]
fn resolve_color_ref_resolves_themed_ref() {
    let mut vars = Variables::new();
    vars.insert(
        "--background".to_string(),
        serde_json::from_value(json!({
            "type": "color",
            "value": [
                {"value": "#FFFFFF", "theme": {"Mode": "Light"}},
                {"value": "#000000", "theme": {"Mode": "Dark"}}
            ]
        }))
        .unwrap(),
    );
    let mut theme = Theme::new();
    theme.insert("Mode".to_string(), "Dark".to_string());
    assert_eq!(
        resolve_color_ref("$--background", &vars, &theme),
        Some("#000000".to_string())
    );

    // With no active theme the first themed entry wins.
    assert_eq!(
        resolve_color_ref("$--background", &vars, &Theme::new()),
        Some("#FFFFFF".to_string())
    );
}

#[test]
fn resolve_color_ref_none_for_unresolvable_ref() {
    let vars = Variables::new();
    let theme = Theme::new();
    // Variable not in the table.
    assert_eq!(resolve_color_ref("$color-missing", &vars, &theme), None);

    // Variable resolves to a non-string (number) → None for a color ref.
    let mut numeric = Variables::new();
    numeric.insert(
        "spacing-lg".to_string(),
        serde_json::from_value(json!({"type": "number", "value": 24})).unwrap(),
    );
    assert_eq!(resolve_color_ref("$spacing-lg", &numeric, &theme), None);
}

#[test]
fn json_number_prefers_integer_encoding() {
    assert_eq!(json_number(40.0), json!(40));
    assert_eq!(json_number(12.5), json!(12.5));
    assert_eq!(json_number(f64::NAN), Value::Null);
}

#[test]
fn node_property_value_reads_height_corner_radius_font_size() {
    let frame = node(json!({
        "type": "frame", "id": "f", "height": 120, "cornerRadius": 8
    }));
    assert_eq!(node_property_value(&frame, "height"), Some(json!(120)));
    assert_eq!(node_property_value(&frame, "cornerRadius"), Some(json!(8)));

    let text = node(json!({"type": "text", "id": "t", "content": "x", "fontSize": 14}));
    assert_eq!(node_property_value(&text, "fontSize"), Some(json!(14)));

    // Per-corner cornerRadius re-encodes as a 4-array.
    let per = node(json!({"type": "frame", "id": "f", "cornerRadius": [4, 8, 4, 8]}));
    assert_eq!(
        node_property_value(&per, "cornerRadius"),
        Some(json!([4, 8, 4, 8]))
    );

    // Absent field / unrecognised property → None.
    let bare = node(json!({"type": "frame", "id": "f"}));
    assert_eq!(node_property_value(&bare, "height"), None);
    assert_eq!(node_property_value(&frame, "unknown"), None);
}

#[test]
fn corner_radius_numeric_treats_per_corner_and_absent_as_zero() {
    let uniform = node(json!({"type": "frame", "id": "f", "cornerRadius": 12}));
    assert_eq!(corner_radius_numeric(&uniform), 12.0);

    // PerCorner is not a TS `number` → 0.
    let per = node(json!({"type": "frame", "id": "f", "cornerRadius": [4, 8, 4, 8]}));
    assert_eq!(corner_radius_numeric(&per), 0.0);

    // Absent → 0.
    let bare = node(json!({"type": "frame", "id": "f"}));
    assert_eq!(corner_radius_numeric(&bare), 0.0);

    // Ellipse carries a plain f64.
    let ell = node(json!({"type": "ellipse", "id": "e", "cornerRadius": 5}));
    assert_eq!(corner_radius_numeric(&ell), 5.0);
}

#[test]
fn padding_accessor_reads_containers_only() {
    let frame = node(json!({"type": "frame", "id": "f", "padding": [8, 16]}));
    assert!(matches!(padding(&frame), Some(Padding::XY([8.0, 16.0]))));

    // Text carries no padding.
    let text = node(json!({"type": "text", "id": "t", "content": "x"}));
    assert!(padding(&text).is_none());
}

#[test]
fn default_theme_takes_first_value_per_axis() {
    let mut themes = BTreeMap::new();
    themes.insert(
        "Mode".to_string(),
        vec!["Light".to_string(), "Dark".to_string()],
    );
    themes.insert("Density".to_string(), vec!["Comfortable".to_string()]);
    let theme = default_theme(Some(&themes));
    assert_eq!(theme.get("Mode"), Some(&"Light".to_string()));
    assert_eq!(theme.get("Density"), Some(&"Comfortable".to_string()));

    // No themes → empty map.
    assert!(default_theme(None).is_empty());
}
