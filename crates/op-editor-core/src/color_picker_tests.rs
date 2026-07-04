//! Tests for `color_picker` — split out to keep the module under
//! the 800-line file ceiling.

use super::*;
use crate::node_id::NodeId;
use crate::test_support::{rect, state_with};
use crate::ui_draft::ColorTarget;

fn doc_with_rect() -> EditorState {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 40.0, 30.0)]);
    s.set_single_selection(NodeId::new("n1"));
    s
}

#[test]
fn set_selected_color_writes_first_solid_fill() {
    let mut s = doc_with_rect();
    assert!(s.set_selected_color(true, "#ff0000"));
    let node = s.selected_node().unwrap();
    assert_eq!(crate::fills::first_solid_fill_hex(node), Some("#ff0000"));
}

#[test]
fn set_selected_color_writes_stroke() {
    let mut s = doc_with_rect();
    assert!(s.set_selected_color(false, "#00ff00"));
    let node = s.selected_node().unwrap();
    assert_eq!(crate::fills::first_solid_stroke_hex(node), Some("#00ff00"));
}

#[test]
fn set_selected_color_no_op_without_selection() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(!s.set_selected_color(true, "#ffffff"));
}

#[test]
fn add_drop_shadow_appends_effect() {
    let mut s = doc_with_rect();
    assert!(s.add_drop_shadow_to_selected());
    // A second call appends a second shadow.
    assert!(s.add_drop_shadow_to_selected());
}

#[test]
fn add_layer_blur_appends_blur_effect() {
    use jian_ops_schema::style::PenEffect;
    let mut s = doc_with_rect();
    assert!(s.add_layer_blur_to_selected());
    let node = s.selected_node().expect("selection");
    let effects = crate::node_effects(node);
    assert!(
        matches!(effects.last(), Some(PenEffect::Blur(_))),
        "add_layer_blur must append a Blur effect, got {effects:?}"
    );
}

#[test]
fn add_effect_on_effectless_node_creates_no_undo_state() {
    // An icon_font node carries no `effects` list. Adding an
    // effect must be a no-op that leaves NO empty undo/dirty state.
    let src = r#"{"version":"0.8.0","children":[
        {"type":"icon_font","id":"i1","name":"Icon",
         "x":0,"y":0,"width":20,"height":20,"iconFontName":"star"}
    ]}"#;
    let doc = jian_ops_schema::load_str(src)
        .expect("fixture parses")
        .value;
    let mut s = EditorState::from_document(doc);
    s.set_single_selection(NodeId::new("i1"));
    assert!(
        !s.add_layer_blur_to_selected(),
        "effect-less node must reject the add"
    );
    assert!(
        !s.undo(),
        "rejected add must not have pushed an empty undo state"
    );
}

#[test]
fn add_layer_blur_is_undoable() {
    let mut s = doc_with_rect();
    let before = crate::node_effects(s.selected_node().unwrap()).len();
    assert!(s.add_layer_blur_to_selected());
    assert_eq!(
        crate::node_effects(s.selected_node().unwrap()).len(),
        before + 1
    );
    assert!(s.undo(), "add layer blur must be undoable");
    assert_eq!(
        crate::node_effects(s.selected_node().unwrap()).len(),
        before,
        "undo must remove the added blur"
    );
}

#[test]
fn open_picker_seeds_hsv_from_fill() {
    let mut s = doc_with_rect();
    s.set_selected_color(true, "#ff8800");
    assert!(s.open_color_picker(ColorTarget::Fill, 120.0));
    let state = s.ui.color_picker.as_ref().unwrap();
    // Orange #ff8800 → hue near 32°.
    assert!(state.hue > 20.0 && state.hue < 45.0, "hue {}", state.hue);
    assert!(state.sat > 0.95);
    assert!(state.val > 0.95);
    assert!(s.ui.pending_color_history.is_some());
}

#[test]
fn picker_set_hsv_writes_through_to_node() {
    let mut s = doc_with_rect();
    assert!(s.open_color_picker(ColorTarget::Fill, 0.0));
    // Pure red: H=0 S=1 V=1.
    assert!(s.color_picker_set_hsv(0.0, 1.0, 1.0));
    let node = s.selected_node().unwrap();
    assert_eq!(crate::fills::first_solid_fill_hex(node), Some("#ff0000"));
}

#[test]
fn close_picker_pushes_history_only_on_change() {
    let mut s = doc_with_rect();
    let depth = s.history.past.len();
    assert!(s.open_color_picker(ColorTarget::Fill, 0.0));
    // No HSV change → close does not push history.
    assert!(s.close_color_picker());
    assert_eq!(s.history.past.len(), depth);

    // Re-open + drag + close → history grows by one.
    assert!(s.open_color_picker(ColorTarget::Fill, 0.0));
    assert!(s.color_picker_set_hsv(180.0, 1.0, 1.0));
    assert!(s.close_color_picker());
    assert_eq!(s.history.past.len(), depth + 1);
}

