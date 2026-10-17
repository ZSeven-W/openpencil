use super::*;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::test_support::VecDocSink;
use op_editor_core::EffectField;
use serde_json::json;

/// Walk the active-page tree by name and return the first matching node's id.
fn find_node_id_by_name(state: &EditorState, name: &str) -> String {
    fn walk(nodes: &[PenNode], name: &str) -> Option<String> {
        for n in nodes {
            if n.base().name.as_deref().unwrap_or("") == name {
                return Some(n.id_str().to_string());
            }
            if let Some(c) = n.children() {
                if let Some(hit) = walk(c, name) {
                    return Some(hit);
                }
            }
        }
        None
    }
    walk(state.active_children(), name).expect("named node exists")
}

/// 同 `frame_json` 但返回 `serde_json::Value`(供嵌套构造)。
fn frame_json_value(id: &str, children: serde_json::Value) -> serde_json::Value {
    json!({
        "type": "frame", "id": id, "name": id,
        "x": 0, "y": 0, "width": 100, "height": 100,
        "children": children,
    })
}

fn frame_json(id: &str, children: serde_json::Value) -> PenNode {
    serde_json::from_value(frame_json_value(id, children)).expect("frame json")
}

#[test]
fn explicit_mobile_viewport_contract_accepts_all_container_variants() {
    for root_type in ["frame", "group", "rectangle"] {
        for viewport_type in ["frame", "group", "rectangle"] {
            let root: PenNode = serde_json::from_value(json!({
                "type": root_type,
                "id": format!("{root_type}-root"),
                "width": 390,
                "height": 844,
                "layout": "vertical",
                "children": [{
                    "type": viewport_type,
                    "id": format!("{viewport_type}-viewport"),
                    "role": "viewport",
                    "width": "fill_container",
                    "height": "fill_container",
                    "layout": "vertical",
                    "clipContent": true,
                    "children": [{
                        "type": "frame",
                        "id": "content",
                        "width": "fill_container",
                        "height": 1200
                    }]
                }]
            }))
            .expect("valid container contract");

            assert!(
                has_explicit_mobile_viewport_contract(&root),
                "{root_type} root with {viewport_type} viewport must preserve its height"
            );
        }
    }
}

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        shared_context: None,
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "P".into(),
            width: 1200.0,
            height: 800.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
        palette: None,
    }
}

// Cluster test modules — this file keeps the shared fixtures; each child
// mounts with `use super::*`.
#[path = "cleanup_nav_surface_tests.rs"]
mod nav_surface_tests;
#[path = "cleanup_root_height_tests.rs"]
mod root_height_tests;
