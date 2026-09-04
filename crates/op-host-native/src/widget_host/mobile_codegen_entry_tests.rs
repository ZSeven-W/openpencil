//! Tablet Code inspector entry coverage.

use super::*;

#[test]
fn tablet_more_code_opens_codegen_without_a_selection() {
    for (class, width, height, expected_sheet) in [
        (
            EditorSizeClass::Medium,
            834.0,
            1112.0,
            Some(MobileSheetKind::Properties),
        ),
        (EditorSizeClass::Expanded, 1194.0, 834.0, None),
    ] {
        let mut host = touch_host(class);
        host.editor_state_mut().clear_selection();
        host.editor_state_mut()
            .editor_ui
            .set_property_tab(op_editor_core::PropertyTab::Design);

        assert!(press_more_entry(
            &mut host,
            MobileMoreEntry::Code,
            width,
            height,
        ));

        let state = host.editor_state();
        assert!(state.selection.is_empty(), "{class:?}");
        assert_eq!(
            state.editor_ui.effective_property_tab(),
            op_editor_core::PropertyTab::Code,
            "{class:?}"
        );
        assert_eq!(state.editor_ui.mobile_sheet, expected_sheet, "{class:?}");
        assert!(state.property_panel_visible(), "{class:?}");
        assert!(
            op_editor_ui::widgets::PropertyPanel::for_selection(state).is_some(),
            "{class:?} must build the selection-independent Code inspector"
        );
    }
}

#[test]
fn compact_more_cannot_reach_codegen() {
    let mut host = touch_host(EditorSizeClass::Compact);
    let entries = MobileMoreEntry::visible(host.editor_state());
    assert!(!entries.contains(&MobileMoreEntry::Code));

    assert!(!host
        .editor_state_mut()
        .editor_ui
        .set_property_tab(op_editor_core::PropertyTab::Code));
    assert_eq!(
        host.editor_state().editor_ui.effective_property_tab(),
        op_editor_core::PropertyTab::Design
    );
}

#[test]
fn expanded_selected_node_can_return_from_code_to_design() {
    let mut host = touch_host(EditorSizeClass::Expanded);
    let (width, height) = (1194.0, 834.0);
    let selected = NodeId::new("n10");
    host.editor_state_mut()
        .set_single_selection(selected.clone());

    assert!(press_more_entry(
        &mut host,
        MobileMoreEntry::Code,
        width,
        height,
    ));
    assert_eq!(
        host.editor_state().editor_ui.effective_property_tab(),
        op_editor_core::PropertyTab::Code
    );
    assert_eq!(host.editor_state().selection.anchor, selected);

    let actions = op_editor_ui::widgets::mobile_chrome::selection_actions_rect_for(
        host.editor_state(),
        width,
        height,
    );
    let design = Point2D::new(actions.origin.x + 36.0, actions.origin.y + 22.0);
    assert!(host.apply_press(design.x, design.y, width, height));
    assert_eq!(
        host.editor_state().editor_ui.effective_property_tab(),
        op_editor_core::PropertyTab::Design
    );
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n10"));
    assert!(host.editor_state().property_panel_visible());
}
