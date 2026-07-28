use super::WidgetHostNative;
use jian_ops_schema::PenDocument;
use op_editor_core::{
    AuthenticatedCollabSession, CollabAvailability, CollabConnectionPhase, CollabNoticeKind,
    CollabShareEndpoint, CollabUiRole, EditOrigin, PeerNamespace, Tool,
};
use op_editor_ui::widgets::{CollabPanel, TopBar, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

fn set_active_role(host: &mut WidgetHostNative, role: CollabUiRole) {
    assert!(host
        .editor_state_mut()
        .editor_ui
        .collab
        .set_authenticated_session(
            CollabConnectionPhase::Active,
            AuthenticatedCollabSession {
                session_name: "Test session".to_string(),
                role,
                share_endpoint: None,
            },
            Vec::new(),
        ));
}

fn document_with_rect(id: &str) -> PenDocument {
    serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{
            "type": "rectangle",
            "id": id,
            "name": "Rect",
            "x": 0,
            "y": 0,
            "width": 10,
            "height": 10
        }]
    }))
    .expect("valid test document")
}

fn expected_pen_rejection(role: CollabUiRole) -> CollabNoticeKind {
    match role {
        CollabUiRole::Viewer => {
            CollabNoticeKind::Reject(op_editor_core::CollabRejectUiCode::ReadOnly)
        }
        CollabUiRole::Owner | CollabUiRole::Editor => CollabNoticeKind::UnsupportedEdit(
            op_editor_core::CollabUnsupportedFeature::UnsupportedNodeProperty,
        ),
    }
}

#[test]
fn owner_share_copy_queues_clipboard_text_in_the_pointer_event() {
    let endpoint = "192.168.1.8:43120";
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.collab.availability = CollabAvailability::Ready;
    host.editor_state_mut().editor_ui.collab.panel.open = true;
    assert!(host
        .editor_state_mut()
        .editor_ui
        .collab
        .set_authenticated_session(
            CollabConnectionPhase::Active,
            AuthenticatedCollabSession {
                session_name: "Test".into(),
                role: CollabUiRole::Owner,
                share_endpoint: CollabShareEndpoint::new(endpoint),
            },
            Vec::new(),
        ));
    let viewport = Rect::xywh(0.0, 0.0, 1_000.0, 800.0);
    let topbar_rect = Rect::xywh(0.0, 0.0, viewport.size.x, TOP_BAR_HEIGHT);
    let topbar = TopBar::for_editor_ui(&host.editor_state().editor_ui);
    let panel = CollabPanel::for_editor_ui(&host.editor_state().editor_ui).unwrap();
    let panel_rect = panel.rect_at(
        topbar.collaboration_chip_rect_estimated(topbar_rect),
        viewport,
    );
    let copy = panel
        .share_endpoint_copy_rect(panel_rect)
        .expect("owner copy target");

    assert!(host.apply_press(
        copy.origin.x + copy.size.x / 2.0,
        copy.origin.y + copy.size.y / 2.0,
        viewport.size.x,
        viewport.size.y,
    ));
    assert_eq!(
        host.editor_state().chat.pending_copy_text.as_deref(),
        Some(endpoint)
    );
    assert!(host
        .editor_state()
        .editor_ui
        .collab
        .pending_action
        .is_none());
}

#[test]
fn supported_native_creation_paths_share_one_collaboration_allocator() {
    let mut host = WidgetHostNative::new();
    host.enable_collaboration_ids(PeerNamespace::try_from("peer-a").unwrap())
        .unwrap();
    set_active_role(&mut host, CollabUiRole::Editor);

    host.editor_state_mut().tool = Tool::Rect;
    let rect = host
        .create_node_for_active_tool(Point2D::new(10.0, 20.0))
        .expect("rectangle created");
    assert_eq!(rect.as_str(), "c_peer-a_0");

    host.editor_state_mut().set_single_selection(rect);
    assert!(host.apply_group());
    assert_eq!(host.editor_state().selection.anchor.as_str(), "c_peer-a_1");
    assert_eq!(host.collaboration_id_next_counter(), Some(2));
}

#[test]
fn viewer_and_unsupported_duplicate_fail_closed_without_consuming_ids() {
    let mut host = WidgetHostNative::new();
    host.enable_collaboration_ids(PeerNamespace::try_from("viewer").unwrap())
        .unwrap();
    set_active_role(&mut host, CollabUiRole::Viewer);
    host.editor_state_mut().tool = Tool::Rect;
    let before = host.editor_state().doc.clone();

    assert!(host
        .create_node_for_active_tool(Point2D::new(1.0, 2.0))
        .is_none());
    assert_eq!(&host.editor_state().doc, &before);
    assert_eq!(host.collaboration_id_next_counter(), Some(0));

    set_active_role(&mut host, CollabUiRole::Editor);
    let rect = host
        .create_node_for_active_tool(Point2D::new(1.0, 2.0))
        .expect("editor may create");
    host.editor_state_mut().set_single_selection(rect);
    let before = host.editor_state().doc.clone();
    assert!(host.apply_duplicate());
    assert_eq!(&host.editor_state().doc, &before);
    assert_eq!(host.collaboration_id_next_counter(), Some(1));
    assert!(matches!(
        host.editor_state()
            .editor_ui
            .collab
            .notice
            .map(|notice| notice.kind),
        Some(CollabNoticeKind::UnsupportedEdit(
            op_editor_core::CollabUnsupportedFeature::Duplicate
        ))
    ));
}

