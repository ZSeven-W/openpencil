//! Retained GLM workout hero: the back button is behind an opaque photo wrapper.
use super::*;
use op_editor_core::{EditorState, PenNodeExt};
use serde_json::json;

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "test_fixtures/workout_buried_back_button.json"
    ))
    .unwrap()
}

fn state(root: Value) -> EditorState {
    EditorState::from_document(
        serde_json::from_value(json!({"version":"1.0","children":[root]})).unwrap(),
    )
}

fn fixes(draft: &EditorState) -> Vec<EditorCommand> {
    let root = serde_json::to_value(&draft.active_children()[0]).unwrap();
    let mut commands = Vec::new();
    collect_buried_overlay_fixes(&root, &super::super::resolved_rects(draft), &mut commands);
    commands
}

#[test]
fn retained_workout_back_button_moves_above_its_photo_viewport() {
    let original = fixture();
    let mut draft = state(original.clone());
    let commands = fixes(&draft);
    assert!(
        commands.iter().any(|command| matches!(command,
        EditorCommand::MoveNode {node_id,target_parent,index:Some(0),..}
        if node_id.as_str()=="n110" && target_parent.as_str()=="n105")),
        "the real back control must not stay buried: {commands:?}"
    );
    for command in commands {
        assert!(draft.apply(command));
    }
    let root = serde_json::to_value(&draft.active_children()[0]).unwrap();
    assert_eq!(root["children"][0], original["children"][1]);
    assert_eq!(root["children"][1], original["children"][0]);
    assert!(fixes(&draft).is_empty(), "correct stacking is stable");
    assert_eq!(draft.active_children()[0].id_str(), "n105");
}

#[test]
fn transparent_or_partial_media_wrappers_do_not_rescue_uncovered_controls() {
    for case in [
        "partial-photo",
        "faded-photo",
        "faded-wrapper",
        "blended-wrapper",
        "masked-photo",
        "rotated-photo",
        "fitted-photo",
        "hidden-photo",
    ] {
        let mut root = fixture();
        match case {
            "partial-photo" => {
                root["children"][0]["children"][2]["height"] = json!(100);
                root["children"][1]["y"] = json!(220);
            }
            "faded-photo" => root["children"][0]["children"][2]["opacity"] = json!(0.4),
            "faded-wrapper" => root["children"][0]["opacity"] = json!(0.4),
            "blended-wrapper" => root["children"][0]["blendMode"] = json!("multiply"),
            "masked-photo" => root["children"][0]["children"][2]["maskType"] = json!("alpha"),
            "rotated-photo" => root["children"][0]["children"][2]["rotation"] = json!(20),
            "fitted-photo" => root["children"][0]["children"][2]["objectFit"] = json!("fit"),
            "hidden-photo" => root["children"][0]["children"][2]["visible"] = json!(false),
            _ => unreachable!(),
        }
        assert!(fixes(&state(root)).is_empty(), "{case}");
    }
}

#[test]
#[ignore = "set OPENPENCIL_MEDIA_VIEWPORT_REPLAY and OPENPENCIL_MEDIA_VIEWPORT_OUT"]
fn replay_retained_media_viewport_document() {
    fn records(node: &Value, out: &mut std::collections::BTreeMap<String, Value>) {
        if let Some(id) = node.get("id").and_then(Value::as_str) {
            let mut value = node.clone();
            value.as_object_mut().unwrap().remove("children");
            out.insert(id.to_string(), value);
        }
        for child in children(node) {
            records(child, out);
        }
    }
    let input = std::env::var("OPENPENCIL_MEDIA_VIEWPORT_REPLAY").unwrap();
    let output = std::env::var("OPENPENCIL_MEDIA_VIEWPORT_OUT").unwrap();
    let source = std::fs::read_to_string(&input).unwrap();
    let doc = jian_ops_schema::load_str(&source).unwrap().value;
    let mut draft = EditorState::from_document(doc);
    let mut before = std::collections::BTreeMap::new();
    records(&serde_json::to_value(&draft.doc).unwrap(), &mut before);
    let ids: Vec<_> = draft
        .active_children()
        .iter()
        .map(|n| n.id_str().to_owned())
        .collect();
    {
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut draft };
        for id in ids {
            assert!(super::super::geometry_validate_and_fix(&mut sink, &id) > 0);
        }
    }
    let mut after = std::collections::BTreeMap::new();
    let value = serde_json::to_value(&draft.doc).unwrap();
    records(&value, &mut after);
    assert_eq!(
        before, after,
        "only stacking order may change; preserve every node's content and style"
    );
    assert!(fixes(&draft).is_empty());
    assert_eq!(std::fs::read_to_string(input).unwrap(), source);
    std::fs::write(output, serde_json::to_vec_pretty(&draft.doc).unwrap()).unwrap();
}
