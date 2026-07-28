//! Mechanical and behavioral coverage for the command id-allocation funnel.

#![cfg(test)]

use crate::command::{BatchInsertItem, EditorCommand};
use crate::test_support::{frame, rect, sample, state_with};
use crate::{
    collect_document_ids, DocumentIdAllocator, EditorState, IdAllocError, NodeId, PeerNamespace,
    Tool,
};
use std::collections::HashSet;

fn namespaced() -> DocumentIdAllocator {
    DocumentIdAllocator::namespaced(PeerNamespace::try_from("audit").unwrap(), 0)
}

fn assert_new_ids_are_namespaced(before: &HashSet<NodeId>, state: &EditorState) {
    let after = collect_document_ids(&state.doc);
    let fresh: Vec<&NodeId> = after.difference(before).collect();
    assert!(!fresh.is_empty(), "test command did not mint an id");
    assert!(
        fresh.iter().all(|id| id.as_str().starts_with("c_audit_")),
        "non-session ids were minted: {fresh:?}",
    );
}

fn leaf_item(name: &str) -> BatchInsertItem {
    BatchInsertItem {
        kind: "rect".to_string(),
        name: name.to_string(),
        x: 0,
        y: 0,
        width: 10,
        height: 10,
        fill_hex: None,
        fill: None,
    }
}

fn insert_command(name: &str) -> EditorCommand {
    EditorCommand::InsertNode {
        kind: "rect".to_string(),
        name: name.to_string(),
        x: 0,
        y: 0,
        width: 10,
        height: 10,
        fill_hex: None,
        target_parent: NodeId::NONE,
        page_id: None,
    }
}

#[test]
fn raw_and_batch_commands_share_the_supplied_namespace() {
    let mut state = state_with(vec![
        rect("n1", "Replace me", 0.0, 0.0, 10.0, 10.0),
        frame(
            "n2",
            "Container",
            0.0,
            0.0,
            100.0,
            100.0,
            vec![rect("n3", "Nested", 0.0, 0.0, 10.0, 10.0)],
        ),
    ]);
    let before = collect_document_ids(&state.doc);
    let mut allocator = namespaced();

    let commands = [
        insert_command("Inserted"),
        EditorCommand::CopyNode {
            node_id: NodeId::new("n2"),
            target_parent: NodeId::NONE,
            overrides_json: None,
            page_id: None,
        },
        EditorCommand::ReplaceNode {
            node_id: NodeId::new("n1"),
            kind: "ellipse".to_string(),
            name: "Replacement".to_string(),
            x: 0,
            y: 0,
            width: 20,
            height: 20,
            fill_hex: None,
            drop_children: false,
            page_id: None,
        },
        EditorCommand::ReplaceSubtree {
            node_id: NodeId::new("n3"),
            node: Box::new(frame(
                "placeholder-root",
                "Subtree",
                0.0,
                0.0,
                20.0,
                20.0,
                vec![rect("placeholder-child", "Child", 0.0, 0.0, 5.0, 5.0)],
            )),
            drop_children: false,
            page_id: None,
        },
        EditorCommand::BatchInsert {
            items: vec![leaf_item("Batch A"), leaf_item("Batch B")],
            page_id: None,
        },
        EditorCommand::InsertSubtree {
            nodes: vec![frame(
                "incoming-root",
                "Incoming",
                0.0,
                0.0,
                20.0,
                20.0,
                vec![rect("incoming-child", "Child", 0.0, 0.0, 5.0, 5.0)],
            )],
            parent_id: NodeId::NONE,
            page_id: None,
        },
        EditorCommand::Batch {
            commands: vec![insert_command("Nested batch")],
        },
    ];
    for command in commands {
        assert!(state.apply_with_allocator(command, &mut allocator).unwrap());
    }

    assert_new_ids_are_namespaced(&before, &state);
}

