use super::*;

fn state_with_stack(layout: &str, children: serde_json::Value) -> EditorState {
    let root: PenNode = serde_json::from_value(serde_json::json!({
        "type": "frame", "id": "root", "name": "Page", "width": 390, "height": 844,
        "layout": "vertical", "children": [{
            "type": "frame", "id": "wrap", "name": "Image Wrap", "width": 168,
            "height": 112, "layout": layout, "children": children
        }]
    }))
    .expect("valid overlay fixture");
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(root);
    state
}

fn surface() -> serde_json::Value {
    serde_json::json!({
        "type": "rectangle", "id": "media", "name": "Event Image", "x": 0, "y": 0,
        "width": "fill_container", "height": "fill_container",
        "fill": [{"type": "solid", "color": "#E8E0D4"}]
    })
}

fn badge() -> serde_json::Value {
    serde_json::json!({
        "type": "frame", "id": "badge", "name": "DateBadge1", "x": 8, "y": 8,
        "width": 46, "height": 46,
        "fill": [{"type": "solid", "color": "#0A3D91"}]
    })
}

#[test]
fn full_bleed_surface_before_badge_reports_index_zero_fix_without_mutation() {
    let state = state_with_stack("none", serde_json::json!([surface(), badge()]));
    let before = serde_json::to_value(&state.doc).unwrap();

    let diagnostics = collect_batch_design_diagnostics(&state);

    assert_eq!(diagnostics.layout_issues.len(), 1, "{diagnostics:?}");
    let issue = &diagnostics.layout_issues[0];
    for marker in [
        "children[0] is topmost",
        "M(\"badge\", \"wrap\", 0)",
        "separate EMPTY frame/rectangle image slot",
        "strict G(...)",
    ] {
        assert!(issue.contains(marker), "missing {marker:?}: {issue}");
    }
    assert_eq!(serde_json::to_value(&state.doc).unwrap(), before);
}

#[test]
fn correct_order_and_out_of_scope_stacks_are_not_reported() {
    let correct = state_with_stack("none", serde_json::json!([badge(), surface()]));
    assert!(collect_batch_design_diagnostics(&correct)
        .layout_issues
        .is_empty());

    let flow = state_with_stack("vertical", serde_json::json!([surface(), badge()]));
    assert!(collect_batch_design_diagnostics(&flow)
        .layout_issues
        .is_empty());

    let small_surface = serde_json::json!({
        "type": "rectangle", "id": "small", "name": "Accent Surface", "x": 0, "y": 0,
        "width": 24, "height": 24,
        "fill": [{"type": "solid", "color": "#E8E0D4"}]
    });
    let non_bleed = state_with_stack("none", serde_json::json!([small_surface, badge()]));
    assert!(collect_batch_design_diagnostics(&non_bleed)
        .layout_issues
        .is_empty());
}

#[test]
fn batch_design_feedback_surfaces_order_issue_without_reordering() {
    let mut state = state_with_stack("none", serde_json::json!([surface(), badge()]));

    let (result, mutated) = crate::design_agent_tools::execute_design_tool(
        &mut state,
        "batch_design",
        r#"{"operations":"note=I(null,{type:'frame',name:'Next Shell',width:80,height:40})"}"#,
    );

    assert!(!result.is_error, "batch failed: {}", result.content);
    assert!(mutated);
    let value: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    assert!(value["layoutIssues"]
        .as_array()
        .is_some_and(|issues| issues.iter().any(|issue| issue
            .as_str()
            .is_some_and(|line| line.contains("M(\"badge\", \"wrap\", 0)")))));
    let stack = &state.active_children()[0].children().unwrap()[0];
    let ids: Vec<&str> = stack
        .children()
        .unwrap()
        .iter()
        .map(|node| node.id_str())
        .collect();
    assert_eq!(ids, ["media", "badge"], "diagnostic must not reorder");
}
