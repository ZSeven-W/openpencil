//! Tests for the public mobile-screen normalization contract.

use super::*;
use op_editor_core::pen_node_ext::PenNodeExt;

fn mobile_root(height: serde_json::Value, children: Option<serde_json::Value>) -> PenNode {
    let mut value = serde_json::json!({
        "type": "frame",
        "id": "screen",
        "name": "Screen",
        "width": 375,
        "height": height,
        "fill": [{"type": "solid", "color": "#f7f8fa"}]
    });
    if let Some(children) = children {
        value["children"] = children;
    }
    serde_json::from_value(value).expect("mobile root")
}

fn hand_drawn_status_bar() -> serde_json::Value {
    serde_json::json!({
        "type": "frame",
        "id": "hand-drawn-status-bar",
        "name": "Status Bar",
        "width": "fill_container",
        "height": 62,
        "children": [
            {"type": "text", "id": "time", "content": "9:41"},
            {"type": "text", "id": "icons", "content": "signal wifi battery"}
        ]
    })
}

fn is_canonical(node: &PenNode) -> bool {
    node.base().role.as_deref() == Some("status-bar")
        && node.children().is_some_and(|children| {
            children
                .iter()
                .any(|child| child.base().name.as_deref() == Some("Levels"))
        })
}

#[test]
fn replaces_hand_drawn_status_bar_without_changing_root_child_count() {
    let mut state = EditorState::new();
    state.active_children_mut().push(mobile_root(
        serde_json::json!(812),
        Some(serde_json::json!([hand_drawn_status_bar()])),
    ));
    let child_count = state.active_children()[0]
        .children()
        .expect("root children")
        .len();

    let report = normalize_mobile_screens(&mut state);

    assert_eq!(report.status_bars_replaced, 1);
    assert_eq!(report.status_bars_inserted, 0);
    assert_eq!(
        state.active_children()[0]
            .children()
            .expect("root children")
            .len(),
        child_count
    );
    assert!(is_canonical(
        &state.active_children()[0]
            .children()
            .expect("root children")[0]
    ));
}

#[test]
fn inserts_canonical_status_bar_at_index_zero_when_missing() {
    let mut state = EditorState::new();
    state
        .active_children_mut()
        .push(mobile_root(serde_json::json!(812), None));

    let report = normalize_mobile_screens(&mut state);

    assert_eq!(report.status_bars_inserted, 1);
    assert!(is_canonical(
        &state.active_children()[0]
            .children()
            .expect("root children")[0]
    ));
}

#[test]
fn fixes_fit_content_mobile_viewport_after_status_bar_insertion() {
    let mut state = EditorState::new();
    state
        .active_children_mut()
        .push(mobile_root(serde_json::json!("fit_content"), None));

    let report = normalize_mobile_screens(&mut state);

    assert_eq!(report.status_bars_inserted, 1);
    assert_eq!(report.viewport_heights_fixed, 1);
    assert_eq!(state.active_children()[0].height_px(), Some(812.0));
}

#[test]
fn leaves_desktop_root_unchanged() {
    let root: PenNode = serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "desktop",
        "name": "Desktop",
        "width": 1440,
        "height": "fit_content",
        "children": [{"type": "text", "id": "title", "content": "Dashboard"}]
    }))
    .expect("desktop root");
    let mut state = EditorState::new();
    state.active_children_mut().push(root);
    let before = state.active_children()[0].clone();

    let report = normalize_mobile_screens(&mut state);

    assert_eq!(report, MobileNormalizeReport::default());
    assert_eq!(state.active_children()[0], before);
}

#[test]
fn leaves_numeric_mobile_height_unchanged() {
    let mut state = EditorState::new();
    state
        .active_children_mut()
        .push(mobile_root(serde_json::json!(600), None));

    let report = normalize_mobile_screens(&mut state);

    assert_eq!(report.status_bars_inserted, 1);
    assert_eq!(report.viewport_heights_fixed, 0);
    assert_eq!(state.active_children()[0].height_px(), Some(600.0));
}

#[test]
fn normalization_is_idempotent() {
    let mut state = EditorState::new();
    state
        .active_children_mut()
        .push(mobile_root(serde_json::json!("fit_content"), None));

    let first = normalize_mobile_screens(&mut state);
    let second = normalize_mobile_screens(&mut state);

    assert_eq!(first.status_bars_inserted, 1);
    assert_eq!(first.viewport_heights_fixed, 1);
    assert_eq!(second, MobileNormalizeReport::default());
}
