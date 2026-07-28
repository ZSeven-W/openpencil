use crate::fills::{set_primary_fill_hex, set_primary_stroke_hex};
use crate::test_support::{frame, rect};
use crate::{
    DocumentInstallError, EditOrigin, EditorState, NodeId, PropertyFocus, SelectionState, Tool,
};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::page::PenPage;
use jian_ops_schema::PenDocument;
use std::collections::BTreeMap;

fn page(id: &str, children: Vec<PenNode>) -> PenPage {
    PenPage {
        id: id.into(),
        name: id.into(),
        children,
        background_color: None,
        state: None,
        lifecycle: None,
    }
}

fn paged(pages: Vec<PenPage>) -> PenDocument {
    let mut doc = EditorState::new().doc;
    doc.pages = Some(pages);
    doc
}

#[test]
fn remote_install_preserves_logical_page_selection_and_editor_chrome() {
    let old = paged(vec![
        page("p1", vec![rect("n1", "One", 0.0, 0.0, 10.0, 10.0)]),
        page("p2", vec![rect("n2", "Two", 0.0, 0.0, 10.0, 10.0)]),
    ]);
    let mut state = EditorState::from_document(old);
    state.ui.active_page_index = 1;
    state.selection = SelectionState {
        anchor: NodeId::new("missing"),
        set: vec![NodeId::new("n2"), NodeId::new("missing")],
    };
    state.viewport.pan_x = 41.0;
    state.viewport.zoom = 2.0;
    state.tool = Tool::Pen;
    state.editor_ui.sidebar_open = false;
    state.editor_ui.agent_settings.mcp_server.port = 4_321;
    state.chat.input.set_text("keep chat");
    state.clipboard = vec![rect("clip", "Clipboard", 0.0, 0.0, 1.0, 1.0)];
    state.commit_history();
    let generation = state.document_generation();
    let history_len = state.history.past.len();
    let revision = state.document_revision();

    let new = paged(vec![
        page(
            "p2",
            vec![
                rect("n2", "Two changed", 1.0, 2.0, 10.0, 10.0),
                rect("n3", "Three", 0.0, 0.0, 10.0, 10.0),
            ],
        ),
        page("p1", vec![rect("n1", "One", 0.0, 0.0, 10.0, 10.0)]),
    ]);
    let report = state
        .install_verified_document(new, EditOrigin::RemoteCommit)
        .unwrap();

    assert_eq!(state.ui.active_page_index, 0);
    assert_eq!(state.selection.set, vec![NodeId::new("n2")]);
    assert_eq!(state.selection.anchor, NodeId::new("n2"));
    assert_eq!(report.retained_selection, 1);
    assert!(!report.active_page_changed);
    assert_eq!(state.viewport.pan_x, 41.0);
    assert_eq!(state.viewport.zoom, 2.0);
    assert_eq!(state.tool, Tool::Pen);
    assert!(!state.editor_ui.sidebar_open);
    assert_eq!(state.editor_ui.agent_settings.mcp_server.port, 4_321);
    assert_eq!(state.chat.input.text(), "keep chat");
    assert_eq!(state.clipboard.len(), 1);
    assert_eq!(state.document_generation(), generation);
    assert_eq!(state.history.past.len(), history_len);
    assert!(state.document_revision() > revision);
    assert!(state.is_dirty());
}

