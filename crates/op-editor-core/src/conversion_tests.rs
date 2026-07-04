use std::collections::BTreeMap;

use crate::node_id::NodeId;
use crate::test_support::state_with;
use crate::EditorCommand;
use crate::{test_support, PenNodeExt};
use jian_ops_schema::conversion::ConversionKind;
use jian_ops_schema::node::{PenNode, TextContent};
use jian_ops_schema::variable::{VariableDefinition, VariableKind, VariableScalar, VariableValue};

fn color_var(hex: &str) -> VariableDefinition {
    VariableDefinition {
        kind: VariableKind::Color,
        value: VariableValue::Scalar(VariableScalar::Str(hex.into())),
    }
}

fn upsert_vars_cmd(key: &str, name: &str, hex: &str) -> EditorCommand {
    let mut variables = BTreeMap::new();
    variables.insert(name.to_string(), color_var(hex));
    EditorCommand::UpsertVariables {
        variables,
        key: key.into(),
        source_path: Some("src/theme.css".into()),
        source_hash: Some("h1".into()),
    }
}

#[test]
fn upsert_variables_merges_and_writes_ledger() {
    let mut state = state_with(vec![]);
    assert!(state.apply(upsert_vars_cmd(
        "tokens:theme.css",
        "color/primary",
        "#3366ff",
    )));
    let doc = &state.doc;
    assert!(doc
        .variables
        .as_ref()
        .unwrap()
        .contains_key("color/primary"));
    let entries = &doc.conversion.as_ref().unwrap().entries;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, ConversionKind::Token);
    assert_eq!(entries[0].key, "tokens:theme.css");
}

#[test]
fn upsert_variables_is_idempotent() {
    let mut state = state_with(vec![]);
    assert!(state.apply(upsert_vars_cmd(
        "tokens:theme.css",
        "color/primary",
        "#3366ff",
    )));
    assert!(state.apply(upsert_vars_cmd(
        "tokens:theme.css",
        "color/primary",
        "#112233",
    )));
    let doc = &state.doc;
    assert_eq!(doc.variables.as_ref().unwrap().len(), 1);
    assert_eq!(doc.conversion.as_ref().unwrap().entries.len(), 1);
}

fn upsert_component_cmd(key: &str, name: &str, width: f64) -> EditorCommand {
    let root = test_support::frame("incoming", "Master", 0.0, 0.0, width, 40.0, vec![]);
    EditorCommand::UpsertComponent {
        key: key.into(),
        name: name.into(),
        root: Box::new(root),
        source_path: Some("src/Button.tsx".into()),
        source_hash: Some("h1".into()),
    }
}

#[test]
fn upsert_component_creates_master_on_components_page() {
    let mut state = state_with(vec![]);
    assert!(state.apply(upsert_component_cmd(
        "src/Button.tsx#Button",
        "Button",
        100.0,
    )));
    assert_eq!(state.components.components.len(), 1);
    assert_eq!(state.components.components[0].name, "Button");
    let entry = crate::conversion::find_conversion_entry(
        &state.doc,
        ConversionKind::Component,
        "src/Button.tsx#Button",
    )
    .unwrap();
    let master_id = entry.node_id.clone().unwrap();
    assert_eq!(state.components.components[0].id.as_str(), master_id);
    assert_eq!(state.components.components[0].root.id_str(), master_id);
    assert!(state
        .doc
        .pages
        .as_ref()
        .unwrap()
        .iter()
        .any(|page| page.name == "Components"));
}

#[test]
fn upsert_component_replaces_master_keeping_id() {
    let mut state = state_with(vec![]);
    assert!(state.apply(upsert_component_cmd("k1", "Button", 100.0)));
    let id_before =
        crate::conversion::find_conversion_entry(&state.doc, ConversionKind::Component, "k1")
            .unwrap()
            .node_id
            .clone()
            .unwrap();
    assert!(state.apply(upsert_component_cmd("k1", "Button", 200.0)));
    let entry =
        crate::conversion::find_conversion_entry(&state.doc, ConversionKind::Component, "k1")
            .unwrap();
    assert_eq!(entry.node_id.as_deref(), Some(id_before.as_str()));
    assert_eq!(state.components.components.len(), 1);
    assert_eq!(state.doc.conversion.as_ref().unwrap().entries.len(), 1);
    assert_eq!(state.components.components[0].root.width_px(), Some(200.0));
}