#[test]
fn selection_page_and_import_commands_share_the_supplied_namespace() {
    let mut state = state_with(vec![
        rect("n1", "A", 0.0, 0.0, 10.0, 10.0),
        rect("n2", "B", 20.0, 0.0, 10.0, 10.0),
    ]);
    let before = collect_document_ids(&state.doc);
    let mut allocator = namespaced();
    state.selection.set = vec![NodeId::new("n1"), NodeId::new("n2")];
    state.selection.anchor = NodeId::new("n2");

    assert!(state
        .apply_with_allocator(
            EditorCommand::DuplicateSelected { offset_px: 10 },
            &mut allocator,
        )
        .unwrap());
    assert!(state
        .apply_with_allocator(EditorCommand::GroupSelected, &mut allocator)
        .unwrap());
    assert!(state.copy_selected());
    assert!(state
        .apply_with_allocator(
            EditorCommand::PasteClipboard { offset_px: 10 },
            &mut allocator,
        )
        .unwrap());
    assert!(state
        .apply_with_allocator(
            EditorCommand::ImportSvg {
                svg: r#"<svg><rect width="10" height="10"/></svg>"#.to_string(),
                x: 0,
                y: 0,
                target_parent: NodeId::NONE,
                page_id: None,
            },
            &mut allocator,
        )
        .unwrap());
    assert!(state
        .apply_with_allocator(
            EditorCommand::AddPage {
                name: None,
                children: None,
            },
            &mut allocator,
        )
        .unwrap());
    assert!(state
        .apply_with_allocator(
            EditorCommand::DuplicatePage {
                index: 1,
                name: None,
            },
            &mut allocator,
        )
        .unwrap());

    assert_new_ids_are_namespaced(&before, &state);
}

#[test]
fn component_conversion_uikit_and_refine_commands_use_the_namespace() {
    let mut component_state = sample();
    assert!(component_state.create_component_from_node(&NodeId::new("n10"), "Hero"));
    let component_before = collect_document_ids(&component_state.doc);
    let mut component_allocator = namespaced();
    assert!(component_state
        .apply_with_allocator(
            EditorCommand::InstantiateComponent {
                component_id: NodeId::new("n10"),
            },
            &mut component_allocator,
        )
        .unwrap());
    assert_new_ids_are_namespaced(&component_before, &component_state);

    let mut kit_state = EditorState::new();
    let kit_before = collect_document_ids(&kit_state.doc);
    let mut kit_allocator = namespaced();
    assert!(kit_state
        .apply_with_allocator(
            EditorCommand::InstantiateKitComponent {
                kit_id: "openpencil-starter".to_string(),
                component_id: "btn-primary".to_string(),
                doc_x: Some(10.0),
                doc_y: Some(20.0),
                target_parent: NodeId::NONE,
                page_id: None,
                overrides_json: None,
            },
            &mut kit_allocator,
        )
        .unwrap());
    assert_new_ids_are_namespaced(&kit_before, &kit_state);

    for command in [
        EditorCommand::UpsertScreen {
            key: "home".to_string(),
            root: Box::new(frame(
                "screen",
                "Screen",
                0.0,
                0.0,
                100.0,
                100.0,
                vec![rect("screen-child", "Child", 0.0, 0.0, 10.0, 10.0)],
            )),
            source_path: None,
            source_hash: None,
        },
        EditorCommand::UpsertComponent {
            key: "button".to_string(),
            name: "Button".to_string(),
            root: Box::new(frame(
                "component",
                "Component",
                0.0,
                0.0,
                100.0,
                40.0,
                vec![rect("component-child", "Child", 0.0, 0.0, 10.0, 10.0)],
            )),
            source_path: None,
            source_hash: None,
        },
    ] {
        let mut state = EditorState::new();
        let before = collect_document_ids(&state.doc);
        let mut allocator = namespaced();
        assert!(state.apply_with_allocator(command, &mut allocator).unwrap());
        assert_new_ids_are_namespaced(&before, &state);
    }

    let mut refine_state = state_with(vec![frame(
        "root",
        "Root",
        0.0,
        0.0,
        100.0,
        100.0,
        vec![
            rect("duplicate", "A", 0.0, 0.0, 10.0, 10.0),
            rect("duplicate", "B", 20.0, 0.0, 10.0, 10.0),
        ],
    )]);
    let refine_before = collect_document_ids(&refine_state.doc);
    let mut refine_allocator = namespaced();
    assert!(refine_state
        .apply_with_allocator(
            EditorCommand::RefineDesign {
                root_id: NodeId::new("root"),
                canvas_width: None,
                page_id: None,
            },
            &mut refine_allocator,
        )
        .unwrap());
    assert_new_ids_are_namespaced(&refine_before, &refine_state);
}

