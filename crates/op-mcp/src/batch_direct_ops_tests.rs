//! Regression tests for direct `U()` operation command selection.

use op_editor_core::{EditorCommand, NodeId};

use super::batch_direct_ops::parse_single_direct_operation;
use super::test_fixtures::sample;

#[test]
fn direct_update_routes_sizing_keywords_to_patch_and_applies() {
    let mut state = sample();
    let command = parse_single_direct_operation(
        r##"U("n10", {"width":"fill_container","height":"fit_content","fill_hex":"#112233"})"##,
    )
    .expect("valid direct update")
    .expect("update command");

    let EditorCommand::PatchNodeData {
        node_id,
        patch_json,
        page_id,
    } = &command
    else {
        panic!("sizing keywords must use PatchNodeData, got {command:?}");
    };
    assert_eq!(node_id.as_str(), "n10");
    assert_eq!(page_id, &None);
    let patch: serde_json::Value = serde_json::from_str(patch_json).expect("patch json");
    assert_eq!(patch["width"], "fill_container");
    assert_eq!(patch["height"], "fit_content");
    assert_eq!(patch["fill"][0]["color"], "#112233");
    assert!(patch.get("fill_hex").is_none());

    assert!(state.apply(command), "keyword patch must apply");
    let node = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new("n10"))
        .expect("updated frame");
    let value = serde_json::to_value(node).expect("updated node json");
    assert_eq!(value["width"], "fill_container");
    assert_eq!(value["height"], "fit_content");
    assert_eq!(value["fill"][0]["color"], "#112233");
}

#[test]
fn direct_update_keeps_numeric_sizes_on_flat_command() {
    let command = parse_single_direct_operation(r#"U("n10", {"width":320,"height":480})"#)
        .expect("valid direct update")
        .expect("update command");

    match command {
        EditorCommand::UpdateNode { width, height, .. } => {
            assert_eq!(width, Some(320));
            assert_eq!(height, Some(480));
        }
        other => panic!("numeric sizes must stay on UpdateNode, got {other:?}"),
    }
}

#[test]
fn direct_update_coerces_numeric_string_to_number() {
    // GLM-5.3 occasionally emits quoted numbers; accept them.
    let command = parse_single_direct_operation(r#"U("n10", {"width":"158","height":"240"})"#)
        .expect("valid direct update")
        .expect("update command");

    match command {
        EditorCommand::UpdateNode { width, height, .. } => {
            assert_eq!(width, Some(158));
            assert_eq!(height, Some(240));
        }
        other => panic!("numeric strings must coerce to UpdateNode, got {other:?}"),
    }
}

#[test]
fn direct_update_rejects_numeric_strings_with_units() {
    // Strings with units ("158px", "50%") are structural errors — the normalizer
    // accepts clean numeric strings but rejects any with non-digit chars.
    let err = parse_single_direct_operation(r#"U("n10", {"width":"158px"})"#)
        .expect_err("should return ProgramError for unit suffix");
    // Error must report the violation clearly.
    let msg = err.to_string();
    assert!(
        msg.contains("158px") || msg.contains("non-numeric"),
        "error message should mention the bad value or non-numeric chars, got: {}",
        msg
    );
}

#[test]
fn direct_update_keeps_non_numeric_field_strings_uncoerced() {
    // Field-scoped coercion: name and other text fields stay as strings.
    let command = parse_single_direct_operation(r#"U("n10", {"name":"158"})"#)
        .expect("valid direct update")
        .expect("update command");

    match command {
        EditorCommand::UpdateNode { name, .. } => {
            assert_eq!(name, Some("158".to_string()));
        }
        other => panic!("text fields must not coerce, got {other:?}"),
    }
}
