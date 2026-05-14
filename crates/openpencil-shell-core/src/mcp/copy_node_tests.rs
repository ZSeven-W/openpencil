//! Tests for `mcp::write_tools::CopyNode` + the matching
//! `Document::apply_mcp_command(CopyNode)` branch. Carved off
//! `write_tools_tests.rs` to keep that file under the 800-line cap
//! once the copy_node surface landed.

use super::write_tools::*;
use super::{McpCommand, McpTool, ToolErrorCode, ToolOutcome};
use std::collections::BTreeMap;

#[test]
fn copy_node_validates_args() {
    let tool = copy_node_snapshot();
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::MissingArgument),
        _ => panic!(),
    }
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "10".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::MissingArgument);
            assert!(msg.contains("target_parent_id"));
        }
        _ => panic!(),
    }
    args.insert("node_id".into(), "0".into());
    args.insert("target_parent_id".into(), "5".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
    // node_id == target_parent_id IS allowed (copying a container's
    // contents under itself is a valid LLM op).
    args.insert("node_id".into(), "10".into());
    args.insert("target_parent_id".into(), "10".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, McpCommand::CopyNode { node_id, target_parent_id }) => {
            assert_eq!(node_id, 10);
            assert_eq!(target_parent_id, 10);
        }
        other => panic!("expected CopyNode, got {other:?}"),
    }
}

#[test]
fn apply_mcp_command_copy_node_clones_to_page_root_with_fresh_ids() {
    use crate::document::Document;
    let mut doc = Document::sample();
    let pre_max = doc.max_node_id();
    let pre_root_len = doc.pages[0].children.len();
    let cmd = McpCommand::CopyNode {
        node_id: 12, // Button group with rect(13) + text(14)
        target_parent_id: 0,
    };
    assert!(doc.apply_mcp_command(&cmd));
    // Original Button still under Frame.
    let frame = doc.pages[0].children.iter().find(|n| n.id.raw() == 10).unwrap();
    assert!(frame.children.iter().any(|n| n.id.raw() == 12));
    // Page root grew by 1.
    assert_eq!(doc.pages[0].children.len(), pre_root_len + 1);
    // The newest root child has a fresh id past pre_max and the
    // same shape (2 children) as the source.
    let clone = doc.pages[0].children.last().unwrap();
    assert!(
        clone.id.raw() > pre_max,
        "clone id {} must exceed pre_max {pre_max}",
        clone.id.raw()
    );
    assert_eq!(clone.children.len(), 2);
    for child in &clone.children {
        assert!(child.id.raw() > pre_max, "child clone id must be fresh");
    }
}

#[test]
fn apply_mcp_command_copy_node_clones_into_container() {
    use crate::document::Document;
    let mut doc = Document::sample();
    let pre_button_children = {
        let frame = doc.pages[0].children.iter().find(|n| n.id.raw() == 10).unwrap();
        let button = frame.children.iter().find(|n| n.id.raw() == 12).unwrap();
        button.children.len()
    };
    // Clone Title (id 11) under Button group (id 12).
    let cmd = McpCommand::CopyNode {
        node_id: 11,
        target_parent_id: 12,
    };
    assert!(doc.apply_mcp_command(&cmd));
    let frame = doc.pages[0].children.iter().find(|n| n.id.raw() == 10).unwrap();
    let button = frame.children.iter().find(|n| n.id.raw() == 12).unwrap();
    assert_eq!(button.children.len(), pre_button_children + 1);
    // Original Title still at its old spot under Frame.
    assert!(frame.children.iter().any(|n| n.id.raw() == 11));
}

#[test]
fn apply_mcp_command_copy_node_rejects_unknown_source_or_target() {
    use crate::document::Document;
    let mut doc = Document::sample();
    let pre_root_len = doc.pages[0].children.len();
    assert!(!doc.apply_mcp_command(&McpCommand::CopyNode {
        node_id: 99999,
        target_parent_id: 0,
    }));
    assert!(!doc.apply_mcp_command(&McpCommand::CopyNode {
        node_id: 11,
        target_parent_id: 99999,
    }));
    // Doc unchanged either way.
    assert_eq!(doc.pages[0].children.len(), pre_root_len);
}
