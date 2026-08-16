//! Focused native-host regressions for touch account and collaboration entry points.

use super::*;

#[test]
fn anonymous_more_sign_in_queues_the_shell_login_request() {
    for (class, width, height) in [
        (EditorSizeClass::Compact, 320.0, 568.0),
        (EditorSizeClass::Medium, 834.0, 1_112.0),
        (EditorSizeClass::Expanded, 1_194.0, 834.0),
    ] {
        let mut host = touch_host(class);
        assert_eq!(
            host.editor_state().editor_ui.account,
            AccountState::Anonymous
        );
        assert!(!host.editor_state().editor_ui.account_ui_available);
        focus_property_owner(&mut host, PropertyKeyboardOwner::Property);

        assert!(press_more_entry(
            &mut host,
            MobileMoreEntry::SignIn,
            width,
            height,
        ));

        // Touch chrome hands the whole sign-in experience to the shell: the
        // engine-painted login modal never opens; the one-shot request lets
        // the shell configure auth lazily and present its native screen (or
        // a native unavailability notice).
        let ui = &host.editor_state().editor_ui;
        assert_eq!(ui.mobile_sheet, None, "{class:?}");
        assert!(ui.pending_mobile_login, "{class:?}");
        assert!(!ui.login_modal_open, "{class:?}");
        assert!(!ui.account_menu_open, "{class:?}");
        assert!(!ui.collab.panel.open, "{class:?}");
        assert!(!ui.collab.panel.join_address_focused, "{class:?}");
        assert_property_owners_released(&host);
        assert!(!host.text_input_focus_active());
    }
}

#[test]
fn begin_mobile_login_reports_stub_unavailability_without_a_modal() {
    let mut host = touch_host(EditorSizeClass::Compact);
    // Unconfigured backend fails closed.
    assert!(!host.begin_mobile_login());
    // The runtime gate is the host's authoritative signal. This test build
    // has the inert bridge, so an immediate begin settles to
    // Failed(Unavailable) instead of creating a handle; the shell surfaces
    // that natively and the modal stays closed.
    host.editor_state_mut().editor_ui.account_ui_available = true;
    assert!(!host.begin_mobile_login());
    let ui = &host.editor_state().editor_ui;
    assert!(!ui.login_modal_open);
    assert_eq!(
        ui.login_modal_status,
        Some(op_editor_core::LoginFlowStatus::Failed(
            op_editor_core::LoginFlowError::Unavailable,
        ))
    );
}

#[test]
fn signed_in_more_account_requests_the_native_account_center() {
    for (class, width, height) in [
        (EditorSizeClass::Compact, 390.0, 844.0),
        (EditorSizeClass::Medium, 834.0, 1_112.0),
        (EditorSizeClass::Expanded, 1_194.0, 834.0),
    ] {
        let mut host = touch_host(class);
        host.editor_state_mut().editor_ui.account = AccountState::SignedIn {
            display_name: "Fini".into(),
            username: "fini".into(),
        };
        assert!(!host.editor_state().editor_ui.account_ui_available);

        assert!(press_more_entry(
            &mut host,
            MobileMoreEntry::Account,
            width,
            height,
        ));

        let state = host.editor_state();
        assert_eq!(state.editor_ui.mobile_sheet, None, "{class:?}");
        // The SSO account experience on phones is platform-native: the tile
        // queues a one-shot shell request instead of opening the painted
        // desktop Settings surface.
        assert!(state.editor_ui.pending_account_center, "{class:?}");
        assert!(!state.editor_ui.agent_settings_open, "{class:?}");
    }
}

#[test]
fn unavailable_more_collaboration_opens_a_modal_panel_without_queueing_actions() {
    for (class, width, height) in [
        (EditorSizeClass::Compact, 390.0, 844.0),
        (EditorSizeClass::Medium, 834.0, 1_112.0),
        (EditorSizeClass::Expanded, 1_194.0, 834.0),
    ] {
        let mut host = touch_host(class);
        assert_eq!(
            host.editor_state().editor_ui.collab.availability,
            CollabAvailability::Unavailable
        );

        assert!(press_more_entry(
            &mut host,
            MobileMoreEntry::Collaboration,
            width,
            height,
        ));
        assert_eq!(host.editor_state().editor_ui.mobile_sheet, None);
        assert!(host.editor_state().editor_ui.collab.panel.open);
        assert_eq!(host.editor_state().editor_ui.collab.pending_action, None);

        let panel = CollabPanel::for_editor_ui(&host.editor_state().editor_ui)
            .expect("unavailable panel remains visible");
        let panel_rect = op_editor_ui::widgets::touch_overlay_geometry::collaboration_panel_rect(
            host.editor_state(),
            &panel,
            width,
            height,
        );
        let inert_body = Point2D::new(
            panel_rect.origin.x + panel_rect.size.x / 2.0,
            panel_rect.origin.y + panel_rect.size.y / 2.0,
        );
        assert!(host.apply_press(inert_body.x, inert_body.y, width, height));
        assert!(host.editor_state().editor_ui.collab.panel.open);
        assert_eq!(host.editor_state().editor_ui.collab.pending_action, None);

        let close = Point2D::new(
            panel_rect.origin.x + panel_rect.size.x - 22.0,
            panel_rect.origin.y + 22.0,
        );
        assert!(host.apply_press(close.x, close.y, width, height));
        assert!(!host.editor_state().editor_ui.collab.panel.open);
        assert_eq!(host.editor_state().editor_ui.collab.pending_action, None);
    }
}