#[test]
fn install_rebuilds_components_color_refs_and_clears_document_drafts() {
    let mut reusable = frame("component", "Component", 0.0, 0.0, 100.0, 100.0, vec![]);
    let PenNode::Frame(component_frame) = &mut reusable else {
        unreachable!();
    };
    component_frame.reusable = Some(true);
    let mut token_rect = rect("token", "Token", 0.0, 0.0, 10.0, 10.0);
    assert!(set_primary_fill_hex(&mut token_rect, "$accent"));
    assert!(set_primary_stroke_hex(&mut token_rect, "$border"));
    let mut doc = paged(vec![page("p1", vec![reusable, token_rect])]);
    doc.themes = Some(BTreeMap::from([(
        "mode".into(),
        vec!["light".into(), "dark".into()],
    )]));

    let mut state = EditorState::new();
    state
        .ui
        .variables
        .active_theme
        .insert("mode".into(), "dark".into());
    state.ui.property_focus = Some(PropertyFocus::PositionX);
    state.ui.property_input.set_text("stale");
    state.ui.text_editing = Some(NodeId::new("old"));
    state.ui.pen_in_progress = Some(NodeId::new("old"));
    state.editor_ui.hovered_layer_id = Some(NodeId::new("old"));
    state.app_state_owner.insert("counter".into(), 4);

    state
        .install_verified_document(doc, EditOrigin::Replay)
        .unwrap();

    assert!(state
        .components
        .find_by_id(&NodeId::new("component"))
        .is_some());
    assert_eq!(
        state
            .ui
            .variables
            .fill_refs
            .get(&NodeId::new("token"))
            .map(String::as_str),
        Some("accent")
    );
    assert_eq!(
        state
            .ui
            .variables
            .stroke_refs
            .get(&NodeId::new("token"))
            .map(String::as_str),
        Some("border")
    );
    assert_eq!(
        state
            .ui
            .variables
            .active_theme
            .get("mode")
            .map(String::as_str),
        Some("dark")
    );
    assert!(state.ui.property_focus.is_none());
    assert!(state.ui.property_input.text().is_empty());
    assert!(state.ui.text_editing.is_none());
    assert!(state.ui.pen_in_progress.is_none());
    assert!(state.editor_ui.hovered_layer_id.is_none());
    assert!(state.app_state_owner.is_empty());
}

#[test]
fn snapshot_install_clears_history_and_bumps_generation() {
    let mut state = EditorState::new();
    state.commit_history();
    let generation = state.document_generation();
    let revision = state.document_revision();
    let mut snapshot = state.doc.clone();
    snapshot.name = Some("snapshot".into());

    state
        .install_verified_document(snapshot, EditOrigin::Snapshot)
        .unwrap();

    assert!(!state.history.can_undo());
    assert!(!state.history.can_redo());
    assert_eq!(state.document_generation(), generation + 1);
    assert!(state.document_revision() > revision);
    assert!(state.is_dirty());
}

#[test]
fn local_install_is_one_undoable_document_change() {
    let mut state = EditorState::new();
    state.doc.name = Some("before".into());
    let mut after = state.doc.clone();
    after.name = Some("after".into());

    state
        .install_verified_document(after, EditOrigin::Local)
        .unwrap();

    assert_eq!(state.doc.name.as_deref(), Some("after"));
    assert!(state.undo());
    assert_eq!(state.doc.name.as_deref(), Some("before"));
}

#[test]
fn invalid_install_is_all_or_nothing() {
    let mut state = EditorState::new();
    state.doc.name = Some("live".into());
    state.viewport.pan_y = 88.0;
    state.commit_history();
    let before = state.doc.clone();
    let revision = state.document_revision();
    let generation = state.document_generation();
    let history_len = state.history.past.len();
    let invalid = paged(vec![page(
        "p1",
        vec![
            rect("duplicate", "A", 0.0, 0.0, 1.0, 1.0),
            rect("duplicate", "B", 0.0, 0.0, 1.0, 1.0),
        ],
    )]);

    assert_eq!(
        state.install_verified_document(invalid, EditOrigin::Snapshot),
        Err(DocumentInstallError::DuplicateId {
            id: "duplicate".into(),
        })
    );
    assert_eq!(state.doc, before);
    assert_eq!(state.document_revision(), revision);
    assert_eq!(state.document_generation(), generation);
    assert_eq!(state.history.past.len(), history_len);
    assert_eq!(state.viewport.pan_y, 88.0);
}

#[test]
fn explicit_pages_reject_root_content_and_empty_page_ids() {
    let mut state = EditorState::new();
    let mut mixed = paged(vec![page("p1", vec![])]);
    mixed.children = vec![rect("n1", "Root", 0.0, 0.0, 1.0, 1.0)];
    assert_eq!(
        state.install_verified_document(mixed, EditOrigin::RemoteCommit),
        Err(DocumentInstallError::ExplicitPagesWithRootChildren)
    );

    let empty_page = paged(vec![page("", vec![])]);
    assert_eq!(
        state.install_verified_document(empty_page, EditOrigin::RemoteCommit),
        Err(DocumentInstallError::EmptyPageId { page_index: 0 })
    );
}