#[test]
fn exhausted_namespace_keeps_snapshot_readable_and_creation_atomic() {
    let mut host = WidgetHostNative::new();
    host.enable_collaboration_ids(PeerNamespace::try_from("peer").unwrap())
        .unwrap();
    set_active_role(&mut host, CollabUiRole::Editor);
    host.install_collaboration_document(
        document_with_rect("c_peer_18446744073709551615"),
        EditOrigin::Snapshot,
    )
    .unwrap();

    host.editor_state_mut().tool = Tool::Rect;
    let before = host.editor_state().doc.clone();
    assert!(host
        .create_node_for_active_tool(Point2D::new(10.0, 20.0))
        .is_none());
    assert_eq!(&host.editor_state().doc, &before);
    assert_eq!(host.collaboration_id_next_counter(), Some(u64::MAX));
    assert!(matches!(
        host.editor_state()
            .editor_ui
            .collab
            .notice
            .map(|notice| notice.kind),
        Some(CollabNoticeKind::Reject(
            op_editor_core::CollabRejectUiCode::ResourceLimit
        ))
    ));
}

#[test]
fn viewer_property_step_discards_draft_without_mutating_document() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().doc = document_with_rect("rect");
    host.editor_state_mut()
        .set_single_selection(op_editor_core::NodeId::new("rect"));
    set_active_role(&mut host, CollabUiRole::Viewer);
    host.editor_state_mut().ui.property_focus = Some(op_editor_core::PropertyFocus::PositionX);
    host.editor_state_mut().ui.property_input.set_text("10");
    host.editor_state_mut().ui.property_input_draft = "10".to_string();
    let before = host.editor_state().doc.clone();

    assert!(host.apply_property_step(1.0));
    assert_eq!(host.editor_state().doc, before);
    assert!(host.editor_state().ui.property_focus.is_none());
    assert!(matches!(
        host.editor_state()
            .editor_ui
            .collab
            .notice
            .map(|notice| notice.kind),
        Some(CollabNoticeKind::Reject(
            op_editor_core::CollabRejectUiCode::ReadOnly
        ))
    ));
}

#[test]
fn pen_start_is_hard_gated_for_every_collaboration_role() {
    for (role, namespace) in [
        (CollabUiRole::Owner, "pen-owner"),
        (CollabUiRole::Editor, "pen-editor"),
        (CollabUiRole::Viewer, "pen-viewer"),
    ] {
        let mut host = WidgetHostNative::new();
        host.enable_collaboration_ids(PeerNamespace::try_from(namespace).unwrap())
            .unwrap();
        set_active_role(&mut host, role);
        host.editor_state_mut().tool = Tool::Pen;
        let before = host.editor_state().doc.clone();
        let revision = host.editor_state().document_revision();

        assert!(host.apply_pen_tool_press(10.0, 10.0, 800.0, 600.0));
        assert!(host
            .create_node_for_active_tool(Point2D::new(20.0, 20.0))
            .is_none());

        assert_eq!(host.editor_state().doc, before, "{role:?}");
        assert_eq!(
            host.editor_state().document_revision(),
            revision,
            "{role:?}"
        );
        assert!(host.editor_state().ui.pen_in_progress.is_none(), "{role:?}");
        assert_eq!(host.collaboration_id_next_counter(), Some(0), "{role:?}");
        assert_eq!(
            host.editor_state()
                .editor_ui
                .collab
                .notice
                .map(|notice| notice.kind),
            Some(expected_pen_rejection(role)),
            "{role:?}"
        );
    }
}

#[test]
fn every_pen_mutation_sink_rechecks_all_roles_after_a_standalone_start() {
    for (role, namespace) in [
        (CollabUiRole::Owner, "race-owner"),
        (CollabUiRole::Editor, "race-editor"),
        (CollabUiRole::Viewer, "race-viewer"),
    ] {
        let mut host = WidgetHostNative::new();
        host.enable_collaboration_ids(PeerNamespace::try_from(namespace).unwrap())
            .unwrap();
        host.editor_state_mut().tool = Tool::Pen;
        assert!(host.apply_pen_tool_press(10.0, 10.0, 800.0, 600.0));
        host.apply_pen_release();
        assert!(host.apply_pen_tool_press(40.0, 40.0, 800.0, 600.0));

        set_active_role(&mut host, role);
        let before = host.editor_state().doc.clone();
        let revision = host.editor_state().document_revision();
        let next_id = host.collaboration_id_next_counter();

        assert_eq!(host.apply_pen_cursor_move(80.0, 80.0), Some(true));
        assert!(host.apply_pen_tool_press(70.0, 70.0, 800.0, 600.0));
        assert!(host.apply_pen_backspace());
        assert_eq!(host.apply_pen_enter(), Some(true));
        host.apply_set_tool(Tool::Rect);
        assert_eq!(host.editor_state().tool, Tool::Pen, "{role:?}");
        assert!(host.apply_pen_escape());

        assert_eq!(host.editor_state().doc, before, "{role:?}");
        assert_eq!(
            host.editor_state().document_revision(),
            revision,
            "{role:?}"
        );
        assert_eq!(host.collaboration_id_next_counter(), next_id, "{role:?}");
        assert!(host.editor_state().ui.pen_in_progress.is_some(), "{role:?}");
        assert_eq!(
            host.editor_state()
                .editor_ui
                .collab
                .notice
                .map(|notice| notice.kind),
            Some(expected_pen_rejection(role)),
            "{role:?}"
        );
    }
}
