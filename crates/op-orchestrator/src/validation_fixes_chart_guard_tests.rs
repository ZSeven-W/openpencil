//! Regression tests for preventing vision validation from outlining chart marks.

use super::*;
use crate::test_support::VecDocSink;
use crate::types::DocSink;
use op_editor_core::{EditorCommand, NodeId};
use serde_json::json;

fn chart_sink() -> VecDocSink {
    let root = serde_json::from_str(
        r#"{
            "type":"frame","id":"root","name":"Health Dashboard",
            "width":390,"height":844,"layout":"vertical",
            "children":[
                {
                    "type":"frame","id":"chart","name":"Weekly Activity Chart",
                    "width":"fill_container","height":300,"layout":"horizontal",
                    "children":[{
                        "type":"frame","id":"track","name":"Monday Bar Track",
                        "width":36,"height":220,"layout":"vertical","children":[{
                            "type":"frame","id":"fill","name":"Monday Bar Fill",
                            "width":"fill_container","height":120,"children":[]
                        }]
                    }]
                },
                {
                    "type":"frame","id":"normal","name":"Summary Card",
                    "width":"fill_container","height":80,"layout":"vertical","children":[]
                }
            ]
        }"#,
    )
    .expect("chart fixture");
    let mut sink = VecDocSink::new();
    sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
    sink
}

#[test]
fn stroke_fixes_skip_chart_marks_but_apply_to_normal_surfaces() {
    // InsertSubtree remaps the depth-first fixture IDs to n1..n5.
    let mut sink = chart_sink();
    let fixes = vec![
        ValidationFix {
            node_id: "n3".into(),
            property: "strokeColor".into(),
            value: json!("#2B2B2B"),
        },
        ValidationFix {
            node_id: "n3".into(),
            property: "strokeWidth".into(),
            value: json!(1),
        },
        ValidationFix {
            node_id: "n3".into(),
            property: "cornerRadius".into(),
            value: json!(18),
        },
        ValidationFix {
            node_id: "n5".into(),
            property: "strokeColor".into(),
            value: json!("#E2E8F0"),
        },
        ValidationFix {
            node_id: "n5".into(),
            property: "strokeWidth".into(),
            value: json!(1),
        },
    ];

    let result = apply_validation_fixes(&mut sink, &fixes, &[]);

    assert_eq!(
        result.applied, 3,
        "chart strokes must be the only skipped fixes"
    );
    assert_eq!(result.errors.len(), 2);
    assert!(
        result
            .errors
            .iter()
            .all(|error| error.contains("chart mark")),
        "unexpected skip reasons: {:?}",
        result.errors
    );
    assert!(sink.applied.iter().any(|command| matches!(
        command,
        EditorCommand::SetNodeCornerRadius { node_id, .. } if node_id.as_str() == "n3"
    )));
    assert!(sink.applied.iter().any(|command| matches!(
        command,
        EditorCommand::SetNodeStrokeHex { node_id, .. } if node_id.as_str() == "n5"
    )));
    assert!(sink.applied.iter().any(|command| matches!(
        command,
        EditorCommand::SetNodeStrokeWidth { node_id, .. } if node_id.as_str() == "n5"
    )));
    assert_eq!(sink.applied.len(), 3, "no chart stroke command emitted");
}

#[test]
fn stroke_fix_applies_to_bar_chart_card_surface() {
    let root = serde_json::from_str(
        r#"{
            "type":"frame","id":"root","name":"Health Dashboard",
            "width":390,"height":844,"layout":"vertical",
            "children":[{
                "type":"frame","id":"chart-card","name":"Bar Chart Card",
                "width":"fill_container","height":300,"layout":"vertical",
                "children":[{
                    "type":"frame","id":"plot","name":"Weekly Activity Chart",
                    "width":"fill_container","height":220,"layout":"horizontal","children":[]
                }]
            }]
        }"#,
    )
    .expect("chart card fixture");
    let mut sink = VecDocSink::new();
    sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
    let fixes = vec![ValidationFix {
        node_id: "n2".into(),
        property: "strokeColor".into(),
        value: json!("#E2E8F0"),
    }];

    let result = apply_validation_fixes(&mut sink, &fixes, &[]);

    assert_eq!(
        result.applied, 1,
        "chart card is a surface, not a data mark"
    );
    assert!(result.errors.is_empty());
    assert!(matches!(
        &sink.applied[0],
        EditorCommand::SetNodeStrokeHex { node_id, .. } if node_id.as_str() == "n2"
    ));
}

#[test]
fn role_tagged_cjk_chart_is_protected_without_name_needles() {
    // "走势" is NOT in the guard's name-needle list — protection must come
    // from the role_infer-assigned role="chart" (F2: structural signal over
    // name heuristics). Before role tagging this chart was unprotected.
    use crate::role_defaults::Theme;
    let mut chart: jian_ops_schema::node::PenNode = serde_json::from_value(serde_json::json!({
        "type": "frame", "id": "trend", "name": "心率走势图",
        "width": 320, "height": 160, "layout": "horizontal",
        "children": [
            { "type": "frame", "id": "bar1", "name": "Mon", "width": 24, "height": 120 },
            { "type": "frame", "id": "bar2", "name": "Tue", "width": 24, "height": 90 }
        ]
    }))
    .expect("chart node");
    crate::role_infer::resolve_forest_roles(std::slice::from_mut(&mut chart), 390.0, Theme::Dark);
    let mut state = op_editor_core::EditorState::new();
    state
        .insert_subtree_returning_root_ids(vec![chart], &op_editor_core::NodeId::NONE)
        .expect("insert chart");

    use op_editor_core::PenNodeExt;
    let bar_id: String = state
        .active_children()
        .iter()
        .flat_map(|n| n.children().into_iter().flatten())
        .find(|c: &&jian_ops_schema::node::PenNode| c.base().name.as_deref() == Some("Mon"))
        .map(|c| c.id_str().to_string())
        .expect("bar child");
    assert!(
        super::chart_guard::should_skip_chart_stroke_fix(
            &state,
            &op_editor_core::NodeId::new(&bar_id),
            "strokeColor"
        ),
        "role=chart context must protect marks even when no name needle matches"
    );
}
