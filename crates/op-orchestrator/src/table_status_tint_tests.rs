use super::*;
use op_editor_core::EditorState;
fn fixture() -> Value {
    let mut rows = vec![
        json!({"type":"frame","id":"header","layout":"horizontal","children":[{"type":"text","id":"name-head","content":"Name"},{"type":"text","id":"status-head","content":"Status"}]}),
    ];
    for (i, (label, dot)) in [
        ("Active", "#16A34A"),
        ("Suspended", "#DC2626"),
        ("Invited", "#D97706"),
    ]
    .iter()
    .enumerate()
    {
        rows.push(json!({"type":"frame","id":format!("row-{i}"),"layout":"horizontal","children":[
            {"type":"text","id":format!("name-{i}"),"content":format!("User {i}")},
            {"type":"frame","id":format!("badge-{i}"),"name":"Status pill","fill":[{"type":"solid","color":"#16A34A1A"}],"children":[
                {"type":"ellipse","id":format!("dot-{i}"),"fill":[{"type":"solid","color":dot}]},
                {"type":"text","id":format!("label-{i}"),"content":label,"fontSize":12,"fill":[{"type":"solid","color":"#0F172A"}]}
            ]}
        ]}));
    }
    json!({"version":"1.1","children":[{"type":"frame","id":"root","children":[{"type":"frame","id":"table","name":"Users Table","layout":"vertical","children":rows}]}]})
}
fn apply(value: Value) -> EditorState {
    let mut state = EditorState::from_document(serde_json::from_value(value).unwrap());
    super::super::repair_status_tints(&mut crate::loop_finalize::StateDocSink {
        state: &mut state,
    });
    state
}
fn node(state: &EditorState, id: &str) -> Value {
    serde_json::to_value(
        op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(id)).unwrap(),
    )
    .unwrap()
}
#[test]
fn copied_state_tints_follow_the_authored_dot_without_restyling_copy() {
    let original = EditorState::from_document(serde_json::from_value(fixture()).unwrap());
    let mut state = apply(fixture());
    for (i, color) in ["#16A34A1A", "#DC26261A", "#D977061A"].iter().enumerate() {
        assert_eq!(
            node(&state, &format!("badge-{i}"))["fill"][0]["color"],
            *color
        );
        for id in [format!("dot-{i}"), format!("label-{i}")] {
            assert_eq!(node(&state, &id), node(&original, &id));
        }
    }
    let settled = state.doc.clone();
    super::super::repair_status_tints(&mut crate::loop_finalize::StateDocSink {
        state: &mut state,
    });
    assert_eq!(state.doc, settled);
}
#[test]
fn neutral_varied_custom_bound_and_opaque_surfaces_are_kept() {
    for colors in [
        ["#E2E8F01A"; 3],
        ["#16A34A1A", "#DC26261A", "#D977061A"],
        ["$--card"; 3],
        ["#16A34AFF"; 3],
    ] {
        let mut value = fixture();
        let rows = value["children"][0]["children"][0]["children"]
            .as_array_mut()
            .unwrap();
        for (row, color) in rows[1..].iter_mut().zip(colors) {
            row["children"][1]["fill"][0]["color"] = json!(color);
        }
        let original: jian_ops_schema::PenDocument = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(apply(value).doc, original);
    }
    let mut value = fixture();
    value["children"][0]["children"][0]["children"][2]["children"][1]["bindings"] =
        json!({"fill":"$app.tint"});
    let original: jian_ops_schema::PenDocument = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(apply(value).doc, original);
}
