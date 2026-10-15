use super::*;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::test_support::VecDocSink;
use jian_ops_schema::node::container::CornerRadius;
use serde_json::json;

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "P".into(),
            width: 390.0,
            height: 844.0,
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

fn find_node<'a>(node: &'a PenNode, id: &str) -> Option<&'a PenNode> {
    if node.id_str() == id {
        return Some(node);
    }
    node.children()?
        .iter()
        .find_map(|child| find_node(child, id))
}

// Cluster test modules — this file keeps the shared fixtures.
#[path = "cleanup_mobile_nav_chrome_tests.rs"]
mod nav_chrome_tests;
#[path = "cleanup_mobile_shell_tests.rs"]
mod shell_tests;
