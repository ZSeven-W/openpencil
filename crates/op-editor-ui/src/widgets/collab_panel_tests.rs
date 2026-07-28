use super::*;
use op_editor_core::{
    AuthenticatedCollabSession, CollabAdmissionRequestKey, CollabAvailability,
    CollabConnectionPhase, CollabParticipantUi, CollabShareEndpoint, CollabUiRole,
};

fn viewport() -> Rect {
    Rect::xywh(0.0, 0.0, 1_000.0, 800.0)
}

fn active_ui(role: CollabUiRole, share_endpoint: Option<CollabShareEndpoint>) -> EditorUiState {
    let mut ui = EditorUiState::default();
    ui.collab.availability = CollabAvailability::Ready;
    ui.collab.panel.open = true;
    ui.collab.set_authenticated_session(
        CollabConnectionPhase::Active,
        AuthenticatedCollabSession {
            session_name: "Design".into(),
            role,
            share_endpoint,
        },
        Vec::new(),
    );
    ui
}

#[test]
fn authenticated_panel_exposes_real_leave_action() {
    let mut ui = EditorUiState::default();
    ui.collab.availability = CollabAvailability::Ready;
    ui.collab.panel.open = true;
    ui.collab.set_authenticated_session(
        CollabConnectionPhase::Active,
        AuthenticatedCollabSession {
            session_name: "Design".into(),
            role: CollabUiRole::Editor,
            share_endpoint: None,
        },
        vec![CollabParticipantUi::new(
            "p1",
            "Ada",
            0x3366ffff,
            CollabUiRole::Editor,
            true,
        )],
    );
    let panel = CollabPanel::for_editor_ui(&ui).unwrap();
    let rect = panel.rect_at(Rect::xywh(600.0, 8.0, 100.0, 26.0), viewport());
    let (leave, _) = panel
        .action_rects(rect)
        .into_iter()
        .find(|(_, action)| action.action == CollabUiAction::Leave)
        .expect("active session has leave");
    assert_eq!(
        panel.hit_test(
            rect,
            Point2D::new(
                leave.origin.x + leave.size.x / 2.0,
                leave.origin.y + leave.size.y / 2.0,
            )
        ),
        Some(CollabPanelHit::Action(CollabUiAction::Leave))
    );
}

#[test]
fn outside_point_is_not_claimed() {
    let mut ui = EditorUiState::default();
    ui.collab.panel.open = true;
    let panel = CollabPanel::for_editor_ui(&ui).unwrap();
    let rect = panel.rect_at(Rect::xywh(600.0, 8.0, 100.0, 26.0), viewport());
    assert_eq!(panel.hit_test(rect, Point2D::new(2.0, 700.0)), None);
}

#[test]
fn owner_can_hit_approve_viewer_for_pending_request() {
    let mut ui = EditorUiState::default();
    ui.collab.availability = CollabAvailability::Ready;
    ui.collab.panel.open = true;
    ui.collab.set_authenticated_session(
        CollabConnectionPhase::Active,
        AuthenticatedCollabSession {
            session_name: "Design".into(),
            role: CollabUiRole::Owner,
            share_endpoint: CollabShareEndpoint::new("192.168.1.8:43120"),
        },
        Vec::new(),
    );
    let request_key = CollabAdmissionRequestKey::new("request-42").unwrap();
    assert!(ui
        .collab
        .publish_pending_admission(request_key.clone(), None));
    let panel = CollabPanel::for_editor_ui(&ui).unwrap();
    let rect = panel.rect_at(Rect::xywh(600.0, 8.0, 100.0, 26.0), viewport());
    let CollabPanelScreen::Session {
        admission_request: Some(request),
        ..
    } = &panel.model.screen
    else {
        panic!("expected owner admission request");
    };
    let expected = CollabUiAction::ApproveAdmissionViewer { request_key };
    let (button, _) = panel
        .admission_action_rects(rect, panel.body_top(rect), request)
        .into_iter()
        .find(|(_, action)| action.action == expected)
        .expect("viewer approval button");
    assert_eq!(
        panel.hit_test(
            rect,
            Point2D::new(
                button.origin.x + button.size.x / 2.0,
                button.origin.y + button.size.y / 2.0,
            )
        ),
        Some(CollabPanelHit::Action(expected))
    );
}

#[test]
fn owner_share_address_extends_panel_geometry() {
    let endpoint = CollabShareEndpoint::new("192.168.1.8:43120").unwrap();
    let owner_without_ui = active_ui(CollabUiRole::Owner, None);
    let owner_with_ui = active_ui(CollabUiRole::Owner, Some(endpoint));
    let owner_without = CollabPanel::for_editor_ui(&owner_without_ui).unwrap();
    let owner_with = CollabPanel::for_editor_ui(&owner_with_ui).unwrap();
    assert_eq!(
        owner_with.panel_height(),
        owner_without.panel_height() + SHARE_ENDPOINT_HEIGHT
    );
    assert_eq!(owner_with.session_share_height(), SHARE_ENDPOINT_HEIGHT);

    let guest_ui = active_ui(
        CollabUiRole::Viewer,
        CollabShareEndpoint::new("192.168.1.8:43120"),
    );
    let guest = CollabPanel::for_editor_ui(&guest_ui).unwrap();
    assert_eq!(guest.session_share_height(), 0.0);
    assert_eq!(guest.panel_height(), owner_without.panel_height());
}

#[test]
fn share_address_copy_hit_is_owner_only_and_redacts_debug() {
    let endpoint = "192.168.1.8:43120";
    let owner_ui = active_ui(CollabUiRole::Owner, CollabShareEndpoint::new(endpoint));
    let owner = CollabPanel::for_editor_ui(&owner_ui).unwrap();
    let panel_rect = owner.rect_at(Rect::xywh(600.0, 8.0, 100.0, 26.0), viewport());
    let copy = owner
        .share_endpoint_copy_rect(panel_rect)
        .expect("owner has copy target");
    let hit = owner
        .hit_test(
            panel_rect,
            Point2D::new(
                copy.origin.x + copy.size.x / 2.0,
                copy.origin.y + copy.size.y / 2.0,
            ),
        )
        .expect("copy target is hit");
    assert_eq!(hit, CollabPanelHit::CopyShareEndpoint(endpoint.to_string()));
    assert!(!format!("{hit:?}").contains(endpoint));

    let guest_ui = active_ui(CollabUiRole::Viewer, CollabShareEndpoint::new(endpoint));
    let guest = CollabPanel::for_editor_ui(&guest_ui).unwrap();
    assert!(guest.share_endpoint_copy_rect(panel_rect).is_none());
}

#[test]
fn share_address_label_is_localized() {
    assert_eq!(
        op_i18n::translate(op_editor_core::Locale::EnUs, "collab.session.shareAddress"),
        "Share address"
    );
    assert_eq!(
        op_i18n::translate(op_editor_core::Locale::ZhCn, "collab.session.shareAddress"),
        "分享地址"
    );
}
