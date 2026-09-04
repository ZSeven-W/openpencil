//! Selection-driven panel construction + snapshot tests — the
//! `for_selection*` builders, the tab-conditional panel lifetime, and
//! the accessibility node.
//!
//! Split out of `property_panel_tests.rs` to keep both files under
//! the openpencil 800-line cap.

use super::{color_eq, RoundFillBackend};
use crate::layout_scene::{LayoutScene, NodeKind, SceneNode, ScenePage};
use crate::widgets::property_panel::PropertyPanel;
use crate::widgets::property_panel_test_support::state_from;
use crate::widgets::{PaintCx, Widget};
use crate::{Point2D, Rect};
use op_editor_core::{EditorState, NodeId, PropertyTab};

#[test]
fn for_selection_with_real_node_builds_snapshot() {
    let state = EditorState::sample();
    let panel = PropertyPanel::for_selection(&state).expect("sample doc has a selection");
    assert_eq!(panel.snapshot.kind, "Text");
    assert_eq!(panel.snapshot.name, "Title");
    // Title node bounds: (60, 60, 240, 28).
    assert_eq!(panel.snapshot.x, 60);
    assert_eq!(panel.snapshot.y, 60);
    assert_eq!(panel.snapshot.width, 240);
    assert_eq!(panel.snapshot.height, 28);
}

#[test]
fn snapshot_reads_imported_node_opacity_as_percent() {
    let mut state = state_from(
        r#"{ "version": "1.0.0", "children": [
              {"type":"rectangle","id":"r","name":"Figma layer",
               "width":100,"height":40,"opacity":0.425}
        ]}"#,
    );
    state.set_single_selection(NodeId::new("r"));

    let panel = PropertyPanel::for_selection(&state).expect("selected rectangle panel");
    assert!((panel.snapshot.opacity_percent - 42.5).abs() < f32::EPSILON);
}

#[test]
fn scene_aware_panel_reports_resolved_fill_and_hug_dimensions() {
    let mut state = state_from(
        r##"{ "version": "1.0.0", "children": [
              {"type":"frame","id":"ff","name":"Frame",
               "width":"fill_container","height":"fit_content","children":[]}
        ]}"##,
    );
    state.set_single_selection(NodeId::new("ff"));
    let mut resolved = SceneNode::leaf("ff", NodeKind::Frame);
    resolved.bounds = Rect::xywh(0.0, 0.0, 390.0, 710.0);
    let scene = LayoutScene {
        pages: vec![ScenePage {
            id: "p1".into(),
            name: "Page 1".into(),
            children: vec![resolved],
        }],
        active_page_index: 0,
    };

    let panel = PropertyPanel::for_selection_with_scene(&state, &scene)
        .expect("scene-aware fill/hug panel");
    assert_eq!((panel.snapshot.width, panel.snapshot.height), (390, 710));
    assert!(panel.snapshot.size_fill_width);
    assert!(panel.snapshot.size_hug_height);
}

#[test]
fn scene_aware_panel_keeps_unbounded_group_aggregate_dimensions() {
    let mut state = state_from(
        r##"{ "version": "1.0.0", "children": [
              {"type":"group","id":"g","name":"Group","children":[
                {"type":"rectangle","id":"child","x":10,"y":20,
                 "width":70,"height":30}
              ]}
        ]}"##,
    );
    state.set_single_selection(NodeId::new("g"));
    let mut child = SceneNode::leaf("child", NodeKind::Rect);
    child.bounds = Rect::xywh(10.0, 20.0, 70.0, 30.0);
    let mut group = SceneNode::leaf("g", NodeKind::Group);
    group.children = vec![child];
    let scene = LayoutScene {
        pages: vec![ScenePage {
            id: "p1".into(),
            name: "Page 1".into(),
            children: vec![group],
        }],
        active_page_index: 0,
    };

    let panel =
        PropertyPanel::for_selection_with_scene(&state, &scene).expect("scene-aware group panel");
    assert_eq!((panel.snapshot.width, panel.snapshot.height), (70, 30));
}

#[test]
fn for_selection_without_selection_returns_none() {
    let state = EditorState::new();
    assert!(PropertyPanel::for_selection(&state).is_none());
}