#[test]
fn undo_after_picker_edit_restores_color() {
    let mut s = doc_with_rect();
    s.set_selected_color(true, "#ff8800");
    assert!(s.open_color_picker(ColorTarget::Fill, 0.0));
    assert!(s.color_picker_set_hsv(0.0, 1.0, 1.0));
    assert!(s.close_color_picker());
    assert_eq!(
        crate::fills::first_solid_fill_hex(s.selected_node().unwrap()),
        Some("#ff0000")
    );
    assert!(s.undo());
    assert_eq!(
        crate::fills::first_solid_fill_hex(s.selected_node().unwrap()),
        Some("#ff8800")
    );
}

#[test]
fn hsv_roundtrip_is_stable() {
    for &hex in &["#ff0000", "#00ff00", "#0000ff", "#808080", "#ff8800"] {
        let rgb = parse_hex_rgb(hex).unwrap();
        let (h, s, v) = rgb_to_hsv(rgb);
        let (r, g, b) = hsv_to_rgb(h, s, v);
        assert_eq!(rgb_to_hex(r, g, b), hex, "roundtrip {hex}");
    }
}

// --- Variable-mode picker (Gap 1) -------------------------------

use jian_ops_schema::variable::{VariableKind, VariableScalar};

/// A state holding one Color variable and no nodes.
fn state_with_color_var(name: &str, hex: &str) -> EditorState {
    let mut s = state_with(vec![]);
    s.create_variable(name, VariableKind::Color, VariableScalar::Str(hex.into()));
    s
}

#[test]
fn open_picker_for_variable_seeds_hsv_from_resolved_color() {
    let mut s = state_with_color_var("brand", "#ff8800");
    assert!(s.open_color_picker_for_variable("brand", 100.0));
    let state = s.ui.color_picker.as_ref().expect("picker open");
    assert_eq!(state.variable.as_deref(), Some("brand"));
    assert!(
        s.ui.pending_color_history.is_some(),
        "undo snapshot captured"
    );
    // #ff8800 (orange) → hue near 32°.
    assert!(state.hue > 20.0 && state.hue < 45.0, "hue {}", state.hue);
    assert!(state.sat > 0.95, "sat {}", state.sat);
    assert!(state.val > 0.95, "val {}", state.val);
}

#[test]
fn open_picker_for_variable_fails_on_missing_or_wrong_kind() {
    let mut s = state_with(vec![]);
    // Unknown name → false, no picker.
    assert!(!s.open_color_picker_for_variable("nope", 0.0));
    assert!(s.ui.color_picker.is_none());
    // Number-kind variable → not a colour → false.
    s.create_variable("spacing", VariableKind::Number, VariableScalar::Num(16.0));
    assert!(!s.open_color_picker_for_variable("spacing", 0.0));
    assert!(s.ui.color_picker.is_none());
}

#[test]
fn picker_set_hsv_writes_through_variable_path() {
    // Open on a variable, push pure-red HSV, confirm the variable
    // flips and no node fill is touched (there are no nodes).
    let mut s = state_with_color_var("brand", "#ff8800");
    assert!(s.open_color_picker_for_variable("brand", 0.0));
    assert!(s.color_picker_set_hsv(0.0, 1.0, 1.0));
    match s.resolve_variable("brand") {
        Some(VariableScalar::Str(hex)) => assert_eq!(hex, "#ff0000"),
        other => panic!("expected red, got {other:?}"),
    }
}

// --- Variant-targeted picker (#19) -------------------------------

/// State with one Color variable and a 2-variant Theme-1 axis,
/// active theme pinned to Light.
fn state_with_two_variant_color(hex: &str) -> EditorState {
    let mut s = state_with_color_var("brand", hex);
    let mut themes = std::collections::BTreeMap::new();
    themes.insert(
        "Theme-1".to_string(),
        vec!["Light".to_string(), "Dark".to_string()],
    );
    s.doc.themes = Some(themes);
    s.ui.variables
        .active_theme
        .insert("Theme-1".into(), "Light".into());
    s
}

#[test]
fn variant_targeted_picker_writes_the_clicked_column() {
    // Click the Dark column swatch while the canvas renders Light:
    // the commit must land in Dark, leaving Light untouched (this
    // was the #19 wrong-cell-write bug).
    let mut s = state_with_two_variant_color("#ff8800");
    assert!(s.open_color_picker_for_variable_theme_at("brand", "Theme-1", "Dark", 10.0, 20.0));
    let state = s.ui.color_picker.as_ref().expect("picker open");
    assert_eq!(
        state.variable_theme,
        Some(("Theme-1".to_string(), "Dark".to_string()))
    );
    assert!(s.color_picker_set_hsv(0.0, 1.0, 1.0)); // → red
                                                    // Active theme (Light) still resolves the original colour.
    assert_eq!(
        s.resolve_variable("brand"),
        Some(&VariableScalar::Str("#ff8800".into()))
    );
    // The Dark entry carries the new red.
    s.ui.variables
        .active_theme
        .insert("Theme-1".into(), "Dark".into());
    assert_eq!(
        s.resolve_variable("brand"),
        Some(&VariableScalar::Str("#ff0000".into()))
    );
}

