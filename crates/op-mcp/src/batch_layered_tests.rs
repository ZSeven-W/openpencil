//! Tests for TS-shaped layered design workflow arguments.

use std::collections::BTreeMap;

use op_editor_core::{NodeId, PenNodeExt};

use super::batch_design::{
    design_content_snapshot, design_refine_snapshot, design_skeleton_snapshot,
};
use super::{parse_tool_call, EditorCommand, McpTool, ToolOutcome};

#[test]
fn design_content_children_insert_under_section_with_page_id() {
    let tool = design_content_snapshot();
    let mut args = BTreeMap::new();
    args.insert("sectionId".into(), "section-1".into());
    args.insert("pageId".into(), "page-2".into());
    args.insert(
        "children".into(),
        r##"[{"type":"text","name":"Title","content":"Hello","width":120,"height":24}]"##.into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::InsertSubtree {
                nodes,
                parent_id,
                page_id,
            },
        ) => {
            assert_eq!(result.get("sectionId"), Some(&"section-1".to_string()));
            assert_eq!(result.get("insertedCount"), Some(&"1".to_string()));
            assert_eq!(parent_id.as_str(), "section-1");
            assert_eq!(page_id.as_deref(), Some("page-2"));
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].base().name.as_deref(), Some("Title"));
        }
        other => panic!("expected design_content InsertSubtree, got {other:?}"),
    }
}

#[test]
fn design_content_counts_nested_nodes_and_reports_default_post_processing() {
    let tool = design_content_snapshot();
    let mut args = BTreeMap::new();
    args.insert("sectionId".into(), "section-1".into());
    args.insert(
        "children".into(),
        r##"[{"type":"frame","name":"Card","width":"fill_container","height":"fit_content","children":[{"type":"text","name":"Body","content":"This text is deliberately long","width":200,"height":20}]}]"##.into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(result, EditorCommand::InsertSubtree { nodes, .. }) => {
            assert_eq!(nodes.len(), 1);
            assert_eq!(result.get("insertedCount"), Some(&"2".to_string()));
            assert_eq!(result.get("postProcessed"), Some(&"true".to_string()));
            let warnings: serde_json::Value =
                serde_json::from_str(result.get("warnings").expect("warnings json")).unwrap();
            assert!(warnings
                .as_array()
                .expect("warnings array")
                .iter()
                .any(|warning| warning
                    .as_str()
                    .is_some_and(|text| text.contains("without textGrowth"))));
        }
        other => panic!("expected design_content InsertSubtree, got {other:?}"),
    }
}

#[test]
fn parse_tool_call_allows_structured_design_skeleton_for_ts_parity() {
    let line = r#"{"id":8,"method":"tools/call","params":{"name":"design_skeleton","arguments":{"rootFrame":{"name":"Page","width":375,"height":812},"sections":[{"name":"Hero","height":240}]}}}"#;
    let call = parse_tool_call(line).expect("design_skeleton must accept TS-style rootFrame");
    assert_eq!(call.tool, "design_skeleton");
    assert_eq!(
        call.arguments.get("rootFrame"),
        Some(&r#"{"name":"Page","width":375,"height":812}"#.to_string())
    );
    assert_eq!(
        call.arguments.get("sections"),
        Some(&r#"[{"name":"Hero","height":240}]"#.to_string())
    );
}

#[test]
fn design_skeleton_root_sections_emit_authored_subtree_with_section_ids() {
    let tool = design_skeleton_snapshot();
    let mut args = BTreeMap::new();
    args.insert(
        "rootFrame".into(),
        r#"{"name":"Page","width":375,"height":812}"#.into(),
    );
    args.insert(
        "sections".into(),
        r#"[{"name":"Hero","height":240,"padding":[24,24],"role":"hero"}]"#.into(),
    );
    args.insert("canvasWidth".into(), "375".into());
    args.insert("pageId".into(), "page-2".into());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::InsertAuthoredSubtree {
                nodes,
                parent_id,
                page_id,
            },
        ) => {
            let root_id = result.get("rootId").expect("root id").to_string();
            let section_id = result.get("sectionIds").expect("section ids").to_string();
            assert_eq!(result.get("phase"), Some(&"skeleton".to_string()));
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].id_str(), root_id);
            assert_eq!(parent_id, NodeId::NONE);
            assert_eq!(page_id.as_deref(), Some("page-2"));
            let sections = nodes[0].children().expect("sections");
            assert_eq!(sections.len(), 1);
            assert_eq!(sections[0].id_str(), section_id);

            let section_rows: serde_json::Value =
                serde_json::from_str(result.get("sections").expect("sections json")).unwrap();
            let first = section_rows
                .as_array()
                .and_then(|items| items.first())
                .expect("first section row");
            assert_eq!(
                first.get("id").and_then(|v| v.as_str()),
                Some(section_id.as_str())
            );
            assert!(first
                .get("guidelines")
                .and_then(|v| v.as_str())
                .is_some_and(
                    |text| text.contains("Large headline") && text.contains("Content width: 327px")
                ));
            assert_eq!(
                first
                    .get("suggestedRoles")
                    .and_then(|v| v.as_array())
                    .map(|roles| {
                        roles
                            .iter()
                            .filter_map(|role| role.as_str())
                            .collect::<Vec<_>>()
                    }),
                Some(vec![
                    "hero",
                    "heading",
                    "subheading",
                    "body-text",
                    "button",
                    "phone-mockup",
                    "row",
                ])
            );
        }
        other => panic!("expected design_skeleton authored subtree, got {other:?}"),
    }
}

#[test]
fn design_refine_root_id_emits_refine_command_with_page_id() {
    let tool = design_refine_snapshot();
    let mut args = BTreeMap::new();
    args.insert("rootId".into(), "root-1".into());
    args.insert("canvasWidth".into(), "375".into());
    args.insert("pageId".into(), "page-2".into());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::RefineDesign {
                root_id,
                canvas_width,
                page_id,
            },
        ) => {
            assert_eq!(result.get("phase"), Some(&"refine".to_string()));
            assert_eq!(result.get("rootId"), Some(&"root-1".to_string()));
            assert_eq!(root_id.as_str(), "root-1");
            assert_eq!(canvas_width, Some(375));
            assert_eq!(page_id.as_deref(), Some("page-2"));
        }
        other => panic!("expected design_refine RefineDesign command, got {other:?}"),
    }
}
