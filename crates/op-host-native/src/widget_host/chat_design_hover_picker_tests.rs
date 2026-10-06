//! Model-picker half of the chat hover suite: the picker as a floating
//! overlay, its precedence over the panels it covers, and what closing
//! it releases.
//!
//! Sibling of `chat_design_hover_tests.rs`, split out at the 800-line cap.

use super::*;

#[test]
fn model_picker_extension_is_a_floating_overlay() {
    let mut host = WidgetHostNative::new();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    // The minimum-height floating panel whose top the picker climbed
    // past is retired. The picker's floating-overlay nature is asserted
    // where an extension still exists: opened from the composer card —
    // a 112 px dock — the capped 288 px picker climbs far above the
    // card over the canvas, and every painted pixel of it must stay
    // interactive (owned by the overlay tier, not the canvas).
    open_populated_model_picker(&mut host);

    let card = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("composer card");
    let picker = host
        .chat_model_picker_rect(viewport_w, viewport_h)
        .expect("model picker rect");
    let point = Point2D::new(picker.origin.x + 8.0, picker.origin.y + 8.0);

    assert!(
        !card.contains(point),
        "the capped picker should extend above the composer card"
    );
    assert!(host.over_floating_overlay(point.x, point.y, viewport_w, viewport_h));
}

#[test]
fn open_model_picker_blocks_layer_panel_hover() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    open_populated_model_picker(&mut host);
    host.editor_state_mut().editor_ui.hovered_layer_id = Some(NodeId::new("stale"));
    let point = Point2D::new(40.0, 180.0);

    assert!(!host.cursor_over_layer_panel(point.x, point.y, viewport_w, viewport_h));
    assert!(host.update_layer_hover(point.x, point.y, viewport_w, viewport_h));
    assert_eq!(host.editor_state().editor_ui.hovered_layer_id, None);
}

#[test]
fn open_model_picker_owns_hover_without_repainting_unchanged_rows() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    open_populated_model_picker(&mut host);
    let picker = host
        .chat_model_picker_rect(viewport_w, viewport_h)
        .expect("model picker rect");
    let point = Point2D::new(
        picker.origin.x + 80.0,
        picker.origin.y
            + ai_chat_model_picker::MODEL_SEARCH_H
            + ai_chat_model_picker::MODEL_PICKER_PAD_Y
            + ai_chat_model_picker::MODEL_GROUP_H
            + ai_chat_model_picker::MODEL_ROW_H / 2.0,
    );
    {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.canvas_hover_node = Some(NodeId::new("stale-canvas"));
        ui.property_action_hover = Some(0);
        ui.chat_header_hover = Some(op_editor_core::ChatHeaderButton::NewChat);
        ui.chat_tab_hover = Some(0);
        ui.chat_footer_hover = Some(op_editor_core::ChatFooterButton::Send);
        ui.chat_example_hover = Some(0);
        ui.parallel_agents_picker_hover = Some(1);
        ui.variables_panel_hover = Some(op_editor_core::VariablesPanelButton::Close);
        ui.variables_preset_menu_hover =
            Some(op_editor_core::variables_panel_state::PresetMenuButton::SaveCurrent);
        ui.topbar_button_hover = Some(op_editor_core::TopBarButton::ToggleSidebar);
        ui.topbar_traffic_hover = true;
    }

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(
        host.editor_state().editor_ui.chat_model_picker.hover,
        Some(0)
    );
    assert_eq!(host.editor_state().editor_ui.canvas_hover_node, None);
    assert_eq!(host.editor_state().editor_ui.property_action_hover, None);
    assert_eq!(host.editor_state().editor_ui.chat_header_hover, None);
    assert_eq!(host.editor_state().editor_ui.chat_tab_hover, None);
    assert_eq!(host.editor_state().editor_ui.chat_footer_hover, None);
    assert_eq!(host.editor_state().editor_ui.chat_example_hover, None);
    assert_eq!(
        host.editor_state().editor_ui.parallel_agents_picker_hover,
        None
    );
    assert_eq!(host.editor_state().editor_ui.variables_panel_hover, None);
    assert_eq!(
        host.editor_state().editor_ui.variables_preset_menu_hover,
        None
    );
    assert_eq!(host.editor_state().editor_ui.topbar_button_hover, None);
    assert!(!host.editor_state().editor_ui.topbar_traffic_hover);

    assert!(
        !host.apply_cursor_move(point.x, point.y),
        "an unchanged picker row must stop dispatch without forcing another repaint"
    );
    assert_eq!(
        host.editor_state().editor_ui.chat_model_picker.hover,
        Some(0)
    );
}

#[test]
fn leaving_higher_context_menu_updates_model_picker_in_same_move() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    open_populated_model_picker(&mut host);
    host.editor_state_mut().ui.path_anchor_menu = Some(PathAnchorMenuState {
        node_id: NodeId::new("n1"),
        anchor_index: 0,
        x: 80.0,
        y: 80.0,
        menu: Default::default(),
    });
    host.editor_state_mut()
        .ui
        .path_anchor_menu
        .as_mut()
        .expect("menu open")
        .menu
        .hover = Some(0);
    let picker = host
        .chat_model_picker_rect(viewport_w, viewport_h)
        .expect("model picker rect");
    let point = Point2D::new(
        picker.origin.x + 80.0,
        picker.origin.y
            + ai_chat_model_picker::MODEL_SEARCH_H
            + ai_chat_model_picker::MODEL_PICKER_PAD_Y
            + ai_chat_model_picker::MODEL_GROUP_H
            + ai_chat_model_picker::MODEL_ROW_H / 2.0,
    );

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(
        host.editor_state().editor_ui.chat_model_picker.hover,
        Some(0),
        "clearing the higher menu must not defer picker hover to another move"
    );
    assert_eq!(
        host.editor_state()
            .ui
            .path_anchor_menu
            .as_ref()
            .expect("menu remains open")
            .menu
            .hover,
        None
    );
}