fn doc_node<'a>(doc: &'a jian_ops_schema::PenDocument, node_id: &str) -> &'a PenNode {
    let id = NodeId::new(node_id);
    if let Some(pages) = doc.pages.as_ref() {
        for page in pages {
            if let Some(node) = crate::walkers::find_node(&page.children, &id) {
                return node;
            }
        }
    }
    crate::walkers::find_node(&doc.children, &id).expect("node must exist")
}

fn child_id_by_name(node: &PenNode, name: &str) -> String {
    node.children()
        .expect("node has children")
        .iter()
        .find(|child| child.base().name.as_deref() == Some(name))
        .map(|child| child.id_str().to_string())
        .expect("child with name must exist")
}

fn text_content_by_name<'a>(node: &'a PenNode, name: &str) -> &'a str {
    let child = node
        .children()
        .expect("node has children")
        .iter()
        .find(|child| child.base().name.as_deref() == Some(name))
        .expect("child with name must exist");
    match child {
        PenNode::Text(text) => match &text.content {
            TextContent::Plain(content) => content,
            TextContent::Styled(_) => panic!("expected plain text"),
        },
        other => panic!("expected text child, got {other:?}"),
    }
}

#[test]
fn upsert_component_rebuilds_group_master_from_conversion_ledger() {
    let mut state = state_with(vec![]);
    let root = test_support::group(
        "group-source",
        "Button Group",
        vec![test_support::rect(
            "bg-source",
            "Background",
            0.0,
            0.0,
            120.0,
            40.0,
        )],
    );
    assert!(state.apply(EditorCommand::UpsertComponent {
        key: "src/Button.tsx#ButtonGroup".into(),
        name: "Button Group".into(),
        root: Box::new(root),
        source_path: Some("src/Button.tsx".into()),
        source_hash: Some("h1".into()),
    }));
    let master_id = crate::conversion::find_conversion_entry(
        &state.doc,
        ConversionKind::Component,
        "src/Button.tsx#ButtonGroup",
    )
    .unwrap()
    .node_id
    .clone()
    .unwrap();

    let reloaded = crate::EditorState::from_document(state.doc.clone());
    let component = reloaded
        .components
        .find_by_id(&NodeId::new(master_id.clone()))
        .expect("group master should rebuild from conversion ledger");
    assert_eq!(component.name, "Button Group");
    assert_eq!(component.root.id_str(), master_id);
}

#[test]
fn upsert_component_rerun_preserves_descendant_ids() {
    let mut state = state_with(vec![]);
    let child = test_support::text("label", "Label", 8.0, 10.0, 80.0, 20.0, "Save");
    let root = test_support::frame("button", "Button", 0.0, 0.0, 120.0, 40.0, vec![child]);
    let command = EditorCommand::UpsertComponent {
        key: "src/Button.tsx#Button".into(),
        name: "Button".into(),
        root: Box::new(root),
        source_path: Some("src/Button.tsx".into()),
        source_hash: Some("h1".into()),
    };
    assert!(state.apply(command.clone()));
    let snapshot = serde_json::to_value(&state.doc).unwrap();
    assert!(state.apply(command));
    assert_eq!(serde_json::to_value(&state.doc).unwrap(), snapshot);
}

