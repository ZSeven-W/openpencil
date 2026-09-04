use crate::icon_path_normalize::{
    canonicalize_path_d, normalize_icon_paths, IconPathNormalizeReport,
};
use crate::PenNodeExt;
use crate::{svg_path_data_bounds, EditorState};
use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

fn state_with_node(node: Value) -> EditorState {
    let document = serde_json::from_value(json!({
        "version": "1.0.0",
        "children": [node]
    }))
    .expect("test document");
    EditorState::from_document(document)
}

fn chevron_lookup(d: &str) -> Option<&'static str> {
    (canonicalize_path_d(d).as_deref() == Some("M 6 9 L 12 15 L 18 9")).then_some("chevron-down")
}

#[test]
fn known_chevron_becomes_icon_font_and_keeps_box_and_stroke_fill() {
    let mut state = state_with_node(json!({
        "type": "path",
        "id": "chevron",
        "name": "ChevronDownIcon",
        "role": "icon",
        "d": "M6 9l6 6 6-6",
        "width": 14,
        "height": 14,
        "stroke": {
            "thickness": 2.2,
            "fill": [{"type": "solid", "color": "#1A1614"}]
        }
    }));

    let report = normalize_icon_paths(&mut state, chevron_lookup);
    assert_eq!(report.converted_to_icon_font, 1);
    assert_eq!(report.refit_uniform, 0);
    let PenNode::IconFont(icon) = &state.active_children()[0] else {
        panic!("expected icon_font")
    };
    assert_eq!(icon.icon_font_name, "chevron-down");
    assert_eq!(icon.icon_font_family.as_deref(), Some("lucide"));
    assert!(
        matches!(icon.width, Some(jian_ops_schema::sizing::SizingBehavior::Number(value)) if value == 14.0)
    );
    assert!(
        matches!(icon.height, Some(jian_ops_schema::sizing::SizingBehavior::Number(value)) if value == 14.0)
    );
    assert_eq!(icon.base.role.as_deref(), Some("icon"));
    assert_eq!(
        icon.fill.as_ref().and_then(|fills| fills.first()),
        Some(&jian_ops_schema::style::PenFill::Solid(
            jian_ops_schema::style::SolidFillBody {
                color: "#1A1614".into(),
                explain: None,
                opacity: None,
                blend_mode: None,
            }
        ))
    );
}

#[test]
fn lookup_accepts_whitespace_and_comma_variants() {
    let mut state = state_with_node(json!({
        "type": "path",
        "id": "chevron",
        "d": "M 6,9 l 6,6 6,-6",
        "width": 14,
        "height": 14
    }));

    let report = normalize_icon_paths(&mut state, chevron_lookup);
    assert_eq!(
        report,
        IconPathNormalizeReport {
            converted_to_icon_font: 1,
            refit_uniform: 0
        }
    );
    assert!(matches!(state.active_children()[0], PenNode::IconFont(_)));
}

#[test]
fn unknown_viewbox_path_is_refit_uniformly_and_marked_idempotent() {
    let mut state = state_with_node(json!({
        "type": "path",
        "id": "square-path",
        "d": "M4 4h16v16H4z",
        "width": 12,
        "height": 20,
        "stroke": {"thickness": 2.0}
    }));

    let first = normalize_icon_paths(&mut state, |_| None);
    assert_eq!(first.converted_to_icon_font, 0);
    assert_eq!(first.refit_uniform, 1);
    let PenNode::Path(path) = &state.active_children()[0] else {
        panic!("expected path")
    };
    let d = path.d.as_deref().expect("refit d");
    let (min_x, min_y, width, height) = svg_path_data_bounds(d).expect("refit bounds");
    assert!((min_x - 2.0).abs() < 0.001);
    assert!((min_y - 6.0).abs() < 0.001);
    assert!((width - 8.0).abs() < 0.001);
    assert!((height - 8.0).abs() < 0.001);
    assert_eq!(path.icon_id.as_deref(), Some("openpencil:icon-path-refit"));
    assert_eq!(
        path.stroke.as_ref().and_then(|s| match s.thickness {
            jian_ops_schema::style::StrokeThickness::Uniform(value) => Some(value),
            _ => None,
        }),
        Some(1.0)
    );

    let d_after_first = d.to_string();
    let second = normalize_icon_paths(&mut state, |_| None);
    assert_eq!(second, IconPathNormalizeReport::default());
    let PenNode::Path(path) = &state.active_children()[0] else {
        panic!("expected path")
    };
    assert_eq!(path.d.as_deref(), Some(d_after_first.as_str()));
}