#[test]
fn leaving_higher_floating_panel_updates_model_picker_in_same_move() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    open_populated_model_picker(&mut host);
    {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.design_md_panel.open = true;
        ui.design_md_panel.pos = Some((0.0, 0.0));
        ui.design_md_panel.hover = Some(op_editor_core::DesignMdButton::Close);
    }
    let picker = host
        .chat_model_picker_rect(viewport_w, viewport_h)
        .expect("model picker rect");
    let point = Point2D::new(
        picker.origin.x + 80.0,
        picker.origin.y
            + ai_chat_model_picker::MODEL_SEARCH_H
            + ai_chat_model_picker::MODEL_PICKER_PAD_Y
            + ai_chat_model_picker::MODEL_GROUP_H
            + ai_chat_model_picker::MODEL_ROW_H / 2.0,
    );
    assert!(
        !host
            .design_md_panel_rect(viewport_w, viewport_h)
            .expect("design panel")
            .contains(point),
        "probe must leave the higher panel"
    );

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(host.editor_state().editor_ui.design_md_panel.hover, None);
    assert_eq!(
        host.editor_state().editor_ui.chat_model_picker.hover,
        Some(0),
        "higher-panel hover cleanup must not defer picker hover"
    );
}

#[test]
fn model_picker_hover_wins_when_overlapping_variables_panel() {
    let mut host = WidgetHostNative::new();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    // The floating panel that could be dragged over the VariablesPanel
    // is retired; the picker overlap that remains real is the composer
    // card's picker climbing over a VariablesPanel the user stretched
    // tall. Derive the probe from both live rects.
    host.editor_state_mut().editor_ui.variables_panel_open = true;
    host.editor_state_mut().editor_ui.variables_panel_size = Some((744.0, 640.0));
    let variables_rect = host
        .variables_panel_rect(viewport_w, viewport_h)
        .expect("variables panel rect");
    open_populated_model_picker(&mut host);
    host.editor_state_mut().editor_ui.variables_panel_hover =
        Some(op_editor_core::VariablesPanelButton::Close);
    let picker = host
        .chat_model_picker_rect(viewport_w, viewport_h)
        .expect("model picker rect");
    let point = Point2D::new(
        picker.origin.x + 80.0,
        picker.origin.y
            + ai_chat_model_picker::MODEL_SEARCH_H
            + ai_chat_model_picker::MODEL_PICKER_PAD_Y
            + ai_chat_model_picker::MODEL_GROUP_H
            + ai_chat_model_picker::MODEL_ROW_H / 2.0,
    );
    assert!(
        variables_rect.contains(point),
        "probe must exercise the visual overlap between both panels"
    );

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(
        host.editor_state().editor_ui.chat_model_picker.hover,
        Some(0)
    );
    assert_eq!(host.editor_state().editor_ui.variables_panel_hover, None);
}

#[test]
fn open_model_picker_suppresses_chat_and_variables_resize_cursors() {
    let mut host = WidgetHostNative::new();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    // The floating panel's north resize gutter is retired. The resize
    // gutter a picker still covers is the VariablesPanel's south edge
    // under the composer card's picker: the popup painted over the
    // gutter owns the cursor, and closing it gives the gutter back.
    host.editor_state_mut().editor_ui.variables_panel_open = true;
    host.editor_state_mut().editor_ui.variables_panel_size = Some((744.0, 640.0));
    open_populated_model_picker(&mut host);
    let variables_rect = host
        .variables_panel_rect(viewport_w, viewport_h)
        .expect("variables panel rect");
    let picker = host
        .chat_model_picker_rect(viewport_w, viewport_h)
        .expect("model picker rect");
    let south_edge = variables_rect.origin.y + variables_rect.size.y;
    let overlap_lo = picker.origin.x.max(variables_rect.origin.x);
    let overlap_hi =
        (picker.origin.x + picker.size.x).min(variables_rect.origin.x + variables_rect.size.x);
    let point = Point2D::new((overlap_lo + overlap_hi) / 2.0, south_edge - 1.0);
    assert!(picker.contains(point), "picker must cover the south gutter");
    assert_eq!(
        host.cursor_hint(point.x, point.y, viewport_w, viewport_h),
        CursorHint::Default,
        "the popup painted over the resize gutter owns the cursor"
    );

    host.editor_state_mut().editor_ui.chat_model_picker.open = false;
    assert_eq!(
        host.cursor_hint(point.x, point.y, viewport_w, viewport_h),
        CursorHint::ResizeNs,
        "sanity: with the picker shut the gutter reads as a resize cursor"
    );
}

#[test]
fn model_picker_without_visible_bounds_closes_and_releases_layer_panel() {
    let mut host = WidgetHostNative::new();
    // The rail stays on its default Layers tab: at this viewport the
    // sidebar leaves the canvas no width, so the composer card — and
    // with it the picker — resolves no visible bounds at all.
    let (viewport_w, viewport_h) = (120.0, 120.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    open_populated_model_picker(&mut host);
    let point = Point2D::new(20.0, 60.0);

    assert!(
        host.chat_model_picker_rect(viewport_w, viewport_h)
            .is_none(),
        "the narrow viewport cannot lay out a visible chat picker"
    );
    assert!(
        host.cursor_over_layer_panel(point.x, point.y, viewport_w, viewport_h),
        "stale open state without painted bounds must not block the layer rail"
    );

    assert!(host.apply_cursor_move(point.x, point.y));
    assert!(
        !host.editor_state().editor_ui.chat_model_picker.open,
        "cursor dispatch should heal an invisible open picker"
    );
}