#[test]
fn upsert_component_rerun_preserves_descendant_ids_by_source_id_not_index() {
    let mut state = state_with(vec![]);
    let first_root = test_support::frame(
        "button-source",
        "Button",
        0.0,
        0.0,
        120.0,
        40.0,
        vec![
            test_support::text("source-label", "Label", 8.0, 10.0, 80.0, 20.0, "Save"),
            test_support::rect("source-icon", "Icon", 96.0, 10.0, 16.0, 16.0),
        ],
    );
    assert!(state.apply(EditorCommand::UpsertComponent {
        key: "src/Button.tsx#Button".into(),
        name: "Button".into(),
        root: Box::new(first_root),
        source_path: Some("src/Button.tsx".into()),
        source_hash: Some("h1".into()),
    }));
    let master_id = crate::conversion::find_conversion_entry(
        &state.doc,
        ConversionKind::Component,
        "src/Button.tsx#Button",
    )
    .unwrap()
    .node_id
    .clone()
    .unwrap();
    let first_master = doc_node(&state.doc, &master_id);
    let label_id = child_id_by_name(first_master, "Label");
    let icon_id = child_id_by_name(first_master, "Icon");

    let second_root = test_support::frame(
        "button-source",
        "Button",
        0.0,
        0.0,
        140.0,
        40.0,
        vec![
            test_support::text("source-badge", "Badge", 8.0, 10.0, 24.0, 20.0, "New"),
            test_support::rect("source-icon", "Icon", 104.0, 10.0, 16.0, 16.0),
            test_support::text("source-label", "Label", 36.0, 10.0, 64.0, 20.0, "Submit"),
        ],
    );
    assert!(state.apply(EditorCommand::UpsertComponent {
        key: "src/Button.tsx#Button".into(),
        name: "Button".into(),
        root: Box::new(second_root),
        source_path: Some("src/Button.tsx".into()),
        source_hash: Some("h2".into()),
    }));
    let second_master = doc_node(&state.doc, &master_id);

    assert_eq!(child_id_by_name(second_master, "Label"), label_id);
    assert_eq!(text_content_by_name(second_master, "Label"), "Submit");
    assert_eq!(child_id_by_name(second_master, "Icon"), icon_id);
    let badge_id = child_id_by_name(second_master, "Badge");
    assert_ne!(badge_id, label_id);
    assert_ne!(badge_id, icon_id);
}

#[test]
fn upsert_component_rejects_non_component_root() {
    let mut state = state_with(vec![]);
    let text_root = test_support::text("text", "Label", 0.0, 0.0, 40.0, 20.0, "hi");
    let command = EditorCommand::UpsertComponent {
        key: "k".into(),
        name: "T".into(),
        root: Box::new(text_root),
        source_path: None,
        source_hash: None,
    };
    assert!(!state.apply(command));
    assert!(state.doc.conversion.is_none());
}

fn upsert_screen_cmd(key: &str, width: f64) -> EditorCommand {
    let root = test_support::frame("screen", "Home", 0.0, 0.0, width, 900.0, vec![]);
    EditorCommand::UpsertScreen {
        key: key.into(),
        root: Box::new(root),
        source_path: Some("src/routes/home.tsx".into()),
        source_hash: Some("h1".into()),
    }
}

#[test]
fn upsert_screen_inserts_then_replaces_keeping_id() {
    let mut state = state_with(vec![]);
    assert!(state.apply(upsert_screen_cmd("route:/", 1440.0)));
    let count_before = state.active_children().len();
    let id_before =
        crate::conversion::find_conversion_entry(&state.doc, ConversionKind::Screen, "route:/")
            .unwrap()
            .node_id
            .clone()
            .unwrap();
    assert!(state.apply(upsert_screen_cmd("route:/", 1920.0)));
    assert_eq!(state.active_children().len(), count_before);
    let entry =
        crate::conversion::find_conversion_entry(&state.doc, ConversionKind::Screen, "route:/")
            .unwrap();
    assert_eq!(entry.node_id.as_deref(), Some(id_before.as_str()));
    assert_eq!(state.active_children()[0].width_px(), Some(1920.0));
}

#[test]
fn upsert_screen_rerun_preserves_descendant_ids() {
    let mut state = state_with(vec![]);
    let child = test_support::text("title", "Title", 24.0, 32.0, 160.0, 32.0, "Home");
    let root = test_support::frame("screen", "Home", 0.0, 0.0, 1440.0, 900.0, vec![child]);
    let command = EditorCommand::UpsertScreen {
        key: "route:/".into(),
        root: Box::new(root),
        source_path: Some("src/routes/home.tsx".into()),
        source_hash: Some("h1".into()),
    };
    assert!(state.apply(command.clone()));
    let snapshot = serde_json::to_value(&state.doc).unwrap();
    assert!(state.apply(command));
    assert_eq!(serde_json::to_value(&state.doc).unwrap(), snapshot);
}

#[test]
fn upsert_screen_rejects_non_frame_root() {
    let mut state = state_with(vec![]);
    let command = EditorCommand::UpsertScreen {
        key: "route:/".into(),
        root: Box::new(test_support::text(
            "text", "Label", 0.0, 0.0, 40.0, 20.0, "x",
        )),
        source_path: None,
        source_hash: None,
    };
    assert!(!state.apply(command));
}
