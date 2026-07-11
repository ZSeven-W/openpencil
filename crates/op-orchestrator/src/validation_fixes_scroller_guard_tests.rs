//! Regression tests for preserving intentional horizontal scrollers during
//! vision-validation fix application.

use super::*;
use crate::test_support::VecDocSink;
use crate::types::DocSink;
use op_editor_core::{EditorCommand, NodeId};
use serde_json::json;

fn scroller_sink() -> VecDocSink {
    let root = serde_json::from_str(
        r#"{
            "type":"frame","id":"root","name":"Explore",
            "width":375,"height":"fit_content","layout":"vertical",
            "children":[
                {
                    "type":"frame","id":"section","name":"Popular Destinations Rail",
                    "width":"fill_container","height":"fit_content","layout":"vertical",
                    "children":[
                        {
                            "type":"frame","id":"header","name":"Section Header",
                            "width":"fill_container","height":"fit_content","layout":"horizontal",
                            "children":[{
                                "type":"text","id":"title","name":"Section Title",
                                "width":"fit_content","height":"fit_content",
                                "content":"Popular Destinations"
                            }]
                        },
                        {
                            "type":"frame","id":"viewport","name":"Destinations Viewport",
                            "width":"fill_container","height":"fit_content","layout":"horizontal",
                            "clipContent":true,
                            "children":[{
                                "type":"frame","id":"rail","name":"Destinations Rail",
                                "width":"fit_content","height":"fit_content","layout":"horizontal","gap":12,
                                "children":[
                                    {
                                        "type":"frame","id":"kyoto","name":"Kyoto Card",
                                        "width":294,"height":300,"layout":"vertical","children":[]
                                    },
                                    {
                                        "type":"frame","id":"santorini","name":"Santorini Card",
                                        "width":208,"height":300,"layout":"vertical","children":[]
                                    }
                                ]
                            }]
                        }
                    ]
                },
                {
                    "type":"frame","id":"normal","name":"Normal Section",
                    "width":"fill_container","height":80,"layout":"vertical","children":[]
                }
            ]
        }"#,
    )
    .expect("scroller fixture");
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
fn layout_fixes_skip_intentional_scroller_region_but_apply_elsewhere() {
    // InsertSubtree remaps the depth-first fixture IDs to n1..n9.
    let mut sink = scroller_sink();
    let fixes = vec![
        ValidationFix {
            node_id: "n1".into(),
            property: "padding".into(),
            value: json!(24),
        },
        ValidationFix {
            node_id: "n2".into(),
            property: "padding".into(),
            value: json!([0, 24, 0, 24]),
        },
        ValidationFix {
            node_id: "n5".into(),
            property: "padding".into(),
            value: json!([0, 24, 0, 0]),
        },
        ValidationFix {
            node_id: "n6".into(),
            property: "gap".into(),
            value: json!(4),
        },
        ValidationFix {
            node_id: "n7".into(),
            property: "width".into(),
            value: json!("fill_container"),
        },
        ValidationFix {
            node_id: "n7".into(),
            property: "cornerRadius".into(),
            value: json!(18),
        },
        ValidationFix {
            node_id: "n9".into(),
            property: "padding".into(),
            value: json!(12),
        },
    ];

    let result = apply_validation_fixes(&mut sink, &fixes, &[]);

    assert_eq!(result.applied, 2, "only non-layout/style-safe fixes apply");
    assert_eq!(result.errors.len(), 5, "all scroller layout fixes skip");
    assert!(
        result
            .errors
            .iter()
            .all(|error| error.contains("intentional horizontal scroller")),
        "unexpected skip reasons: {:?}",
        result.errors
    );
    assert!(sink.applied.iter().any(|command| matches!(
        command,
        EditorCommand::SetNodeCornerRadius { node_id, radius }
            if node_id.as_str() == "n7" && (*radius - 18.0).abs() < f32::EPSILON
    )));
    assert!(sink.applied.iter().any(|command| matches!(
        command,
        EditorCommand::SetNodeLayoutProp { node_id, property, .. }
            if node_id.as_str() == "n9" && property == "padding"
    )));
    assert_eq!(sink.applied.len(), 2, "no scroller layout command emitted");
}

#[test]
fn structural_fixes_skip_intentional_scroller_region_but_apply_elsewhere() {
    let mut sink = scroller_sink();
    let structural_fixes = vec![
        StructuralFix::RemoveNode {
            node_id: "n7".into(),
        },
        StructuralFix::AddChild {
            parent_id: "n6".into(),
            index: None,
            spec: json!({
                "type": "frame",
                "name": "Injected Card",
                "width": 200,
                "height": 300
            }),
        },
        StructuralFix::AddChild {
            parent_id: "n9".into(),
            index: None,
            spec: json!({
                "type": "text",
                "name": "Normal Label",
                "content": "Allowed"
            }),
        },
    ];

    let result = apply_validation_fixes(&mut sink, &[], &structural_fixes);

    assert_eq!(
        result.applied, 1,
        "only the unrelated structural fix applies"
    );
    assert_eq!(result.errors.len(), 2);
    assert!(
        result
            .errors
            .iter()
            .all(|error| error.contains("intentional horizontal scroller")),
        "unexpected skip reasons: {:?}",
        result.errors
    );
    assert_eq!(
        sink.applied.len(),
        1,
        "scroller structure remains untouched"
    );
    assert!(matches!(
        &sink.applied[0],
        EditorCommand::InsertSubtree { parent_id, .. } if parent_id.as_str() == "n9"
    ));
}