#[test]
fn variant_targeted_picker_seeds_hsv_from_that_column() {
    let mut s = state_with_two_variant_color("#ff0000");
    // Materialize a green Dark entry, keep Light red.
    assert!(s.set_variable_color_for_theme("brand", "Theme-1", "Dark", "#00ff00"));
    assert!(s.open_color_picker_for_variable_theme_at("brand", "Theme-1", "Dark", 0.0, 0.0));
    let state = s.ui.color_picker.as_ref().expect("picker open");
    // Green hue ≈ 120°, not red's 0° (which an active-theme seed
    // would have produced).
    assert!(
        (state.hue - 120.0).abs() < 1.0,
        "expected green seed, hue {}",
        state.hue
    );
}

#[test]
fn close_picker_after_variable_edit_pushes_history_only_on_change() {
    let mut s = state_with_color_var("brand", "#ff8800");
    let depth = s.history.past.len();
    // No HSV change → close does not push history.
    assert!(s.open_color_picker_for_variable("brand", 0.0));
    assert!(s.close_color_picker());
    assert_eq!(s.history.past.len(), depth);
    // Re-open + drag + close → history grows by one.
    assert!(s.open_color_picker_for_variable("brand", 0.0));
    assert!(s.color_picker_set_hsv(180.0, 1.0, 1.0));
    assert!(s.close_color_picker());
    assert_eq!(s.history.past.len(), depth + 1);
}

#[test]
fn undo_after_variable_picker_edit_restores_color() {
    // The picker's pre-edit snapshot carries the whole PenDocument
    // (variables included), so undo round-trips the variable.
    let mut s = state_with_color_var("brand", "#ff8800");
    assert!(s.open_color_picker_for_variable("brand", 0.0));
    assert!(s.color_picker_set_hsv(0.0, 1.0, 1.0)); // → red
    assert!(s.close_color_picker());
    match s.resolve_variable("brand") {
        Some(VariableScalar::Str(hex)) => assert_eq!(hex, "#ff0000"),
        other => panic!("expected red post-edit, got {other:?}"),
    }
    assert!(s.undo());
    match s.resolve_variable("brand") {
        Some(VariableScalar::Str(hex)) => assert_eq!(hex, "#ff8800"),
        other => panic!("undo must restore #ff8800, got {other:?}"),
    }
}

#[test]
fn gradient_stop_picker_preserves_alpha() {
    // Open the picker on a transparent stop (`#00000000`) and
    // drag SV → the resulting stop must still carry the original
    // alpha, not silently flip to opaque.
    use jian_ops_schema::node::PenNode;
    use jian_ops_schema::style::{GradientStop, LinearGradientBody, PenFill};
    let mut node = rect("n1", "r", 0.0, 0.0, 40.0, 30.0);
    // Seed a 2-stop gradient where stop 1 is fully transparent.
    let body = PenFill::LinearGradient(LinearGradientBody {
        angle: Some(0.0),
        stops: vec![
            GradientStop {
                offset: 0.0,
                color: "#ffffff".into(),
            },
            GradientStop {
                offset: 1.0,
                color: "#00000000".into(),
            },
        ],
        explain: None,
        opacity: None,
        blend_mode: None,
    });
    if let PenNode::Rectangle(r) = &mut node {
        r.container.fill = Some(vec![body]);
    } else {
        panic!("expected rectangle");
    }
    let mut s = state_with(vec![node]);
    s.set_single_selection(NodeId::new("n1"));
    assert!(s.open_color_picker(ColorTarget::GradientStop(1), 100.0));
    assert!(s.color_picker_set_hsv(0.0, 1.0, 1.0)); // → red
    let _ = s.close_color_picker();
    let node = s.selected_node().expect("rect");
    let stops = match crate::fills::node_fills(node)
        .and_then(|f| f.first())
        .expect("first fill")
    {
        PenFill::LinearGradient(b) => &b.stops,
        other => panic!("expected linear, got {other:?}"),
    };
    let written = &stops[1].color;
    assert!(
        written.eq_ignore_ascii_case("#ff000000"),
        "alpha must round-trip; got {written}"
    );
    // Stop 0 (opaque) must be untouched.
    assert!(stops[0].color.eq_ignore_ascii_case("#ffffff"));
}
