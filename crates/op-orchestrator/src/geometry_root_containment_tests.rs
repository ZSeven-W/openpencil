use crate::geometry_validation::geometry_diagnostics;
use serde_json::json;

fn diagnostics_for(root: serde_json::Value) -> Vec<String> {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(json!({
        "version": "1.0",
        "children": [root]
    }))
    .expect("valid document");
    let state = op_editor_core::EditorState::from_document(doc);
    geometry_diagnostics(&state)
}

#[test]
fn clipped_mobile_root_reports_direct_flow_bottom_nav_past_root_bottom() {
    let issues = diagnostics_for(json!({
        "type": "frame", "id": "root", "name": "Music App Home",
        "width": 402, "height": 1100, "layout": "vertical", "clipContent": true,
        "children": [
            { "type": "frame", "id": "status", "name": "Status Bar",
              "width": "fill_container", "height": 62 },
            { "type": "frame", "id": "content", "name": "Content Wrapper",
              "width": "fill_container", "height": 1038 },
            { "type": "frame", "id": "nav", "name": "Bottom Tab Bar",
              "role": "bottom-tab-bar", "width": "fill_container", "height": 72 }
        ]
    }));

    assert!(
        issues.iter().any(|issue| {
            issue.contains("mobile-root-bottom-nav-overflow")
                && issue.contains("Bottom Tab Bar")
                && issue.contains("72px past")
        }),
        "root clipping must not hide required app chrome overflow: {issues:?}"
    );
}

#[test]
fn absolute_or_nested_bottom_nav_overflow_is_not_promoted_to_root_issue() {
    let absolute_issues = diagnostics_for(json!({
        "type": "frame", "id": "absolute-root", "name": "Absolute Screen",
        "width": 402, "height": 1100, "layout": "vertical", "clipContent": true,
        "children": [
            { "type": "frame", "id": "content", "name": "Content",
              "width": "fill_container", "height": 1100 },
            { "type": "frame", "id": "absolute-nav", "name": "Bottom Tab Bar",
              "role": "bottom-tab-bar", "x": 0, "y": 1100,
              "width": 402, "height": 72 }
        ]
    }));
    assert!(
        !absolute_issues
            .iter()
            .any(|issue| issue.contains("mobile-root-bottom-nav-overflow")),
        "an authored absolute position is not ordinary-flow evidence: {absolute_issues:?}"
    );

    let nested_issues = diagnostics_for(json!({
        "type": "frame", "id": "nested-root", "name": "Nested Screen",
        "width": 402, "height": 1100, "layout": "vertical", "clipContent": true,
        "children": [{
            "type": "frame", "id": "content", "name": "Content Wrapper",
            "width": "fill_container", "height": 1172, "layout": "vertical",
            "children": [{
                "type": "frame", "id": "nested-nav", "name": "Bottom Tab Bar",
                "role": "bottom-tab-bar", "width": "fill_container", "height": 72
            }]
        }]
    }));
    assert!(
        !nested_issues
            .iter()
            .any(|issue| issue.contains("mobile-root-bottom-nav-overflow")),
        "only a direct root child gets the narrow containment exception: {nested_issues:?}"
    );
}

#[test]
fn narrow_component_or_non_trailing_nav_is_not_reported_as_mobile_root_overflow() {
    let horizontal = diagnostics_for(json!({
        "type": "frame", "id": "toolbar", "name": "Narrow Toolbar",
        "width": 420, "height": 100, "layout": "horizontal", "children": [
            {"type":"frame", "id":"body", "width":300, "height":100},
            {"type":"frame", "id":"nav", "name":"Bottom Nav", "role":"bottom-tab-bar",
             "width":120, "height":180}
        ]
    }));
    assert!(
        !horizontal
            .iter()
            .any(|issue| issue.contains("mobile-root-bottom-nav-overflow")),
        "narrow components are not mobile artboards: {horizontal:?}"
    );

    let non_trailing = diagnostics_for(json!({
        "type": "frame", "id": "root", "name": "Mobile", "width": 402,
        "height": 800, "layout": "vertical", "clipContent": true, "children": [
            {"type":"frame", "id":"content", "width":"fill_container", "height":800},
            {"type":"frame", "id":"nav", "name":"Bottom Nav", "role":"bottom-tab-bar",
             "width":"fill_container", "height":72},
            {"type":"frame", "id":"late", "name":"Late Section",
             "width":"fill_container", "height":100}
        ]
    }));
    assert!(
        !non_trailing
            .iter()
            .any(|issue| issue.contains("mobile-root-bottom-nav-overflow")),
        "nav-order diagnostics own the non-trailing case: {non_trailing:?}"
    );
}