#[test]
fn host_creation_seams_share_the_supplied_namespace() {
    let mut state = EditorState::new();
    let before = collect_document_ids(&state.doc);
    let mut allocator = namespaced();
    let first = state
        .create_node_for_tool_with_allocator(Tool::Pen, &mut allocator, 0.0, 0.0, 10.0, 10.0)
        .unwrap()
        .unwrap();
    let second = state
        .create_node_for_tool_with_allocator(Tool::Pen, &mut allocator, 20.0, 0.0, 10.0, 10.0)
        .unwrap()
        .unwrap();
    state
        .replace_paths_with_polyline_with_allocator(
            &[first, second],
            &[vec![(0.0, 0.0), (10.0, 10.0)]],
            &mut allocator,
        )
        .unwrap()
        .unwrap();
    state
        .insert_image_node_at_viewport_with_allocator("Image", "data:", &mut allocator)
        .unwrap()
        .unwrap();
    state
        .insert_icon_font_node_at_with_allocator("circle", "lucide", 0.0, 0.0, &mut allocator)
        .unwrap()
        .unwrap();
    state
        .insert_icon_node_at_with_allocator(
            "circle",
            "remote",
            Some("M 0 0 L 1 1"),
            0.0,
            0.0,
            &mut allocator,
        )
        .unwrap()
        .unwrap();

    assert_new_ids_are_namespaced(&before, &state);
}

#[test]
fn apply_batch_rolls_back_on_late_allocator_exhaustion() {
    let mut state = EditorState::new();
    let before_doc = state.doc.clone();
    let before_history = state.history.clone();
    let before_revision = state.revision;
    let mut allocator =
        DocumentIdAllocator::namespaced(PeerNamespace::try_from("audit").unwrap(), u64::MAX - 1);

    assert_eq!(
        state.apply_with_allocator(
            EditorCommand::Batch {
                commands: vec![insert_command("First"), insert_command("Second")],
            },
            &mut allocator,
        ),
        Err(IdAllocError::CounterExhausted),
    );
    assert_eq!(state.doc, before_doc);
    assert_eq!(state.history.past, before_history.past);
    assert_eq!(state.history.future, before_history.future);
    assert_eq!(state.revision, before_revision);
}

#[test]
fn command_dispatch_has_no_legacy_allocator_escape_hatch() {
    let source = [
        include_str!("command_apply.rs"),
        include_str!("command_apply/helpers.rs"),
    ]
    .concat();
    for required in [
        "cmd_insert_node_with_allocator",
        "cmd_copy_node_with_allocator",
        "cmd_replace_node_with_allocator",
        "cmd_replace_subtree_with_allocator",
        "cmd_batch_insert_with_allocator",
        "cmd_insert_subtree_with_allocator",
        "cmd_refine_design_with_allocator",
        "upsert_component_with_allocator",
        "upsert_screen_with_allocator",
        "add_page_with_allocator",
        "duplicate_page_with_allocator",
        "duplicate_selected_with_allocator",
        "group_selected_with_allocator",
        "paste_clipboard_with_allocator",
        "apply_import_svg_on_active_page",
        "instantiate_component_with_allocator",
        "apply_kit_component_on_page",
        "cmd_batch_with_allocator",
    ] {
        assert!(
            source.contains(required),
            "missing allocator route: {required}"
        );
    }
    for forbidden in [
        "self.next_node_id_seed()",
        "self.duplicate_selected(",
        "self.group_selected(",
        "self.paste_clipboard(",
        "self.instantiate_component(",
        "self.add_page_with_name_and_children(",
        "self.duplicate_page_with_name(",
        "self.cmd_batch(",
    ] {
        assert!(
            !source.contains(forbidden),
            "legacy allocator route returned: {forbidden}",
        );
    }

    // InsertAuthoredSubtree is intentionally absent: it preserves
    // caller-authored ids and the collaboration gate rejects that bulk path.
}