#[test]
fn for_selection_code_tab_builds_panel_without_selection() {
    // The Code tab is selection-independent (TS falls back to the active
    // page's children), so the panel must stay alive with no selection.
    let mut state = EditorState::sample();
    state.clear_selection();
    state.editor_ui.property_tab = PropertyTab::Code;
    let panel =
        PropertyPanel::for_selection(&state).expect("Code tab panel survives empty selection");
    assert!(matches!(panel.tab, PropertyTab::Code));
    // The idle node-count label reads the LIVE generation targets — with
    // an empty selection that is every active-page child.
    assert_eq!(
        panel.codegen.selection_snapshot.len(),
        state.active_children().len()
    );
    // Design input rows are never clickable under the Code body.
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 700.0),
    };
    assert!(panel.hit_test(rect, Point2D::new(140.0, 120.0)).is_none());
}

#[test]
fn compact_touch_hides_code_tab_and_presents_retained_code_as_design() {
    use op_editor_core::size_class::EditorSizeClass;

    let mut state = EditorState::sample();
    state.editor_ui.touch = true;
    state.editor_ui.size_class = EditorSizeClass::Compact;
    state.editor_ui.property_tab = PropertyTab::Code;

    let panel = PropertyPanel::for_selection(&state).expect("sample doc has a selection");
    assert_eq!(panel.tab, PropertyTab::Design);
    assert!(!panel.code_tab_available);

    state.clear_selection();
    assert!(
        PropertyPanel::for_selection(&state).is_none(),
        "retained Code state must use Design's selection gate on Compact"
    );
}

#[test]
fn medium_touch_keeps_selection_independent_code_panel() {
    use op_editor_core::size_class::EditorSizeClass;

    let mut state = EditorState::sample();
    state.editor_ui.touch = true;
    state.editor_ui.size_class = EditorSizeClass::Medium;
    state.editor_ui.property_tab = PropertyTab::Code;
    state.clear_selection();

    let panel = PropertyPanel::for_selection(&state).expect("iPad Code tab remains available");
    assert_eq!(panel.tab, PropertyTab::Code);
    assert!(panel.code_tab_available);
}

#[test]
fn for_selection_design_tab_still_hides_panel_without_selection() {
    let mut state = EditorState::sample();
    state.clear_selection();
    state.editor_ui.property_tab = PropertyTab::Design;
    assert!(PropertyPanel::for_selection(&state).is_none());
}

#[test]
fn for_selection_interact_tab_still_hides_panel_without_selection() {
    let mut state = EditorState::sample();
    state.clear_selection();
    state.editor_ui.property_tab = PropertyTab::Interact;
    assert!(PropertyPanel::for_selection(&state).is_none());
}

#[test]
fn inactive_property_tab_hover_paints_pill_background() {
    let mut state = EditorState::sample();
    state.editor_ui.property_tab = PropertyTab::Code;
    state.editor_ui.property_tab_hover = Some(PropertyTab::Design);
    let panel = PropertyPanel::for_selection(&state).expect("sample doc has a selection");
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(280.0, 700.0),
    };
    let mut backend = RoundFillBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        panel.paint(&mut cx, rect);
    }

    let muted_pills = backend
        .fills
        .iter()
        .filter(|(_, color)| color_eq(*color, panel.theme.muted))
        .count();
    assert!(
        muted_pills >= 2,
        "active Code tab and hovered inactive Design tab should both paint a visible pill"
    );
}

#[test]
fn for_selection_with_stale_selection_returns_none() {
    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n9999"));
    assert!(PropertyPanel::for_selection(&state).is_none());
}

#[test]
fn access_node_advertises_group_with_kind_label() {
    let state = EditorState::sample();
    let panel = PropertyPanel::for_selection(&state).unwrap();
    let node = panel.access_node();
    assert_eq!(node.role(), accesskit::Role::Group);
    assert_eq!(node.label(), Some("Text"));
}

#[test]
fn group_snapshot_aggregates_child_bounds() {
    // A Group has no own bounds, so `from_node` must derive W/H
    // from children — else the panel shows "0 × 0" for a container.
    let mut state = EditorState::sample();
    state.set_single_selection(NodeId::new("n12"));
    let panel = PropertyPanel::for_selection(&state).unwrap();
    assert_eq!(panel.snapshot.kind, "Group");
    assert_eq!(panel.snapshot.x, 60);
    assert_eq!(panel.snapshot.y, 130);
    assert!(panel.snapshot.width > 0);
    assert!(panel.snapshot.height > 0);
}
