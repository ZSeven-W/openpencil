use super::WidgetHost;
use op_editor_core::chat::{AgentProvider, ModelEntry};
use op_editor_core::{EditorState, NodeId, PathAnchorMenuState, Tool};
use op_editor_ui::widgets::{AIChatPlaceholder, AlignToolbar, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

fn open_model_picker(host: &mut WidgetHost) {
    host.editor_state
        .chat
        .available_models
        .push(ModelEntry::new(AgentProvider::CodexCli, "gpt-5", "GPT-5"));
    host.editor_state.editor_ui.chat_model_picker.open = true;
}

fn seed_two_selected_rects(host: &mut WidgetHost) {
    let mut state = EditorState::starter();
    state.active_children_mut().clear();
    let mut next_id = 100;
    let a = state
        .create_node_for_tool(Tool::Rect, &mut next_id, 0.0, 0.0, 120.0, 80.0)
        .expect("first rect");
    let b = state
        .create_node_for_tool(Tool::Rect, &mut next_id, 40.0, 0.0, 120.0, 80.0)
        .expect("second rect");
    state.selection.set = vec![a, b.clone()];
    state.selection.anchor = b;
    host.editor_state = state;
    host.editor_state_dirty = true;
}

fn seed_stale_chat_and_lower_hover(host: &mut WidgetHost) {
    let ui = &mut host.editor_state.editor_ui;
    ui.chat_header_hover = Some(op_editor_core::ChatHeaderButton::NewChat);
    ui.chat_tab_hover = Some(0);
    ui.chat_design_block_hover = Some((0, 0));
    ui.chat_footer_hover = Some(op_editor_core::ChatFooterButton::Send);
    ui.chat_example_hover = Some(0);
    ui.parallel_agents_picker_hover = Some(1);
    ui.hovered_layer_id = Some(NodeId::new("stale-layer"));
    ui.canvas_hover_node = Some(NodeId::new("stale-canvas"));
    ui.property_action_hover = Some(2);
}

fn assert_chat_and_lower_hover_cleared(host: &WidgetHost) {
    let ui = &host.editor_state.editor_ui;
    assert_eq!(ui.chat_header_hover, None);
    assert_eq!(ui.chat_tab_hover, None);
    assert_eq!(ui.chat_design_block_hover, None);
    assert_eq!(ui.chat_footer_hover, None);
    assert_eq!(ui.chat_example_hover, None);
    assert_eq!(ui.parallel_agents_picker_hover, None);
    assert_eq!(ui.hovered_layer_id, None);
    assert_eq!(ui.canvas_hover_node, None);
    assert_eq!(ui.property_action_hover, None);
}

#[test]
fn cursor_move_tracks_hovered_design_json_card_for_copy_reveal() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.enter_chat_tab();
    host.editor_state
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant(
            r#"```json
[{"id":"frame-1","type":"Frame"}]
```"#,
        ));
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    let chat_rect = host.ai_chat_rect(viewport_w, viewport_h).unwrap();
    // Stamp with the host's real owner, mirroring production: `design_block_hover_at`
    // resolves the transcript cache, which debug-asserts against the UNOWNED sentinel.
    let panel = AIChatPlaceholder::from_editor(&host.editor_state).owned_by(host.chat_panel_owner);
    let point = Point2D::new(chat_rect.origin.x + 24.0, chat_rect.origin.y + 52.0);
    assert_eq!(panel.design_block_hover_at(chat_rect, point), Some((0, 0)));

    assert!(host.apply_cursor_move(point.x, point.y));

    assert_eq!(
        host.editor_state.editor_ui.chat_design_block_hover,
        Some((0, 0))
    );
}

#[test]
fn cursor_move_tracks_chat_footer_buttons() {
    let mut host = WidgetHost::new();
    host.editor_state
        .chat
        .available_models
        .push(op_editor_core::chat::ModelEntry::new(
            op_editor_core::chat::AgentProvider::CodexCli,
            "gpt-5",
            "GPT-5",
        ));
    host.editor_state.chat.set_input_text("design");
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    let chat_rect = host.ai_chat_rect(viewport_w, viewport_h).unwrap();
    // Footer right cluster: ⚡ speed | 📎 attach | ↑ send, laid out
    // right-to-left (the inert palette slot is gone). The attach icon
    // centres at `size.x - 60` (send centre ≈ -30). The input text must
    // stay single-line — a wrapped line lifts the whole footer band.
    let attach = Point2D::new(
        chat_rect.origin.x + chat_rect.size.x - 60.0,
        chat_rect.origin.y + chat_rect.size.y - 19.0,
    );
    let send = Point2D::new(
        chat_rect.origin.x + chat_rect.size.x - 28.0,
        chat_rect.origin.y + chat_rect.size.y - 19.0,
    );

    host.editor_state.editor_ui.canvas_hover_node = Some(NodeId::new("stale-canvas"));
    host.editor_state.editor_ui.property_action_hover = Some(3);
    host.editor_state.editor_ui.design_md_panel.open = true;
    host.editor_state.editor_ui.design_md_panel.pos = Some((0.0, 0.0));
    host.editor_state.editor_ui.design_md_panel.hover = Some(op_editor_core::DesignMdButton::Close);

    assert!(host.apply_cursor_move(attach.x, attach.y));
    assert_eq!(
        host.editor_state.editor_ui.chat_footer_hover,
        Some(op_editor_core::ChatFooterButton::AddAttachment)
    );
    assert_eq!(host.editor_state.editor_ui.canvas_hover_node, None);
    assert_eq!(host.editor_state.editor_ui.property_action_hover, None);
    assert_eq!(host.editor_state.editor_ui.design_md_panel.hover, None);

    assert!(host.apply_cursor_move(send.x, send.y));
    assert_eq!(
        host.editor_state.editor_ui.chat_footer_hover,
        Some(op_editor_core::ChatFooterButton::Send)
    );
}

#[test]
fn chat_blank_surface_blocks_lower_hover_and_stable_move_needs_no_repaint() {
    let mut host = WidgetHost::new();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    let chat_rect = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("chat rect");
    // Eight pixels inside the side border is outside header buttons, footer
    // controls and empty-state cards, but still belongs to the opaque panel.
    let point = Point2D::new(
        chat_rect.origin.x + chat_rect.size.x - 8.0,
        chat_rect.origin.y + chat_rect.size.y / 2.0,
    );
    let panel = AIChatPlaceholder::from_editor(&host.editor_state).owned_by(host.chat_panel_owner);
    let probe = panel.cursor_probe(chat_rect, point);
    assert!(probe.hit.is_some(), "blank chat chrome must own the point");
    assert_eq!(panel.tab_hover_at(chat_rect, point), None);
    assert_eq!(panel.footer_hover_at(chat_rect, point), None);
    assert_eq!(panel.example_hover_at(chat_rect, point), None);

    {
        let ui = &mut host.editor_state.editor_ui;
        ui.hovered_layer_id = Some(NodeId::new("stale-layer"));
        ui.hovered_page_index = Some(0);
        ui.canvas_hover_node = Some(NodeId::new("stale-canvas"));
        ui.toolbar_hover = Some(op_editor_core::toolbar_state::ToolbarHover::Tool(
            Tool::Select,
        ));
        ui.variables_panel_hover = Some(op_editor_core::VariablesPanelButton::Close);
        ui.property_action_hover = Some(2);
        ui.property_tab_hover = Some(op_editor_core::PropertyTab::Design);
        ui.fill_type_picker.hover = Some(0);
    }

    assert!(host.apply_cursor_move(point.x, point.y));
    let ui = &host.editor_state.editor_ui;
    assert_eq!(ui.hovered_layer_id, None);
    assert_eq!(ui.hovered_page_index, None);
    assert_eq!(ui.canvas_hover_node, None);
    assert_eq!(ui.toolbar_hover, None);
    assert_eq!(ui.variables_panel_hover, None);
    assert_eq!(ui.property_action_hover, None);
    assert_eq!(ui.property_tab_hover, None);
    assert_eq!(ui.fill_type_picker.hover, None);
    assert!(
        !host.apply_cursor_move(point.x, point.y),
        "an unchanged owned move is still truncated without requesting repaint"
    );
}

#[test]
fn entering_chat_clears_stale_higher_and_lower_hover_in_one_move() {
    let mut host = WidgetHost::new();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    let chat_rect = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("chat rect");
    let point = Point2D::new(
        chat_rect.origin.x + chat_rect.size.x - 8.0,
        chat_rect.origin.y + chat_rect.size.y / 2.0,
    );

    host.editor_state.editor_ui.design_md_panel.open = true;
    host.editor_state.editor_ui.design_md_panel.pos = Some((0.0, 0.0));
    host.editor_state.editor_ui.design_md_panel.hover = Some(op_editor_core::DesignMdButton::Close);
    host.editor_state.editor_ui.import_menu_open = true;
    host.editor_state.editor_ui.import_menu.open = true;
    host.editor_state.editor_ui.import_menu.hover = Some(0);
    host.editor_state.ui.path_anchor_menu = Some(PathAnchorMenuState {
        node_id: NodeId::new("anchor-node"),
        anchor_index: 0,
        x: 80.0,
        y: 80.0,
        menu: Default::default(),
    });
    host.editor_state
        .ui
        .path_anchor_menu
        .as_mut()
        .expect("path menu")
        .menu
        .hover = Some(0);
    {
        let ui = &mut host.editor_state.editor_ui;
        ui.hovered_layer_id = Some(NodeId::new("stale-layer"));
        ui.canvas_hover_node = Some(NodeId::new("stale-canvas"));
        ui.property_action_hover = Some(4);
    }
    assert!(!host
        .design_md_panel_rect(viewport_w, viewport_h)
        .expect("design panel")
        .contains(point));
    assert!(!host
        .import_menu_rect(viewport_w, viewport_h)
        .contains(point));

    assert!(host.apply_cursor_move(point.x, point.y));
    let ui = &host.editor_state.editor_ui;
    assert_eq!(ui.design_md_panel.hover, None);
    assert_eq!(ui.import_menu.hover, None);
    assert_eq!(ui.hovered_layer_id, None);
    assert_eq!(ui.canvas_hover_node, None);
    assert_eq!(ui.property_action_hover, None);
    assert_eq!(
        host.editor_state
            .ui
            .path_anchor_menu
            .as_ref()
            .expect("path menu remains open")
            .menu
            .hover,
        None
    );
    assert!(
        !host.apply_cursor_move(point.x, point.y),
        "stable Chat ownership must not repaint after all exits are clear"
    );
}

#[test]
fn regular_chat_wins_when_overlapping_variables_panel() {
    let mut host = WidgetHost::new();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    host.editor_state.editor_ui.variables_panel_open = true;
    // A tall variables panel overlaps the floor-docked composer card.
    host.editor_state.editor_ui.variables_panel_size = Some((744.0, 760.0));
    host.editor_state.editor_ui.variables_panel_hover =
        Some(op_editor_core::VariablesPanelButton::Close);
    host.editor_state.editor_ui.canvas_hover_node = Some(NodeId::new("stale-canvas"));

    let chat = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("chat rect");
    let variables = host
        .variables_panel_rect(viewport_w, viewport_h)
        .expect("variables rect");
    let left = chat.origin.x.max(variables.origin.x);
    let top = chat.origin.y.max(variables.origin.y);
    let right = (chat.origin.x + chat.size.x).min(variables.origin.x + variables.size.x);
    let bottom = (chat.origin.y + chat.size.y).min(variables.origin.y + variables.size.y);
    assert!(
        right > left && bottom > top,
        "fixtures must visually overlap"
    );
    let point = Point2D::new((left + right) / 2.0, (top + bottom) / 2.0);
    assert!(
        AIChatPlaceholder::from_editor(&host.editor_state)
            .owned_by(host.chat_panel_owner)
            .cursor_probe(chat, point)
            .hit
            .is_some(),
        "the regular Chat surface must own the overlap point"
    );

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(host.editor_state.editor_ui.variables_panel_hover, None);
    assert_eq!(host.editor_state.editor_ui.canvas_hover_node, None);
}

#[test]
fn align_toolbar_padding_clears_chat_and_lower_hover() {
    let mut host = WidgetHost::new();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    seed_two_selected_rects(&mut host);
    host.editor_state.editor_ui.chat_header_hover = Some(op_editor_core::ChatHeaderButton::NewChat);
    host.editor_state.editor_ui.canvas_hover_node = Some(NodeId::new("stale-canvas"));

    let (cx, _, cw, ch) = host.canvas_region(viewport_w, viewport_h);
    let canvas_region = Rect {
        origin: Point2D::new(cx, TOP_BAR_HEIGHT),
        size: Point2D::new(cw, ch),
    };
    let toolbar =
        AlignToolbar::for_canvas_region(canvas_region, &host.editor_state).expect("align toolbar");
    let rect = toolbar.rect();
    let point = Point2D::new(rect.origin.x + 2.0, rect.origin.y + rect.size.y / 2.0);
    assert!(rect.contains(point));
    assert_eq!(
        toolbar.hit_test(point),
        None,
        "probe must land in opaque toolbar padding, not an action button"
    );
    // Docked chat no longer overlaps the toolbar; padding still owns the
    // pointer and must clear stale chat hover in the same move.
    assert!(!host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("composer card")
        .contains(point));

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(host.editor_state.editor_ui.chat_header_hover, None);
    assert_eq!(host.editor_state.editor_ui.canvas_hover_node, None);
    assert!(
        !host.apply_cursor_move(point.x, point.y),
        "stable AlignToolbar padding must truncate without repaint"
    );
}

#[test]
fn context_menu_footprint_clears_chat_and_lower_hover_in_one_move() {
    let mut host = WidgetHost::new();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    host.editor_state.editor_ui.enter_chat_tab();
    host.editor_state.ui.path_anchor_menu = Some(PathAnchorMenuState {
        node_id: NodeId::new("anchor-node"),
        anchor_index: 0,
        x: 40.0,
        y: 220.0,
        menu: Default::default(),
    });
    seed_stale_chat_and_lower_hover(&mut host);
    let menu = op_editor_ui::widgets::path_anchor_context_menu::PathAnchorContextMenu::for_state(
        &host.editor_state,
        host.editor_state
            .ui
            .path_anchor_menu
            .clone()
            .expect("path menu"),
    );
    let rect = menu.rect();
    let point = Point2D::new(rect.origin.x + 20.0, rect.origin.y + 20.0);
    assert!(rect.contains(point));
    assert!(AIChatPlaceholder::from_editor(&host.editor_state)
        .owned_by(host.chat_panel_owner)
        .cursor_probe(
            host.ai_chat_rect(viewport_w, viewport_h)
                .expect("pinned chat"),
            point,
        )
        .hit
        .is_some());

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_chat_and_lower_hover_cleared(&host);
    assert!(
        !host.apply_cursor_move(point.x, point.y),
        "stable context-menu ownership must not repaint"
    );
}

#[test]
fn status_bar_footprint_clears_chat_and_lower_hover_in_one_move() {
    let mut host = WidgetHost::new();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    host.editor_state.chat.maximized = true;
    seed_stale_chat_and_lower_hover(&mut host);
    let status = host
        .status_bar_rect(viewport_w, viewport_h)
        .expect("status bar");
    let point = Point2D::new(status.origin.x + 4.0, status.origin.y + status.size.y / 2.0);
    assert!(status.contains(point));

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_chat_and_lower_hover_cleared(&host);
    assert!(
        !host.apply_cursor_move(point.x, point.y),
        "stable StatusBar ownership must not repaint"
    );
}

#[test]
fn static_color_picker_clears_chat_and_lower_hover() {
    let mut host = WidgetHost::new();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    host.editor_state = EditorState::sample();
    host.editor_state.set_single_selection(NodeId::new("n13"));
    assert!(host
        .editor_state
        .open_color_picker(op_editor_core::ui_draft::ColorTarget::Fill, 220.0,));
    seed_stale_chat_and_lower_hover(&mut host);
    let state = host
        .editor_state
        .ui
        .color_picker
        .clone()
        .expect("color picker");
    let picker =
        op_editor_ui::widgets::color_picker::ColorPicker::for_state(&host.editor_state, state);
    let rect = picker.rect(viewport_w, viewport_h);
    let point = Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    );
    // The property popup and composer are disjoint; popup ownership must
    // still clear stale chat hover in one move.
    assert!(!host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("composer card")
        .contains(point));

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_chat_and_lower_hover_cleared(&host);
    assert!(host.editor_state.ui.color_picker.is_some());
}

#[test]
fn property_image_popup_clears_chat_model_picker_hover() {
    let mut host = WidgetHost::new();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    let _ = host
        .editor_state
        .insert_image_node_at_viewport("Hero photo", "https://x/y.png");
    host.editor_state.editor_ui.image_panel.search_open = true;
    open_model_picker(&mut host);
    host.editor_state.editor_ui.chat_model_picker.hover = Some(0);
    seed_stale_chat_and_lower_hover(&mut host);
    let panel = op_editor_ui::widgets::PropertyPanel::for_selection(&host.editor_state)
        .expect("image property panel");
    let property_rect = Rect {
        origin: Point2D::new(
            viewport_w - host.editor_state.editor_ui.property_panel_width,
            TOP_BAR_HEIGHT,
        ),
        size: Point2D::new(
            host.editor_state.editor_ui.property_panel_width,
            viewport_h - TOP_BAR_HEIGHT,
        ),
    };
    let chat_rect = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("composer card");
    let mut owned_point = None;
    let mut y = TOP_BAR_HEIGHT;
    while y < viewport_h && owned_point.is_none() {
        let mut x = 0.0;
        while x < viewport_w {
            let point = Point2D::new(x, y);
            if panel.image_popovers_contain(property_rect, point) {
                owned_point = Some(point);
                break;
            }
            x += 4.0;
        }
        y += 4.0;
    }
    let point = owned_point.expect("image search popup");
    assert!(!chat_rect.contains(point));
    assert!(!host
        .chat_model_picker_rect(viewport_w, viewport_h)
        .expect("model picker")
        .contains(point));

    assert!(host.apply_cursor_move(point.x, point.y));
    assert!(host.editor_state.editor_ui.image_panel.search_open);
    assert!(host.editor_state.editor_ui.chat_model_picker.open);
    assert_chat_and_lower_hover_cleared(&host);
    assert_eq!(host.editor_state.editor_ui.chat_model_picker.hover, None);
}

#[test]
fn cursor_move_tracks_chat_model_picker_row_hover() {
    use op_editor_ui::widgets::ai_chat_model_picker::{
        MODEL_GROUP_H, MODEL_PICKER_PAD_Y, MODEL_ROW_H, MODEL_SEARCH_H,
    };

    let mut host = WidgetHost::new();
    host.editor_state
        .chat
        .available_models
        .push(op_editor_core::chat::ModelEntry::new(
            op_editor_core::chat::AgentProvider::CodexCli,
            "gpt-5",
            "GPT-5",
        ));
    host.editor_state.editor_ui.chat_model_picker.open = true;
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    let chat_rect = host.ai_chat_rect(viewport_w, viewport_h).unwrap();
    let picker = AIChatPlaceholder::from_editor(&host.editor_state)
        .model_picker_bounds(chat_rect)
        .unwrap();
    let row = Point2D::new(
        picker.origin.x + 48.0,
        picker.origin.y + MODEL_SEARCH_H + MODEL_PICKER_PAD_Y + MODEL_GROUP_H + MODEL_ROW_H / 2.0,
    );

    assert!(host.apply_cursor_move(row.x, row.y));

    assert_eq!(host.editor_state.editor_ui.chat_model_picker.hover, Some(0));
}

#[test]
fn open_model_picker_blocks_layer_and_lower_hover_dispatch() {
    let mut host = WidgetHost::new();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    open_model_picker(&mut host);
    {
        let ui = &mut host.editor_state.editor_ui;
        ui.hovered_layer_id = Some(NodeId::new("stale-layer"));
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
    }
    let point = Point2D::new(20.0, 180.0);

    assert!(host.apply_cursor_move(point.x, point.y));
    let ui = &host.editor_state.editor_ui;
    assert_eq!(ui.hovered_layer_id, None);
    assert_eq!(ui.canvas_hover_node, None);
    assert_eq!(ui.property_action_hover, None);
    assert_eq!(ui.chat_header_hover, None);
    assert_eq!(ui.chat_tab_hover, None);
    assert_eq!(ui.chat_footer_hover, None);
    assert_eq!(ui.chat_example_hover, None);
    assert_eq!(ui.parallel_agents_picker_hover, None);
    assert_eq!(ui.variables_panel_hover, None);
    assert_eq!(ui.variables_preset_menu_hover, None);
    assert!(
        !host.apply_cursor_move(point.x, point.y),
        "an unchanged picker miss must still truncate dispatch without repainting"
    );
}

#[test]
fn leaving_higher_context_menu_updates_model_picker_in_same_move() {
    use op_editor_ui::widgets::ai_chat_model_picker::{
        MODEL_GROUP_H, MODEL_PICKER_PAD_Y, MODEL_ROW_H, MODEL_SEARCH_H,
    };

    let mut host = WidgetHost::new();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    open_model_picker(&mut host);
    host.editor_state.ui.path_anchor_menu = Some(PathAnchorMenuState {
        node_id: NodeId::new("n1"),
        anchor_index: 0,
        x: 80.0,
        y: 80.0,
        menu: Default::default(),
    });
    host.editor_state
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
        picker.origin.x + 48.0,
        picker.origin.y + MODEL_SEARCH_H + MODEL_PICKER_PAD_Y + MODEL_GROUP_H + MODEL_ROW_H / 2.0,
    );

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(host.editor_state.editor_ui.chat_model_picker.hover, Some(0));
    assert_eq!(
        host.editor_state
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
    use op_editor_ui::widgets::ai_chat_model_picker::{
        MODEL_GROUP_H, MODEL_PICKER_PAD_Y, MODEL_ROW_H, MODEL_SEARCH_H,
    };

    let mut host = WidgetHost::new();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    open_model_picker(&mut host);
    host.editor_state.editor_ui.design_md_panel.open = true;
    host.editor_state.editor_ui.design_md_panel.pos = Some((0.0, 0.0));
    host.editor_state.editor_ui.design_md_panel.hover = Some(op_editor_core::DesignMdButton::Close);
    let picker = host
        .chat_model_picker_rect(viewport_w, viewport_h)
        .expect("model picker rect");
    let point = Point2D::new(
        picker.origin.x + 48.0,
        picker.origin.y + MODEL_SEARCH_H + MODEL_PICKER_PAD_Y + MODEL_GROUP_H + MODEL_ROW_H / 2.0,
    );
    assert!(
        !host
            .design_md_panel_rect(viewport_w, viewport_h)
            .expect("design panel")
            .contains(point),
        "probe must leave the higher panel"
    );

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(host.editor_state.editor_ui.design_md_panel.hover, None);
    assert_eq!(host.editor_state.editor_ui.chat_model_picker.hover, Some(0));
}

#[test]
fn import_menu_owns_hover_above_model_picker() {
    let mut host = WidgetHost::new();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    open_model_picker(&mut host);
    host.editor_state.editor_ui.chat_model_picker.hover = Some(0);
    host.editor_state.editor_ui.import_menu_open = true;
    host.editor_state.editor_ui.import_menu.open = true;
    let menu = host.import_menu_rect(viewport_w, viewport_h);
    let point = Point2D::new(menu.origin.x + 20.0, menu.origin.y + 20.0);

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(host.editor_state.editor_ui.chat_model_picker.hover, None);
    assert_eq!(host.editor_state.editor_ui.import_menu.hover, Some(0));
}

#[test]
fn model_picker_hover_wins_when_overlapping_variables_panel() {
    let mut host = WidgetHost::new();
    let (viewport_w, viewport_h) = (1440.0, 600.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    host.editor_state.editor_ui.variables_panel_open = true;
    open_model_picker(&mut host);
    host.editor_state.editor_ui.variables_panel_hover =
        Some(op_editor_core::VariablesPanelButton::Close);
    let variables_rect = host
        .variables_panel_rect(viewport_w, viewport_h)
        .expect("variables panel rect");
    let picker = host
        .chat_model_picker_rect(viewport_w, viewport_h)
        .expect("model picker rect");
    let point = Point2D::new(
        picker.origin.x + 48.0,
        picker.origin.y
            + op_editor_ui::widgets::ai_chat_model_picker::MODEL_SEARCH_H
            + op_editor_ui::widgets::ai_chat_model_picker::MODEL_PICKER_PAD_Y
            + op_editor_ui::widgets::ai_chat_model_picker::MODEL_GROUP_H
            + op_editor_ui::widgets::ai_chat_model_picker::MODEL_ROW_H / 2.0,
    );
    assert!(
        variables_rect.contains(point),
        "probe must exercise the visual overlap between both panels"
    );

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(host.editor_state.editor_ui.chat_model_picker.hover, Some(0));
    assert_eq!(host.editor_state.editor_ui.variables_panel_hover, None);
}

#[test]
fn model_picker_without_visible_bounds_closes_and_releases_layer_hover() {
    let mut host = WidgetHost::new();
    let (viewport_w, viewport_h) = (120.0, 120.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    open_model_picker(&mut host);
    host.editor_state.editor_ui.hovered_layer_id = Some(NodeId::new("stale-layer"));
    let point = Point2D::new(20.0, 60.0);

    assert!(
        host.chat_model_picker_rect(viewport_w, viewport_h)
            .is_none(),
        "the narrow viewport cannot lay out a visible chat picker"
    );
    assert!(
        host.update_layer_hover(point.x, point.y, viewport_h),
        "stale open state without bounds must not block layer hover cleanup"
    );
    assert_eq!(host.editor_state.editor_ui.hovered_layer_id, None);

    assert!(host.apply_cursor_move(point.x, point.y));
    assert!(
        !host.editor_state.editor_ui.chat_model_picker.open,
        "cursor dispatch should heal an invisible open picker"
    );
}