#[test]
fn large_decorative_path_is_untouched() {
    let mut state = state_with_node(json!({
        "type": "path",
        "id": "decorative",
        "d": "M0 0C60 40 120 -20 180 0",
        "width": 200,
        "height": 120
    }));
    let before = serde_json::to_value(&state.active_children()[0]).unwrap();
    let report = normalize_icon_paths(&mut state, |_| Some("not-used"));
    assert_eq!(report, IconPathNormalizeReport::default());
    assert_eq!(
        serde_json::to_value(&state.active_children()[0]).unwrap(),
        before
    );
}

#[test]
fn editable_anchored_path_is_untouched() {
    let mut state = state_with_node(json!({
        "type": "path",
        "id": "editable",
        "d": "M4 4L20 20",
        "anchors": [{"x": 4, "y": 4}, {"x": 20, "y": 20}],
        "width": 14,
        "height": 14
    }));
    let before = serde_json::to_value(&state.active_children()[0]).unwrap();
    let report = normalize_icon_paths(&mut state, |_| Some("not-used"));
    assert_eq!(report, IconPathNormalizeReport::default());
    assert_eq!(
        serde_json::to_value(&state.active_children()[0]).unwrap(),
        before
    );
}

#[test]
fn local_px_path_that_already_fills_its_box_is_untouched() {
    // The status bar's wifi glyph shape: authored in local px, bounds ≈ box.
    let mut state = state_with_node(json!({
        "type": "path", "id": "wifi", "name": "Wifi", "width": 17.142, "height": 12.328,
        "d": "M0.5 3.2 C5.2 -0.9 11.9 -0.9 16.6 3.2 L14.8 5.1 C11.2 2 5.9 2 2.3 5.1 Z M3.6 6.5 C6.6 3.9 10.5 3.9 13.5 6.5 L11.7 8.4 C9.7 6.8 7.4 6.8 5.4 8.4 Z M6.7 9.8 C7.9 8.9 9.2 8.9 10.4 9.8 L8.55 11.9 Z",
        "fill": [{"type":"solid","color":"#000000"}]
    }));
    let before = serde_json::to_string(&state.active_children()[0]).unwrap();
    let report = normalize_icon_paths(&mut state, chevron_lookup);
    assert_eq!(report, IconPathNormalizeReport::default());
    assert_eq!(
        serde_json::to_string(&state.active_children()[0]).unwrap(),
        before
    );
}

#[test]
fn status_bar_subtree_is_skipped_even_for_a_lucide_glyph() {
    let mut state = state_with_node(json!({
        "type": "frame", "id": "sb", "name": "Status Bar", "role": "status-bar",
        "width": 375, "height": 62, "layout": "horizontal",
        "children": [
            {"type":"path","id":"chev","name":"ChevronDownIcon","width":14,"height":14,
             "d":"M6 9l6 6 6-6","stroke":{"thickness":2.0,"fill":[{"type":"solid","color":"#000"}]}}
        ]
    }));
    let report = normalize_icon_paths(&mut state, chevron_lookup);
    assert_eq!(report, IconPathNormalizeReport::default());
    assert!(matches!(
        state.active_children()[0].children().unwrap()[0],
        PenNode::Path(_)
    ));
}
