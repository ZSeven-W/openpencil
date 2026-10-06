use super::*;
use op_editor_core::chat::{AgentProvider, ModelEntry};
use op_editor_core::{NodeId, PathAnchorMenuState, Tool};
use op_editor_ui::widgets::{
    ai_chat_model_picker, AIChatPlaceholder, StatusBar, AI_CHAT_MIN_HEIGHT,
};
use op_editor_ui::Point2D;

fn open_populated_model_picker(host: &mut WidgetHostNative) {
    for index in 0..10 {
        host.editor_state_mut()
            .chat
            .available_models
            .push(ModelEntry::new(
                AgentProvider::CodexCli,
                format!("gpt-{index}"),
                format!("GPT {index}"),
            ));
    }
    host.editor_state_mut().chat.panel_height = AI_CHAT_MIN_HEIGHT;
    host.editor_state_mut().editor_ui.chat_model_picker.open = true;
}

fn seed(host: &mut WidgetHostNative, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.mark_paint_dirty_for_test();
}

fn seed_chat_and_lower_hover(host: &mut WidgetHostNative) {
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.chat_model_picker.hover = Some(0);
    ui.chat_header_hover = Some(op_editor_core::ChatHeaderButton::NewChat);
    ui.chat_tab_hover = Some(0);
    ui.chat_design_block_hover = Some((0, 0));
    ui.chat_footer_hover = Some(op_editor_core::ChatFooterButton::Send);
    ui.chat_example_hover = Some(0);
    ui.parallel_agents_picker_hover = Some(1);
    ui.canvas_hover_node = Some(NodeId::new("stale-canvas"));
    ui.hovered_layer_id = Some(NodeId::new("stale-layer"));
    ui.hovered_page_index = Some(0);
    ui.toolbar_hover = Some(op_editor_core::toolbar_state::ToolbarHover::Tool(
        Tool::Select,
    ));
    ui.variables_panel_hover = Some(op_editor_core::VariablesPanelButton::Close);
    ui.variables_preset_menu_hover =
        Some(op_editor_core::variables_panel_state::PresetMenuButton::SaveCurrent);
    ui.property_action_hover = Some(2);
    ui.property_tab_hover = Some(op_editor_core::PropertyTab::Design);
    ui.fill_type_picker.hover = Some(0);
    ui.topbar_button_hover = Some(op_editor_core::TopBarButton::ToggleSidebar);
    ui.topbar_traffic_hover = true;
    host.editor_state_mut().codegen.framework_hover = Some(op_editor_core::codegen::Framework::Vue);
    host.editor_state_mut().codegen.action_hover =
        Some(op_editor_core::codegen::CodegenHover::Copy);
    host.last_hover_probe = Some((400.0, 400.0));
}

fn assert_chat_and_lower_hover_cleared(host: &WidgetHostNative) {
    let ui = &host.editor_state().editor_ui;
    assert_eq!(ui.chat_model_picker.hover, None);
    assert_eq!(ui.chat_header_hover, None);
    assert_eq!(ui.chat_tab_hover, None);
    assert_eq!(ui.chat_design_block_hover, None);
    assert_eq!(ui.chat_footer_hover, None);
    assert_eq!(ui.chat_example_hover, None);
    assert_eq!(ui.parallel_agents_picker_hover, None);
    assert_eq!(ui.canvas_hover_node, None);
    assert_eq!(ui.hovered_layer_id, None);
    assert_eq!(ui.hovered_page_index, None);
    assert_eq!(ui.toolbar_hover, None);
    assert_eq!(ui.variables_panel_hover, None);
    assert_eq!(ui.variables_preset_menu_hover, None);
    assert_eq!(ui.property_action_hover, None);
    assert_eq!(ui.property_tab_hover, None);
    assert_eq!(ui.fill_type_picker.hover, None);
    assert_eq!(ui.topbar_button_hover, None);
    assert!(!ui.topbar_traffic_hover);
    assert_eq!(host.editor_state().codegen.framework_hover, None);
    assert_eq!(host.editor_state().codegen.action_hover, None);
    assert_eq!(host.last_hover_probe, None);
}

#[test]
fn cursor_move_sets_chat_tab_hover_when_over_tab() {
    // Seed two tabs so the tab row renders (tab row only paints when
    // `tabs_snapshot.len() >= 1`). The default ChatSessions starts with
    // one implicit tab; `new_tab()` adds a second.
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    host.editor_state_mut().chat.new_tab(); // now 2 tabs
    let viewport_w = 1440.0_f32;
    let viewport_h = 900.0_f32;
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;

    let chat_rect = host.ai_chat_rect(viewport_w, viewport_h).unwrap();

    // Verify that `tab_hover_at` agrees with our probe point before wiring.
    // Tab 0 body starts at tab_row_left = rect.origin.x + PAD + CHEVRON_W + PILL_GAP
    // (8 + 18 + 6 = 32 px from the panel left edge).  Center of the first tab
    // body: x = tab_row_left + TAB_MAX_W / 2, y = rect.origin.y + HEADER_HEIGHT / 2.
    let over_tab0 = Point2D::new(
        chat_rect.origin.x + 32.0 + 60.0, // tab row left + half TAB_MAX_W
        chat_rect.origin.y + 18.0,        // header mid
    );
    let panel = AIChatPlaceholder::from_editor(host.editor_state());
    assert_eq!(
        panel.tab_hover_at(chat_rect, over_tab0),
        Some(0),
        "tab_hover_at must return tab index 0 for a point inside the first tab body"
    );

    // Now drive the host cursor move and confirm the state field is updated.
    assert!(host.apply_cursor_move(over_tab0.x, over_tab0.y));
    assert_eq!(
        host.editor_state().editor_ui.chat_tab_hover,
        Some(0),
        "apply_cursor_move must write chat_tab_hover = Some(0) when cursor is over tab 0"
    );

    // Moving off the panel clears the hover.
    assert!(host.apply_cursor_move(0.0, 0.0));
    assert_eq!(
        host.editor_state().editor_ui.chat_tab_hover,
        None,
        "apply_cursor_move must clear chat_tab_hover when cursor leaves the panel"
    );
}

#[test]
fn cursor_move_tracks_hovered_design_json_card_for_copy_reveal() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    host.editor_state_mut()
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
    let panel = AIChatPlaceholder::from_editor(host.editor_state()).owned_by(host.chat_panel_owner);
    let point = Point2D::new(chat_rect.origin.x + 24.0, chat_rect.origin.y + 52.0);
    assert_eq!(panel.design_block_hover_at(chat_rect, point), Some((0, 0)));

    assert!(host.apply_cursor_move(point.x, point.y));

    assert_eq!(
        host.editor_state().editor_ui.chat_design_block_hover,
        Some((0, 0))
    );
}

#[test]
fn cursor_move_tracks_chat_footer_buttons() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    host.editor_state_mut()
        .chat
        .available_models
        .push(op_editor_core::chat::ModelEntry::new(
            op_editor_core::chat::AgentProvider::CodexCli,
            "gpt-5",
            "GPT-5",
        ));
    host.editor_state_mut().chat.set_input_text("design");
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    let chat_rect = host.ai_chat_rect(viewport_w, viewport_h).unwrap();
    // Footer right cluster, laid out right-to-left from the panel edge:
    // ⚡ speed | 📎 attach | ↑ send (the inert palette slot is gone). With
    // PAD=16, a 28px send circle, 4px gaps and 24px icon buttons, the attach
    // icon centres at `size.x - 60` (send centre ≈ -30). The input text must
    // stay single-line at the panel's real inner width — a wrapped line adds
    // 20px to the input area and shifts the whole footer band up.
    let attach = Point2D::new(
        chat_rect.origin.x + chat_rect.size.x - 60.0,
        chat_rect.origin.y + chat_rect.size.y - 19.0,
    );
    let send = Point2D::new(
        chat_rect.origin.x + chat_rect.size.x - 28.0,
        chat_rect.origin.y + chat_rect.size.y - 19.0,
    );

    assert!(host.apply_cursor_move(attach.x, attach.y));
    assert_eq!(
        host.editor_state().editor_ui.chat_footer_hover,
        Some(op_editor_core::ChatFooterButton::AddAttachment)
    );

    assert!(host.apply_cursor_move(send.x, send.y));
    assert_eq!(
        host.editor_state().editor_ui.chat_footer_hover,
        Some(op_editor_core::ChatFooterButton::Send)
    );
}

#[test]
fn cursor_move_tracks_quick_action_card_hover() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    host.editor_state_mut()
        .chat
        .available_models
        .push(op_editor_core::chat::ModelEntry::new(
            op_editor_core::chat::AgentProvider::CodexCli,
            "gpt-5",
            "GPT-5",
        ));
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    let chat_rect = host.ai_chat_rect(viewport_w, viewport_h).unwrap();
    let over_first = Point2D::new(chat_rect.origin.x + 100.0, chat_rect.origin.y + 104.0);
    // The quick-action pills are now full-width and vertically stacked (#33/#43):
    // four 40px pills with 8px gaps start ~86px below the panel top, so the
    // stack reaches ~270px. y=260 lands on pill index 3; a point truly "off the
    // cards" must sit below the last pill.
    let off_cards = Point2D::new(chat_rect.origin.x + 24.0, chat_rect.origin.y + 280.0);

    assert!(host.apply_cursor_move(over_first.x, over_first.y));
    assert_eq!(host.editor_state().editor_ui.chat_example_hover, Some(0));

    assert!(host.apply_cursor_move(off_cards.x, off_cards.y));
    assert_eq!(host.editor_state().editor_ui.chat_example_hover, None);
}

#[test]
fn ordinary_chat_blank_surface_blocks_canvas_hover_and_stays_stable() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[
          {"type":"rectangle","id":"under-chat","name":"Under chat",
           "x":0,"y":0,"width":3000,"height":3000}
        ]}"#,
    );
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    host.editor_state_mut().chat.panel_position = Some((400.0, 280.0));
    let _ = host.layout_scene();

    let chat_rect = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("chat rect");
    // The empty middle of the header is a Chat-owned drag surface with no
    // per-control hover wash. It used to fall through to the giant node below.
    let point = Point2D::new(
        chat_rect.origin.x + chat_rect.size.x / 2.0,
        chat_rect.origin.y + 18.0,
    );
    let probe = AIChatPlaceholder::from_editor(host.editor_state())
        .owned_by(host.chat_panel_owner)
        .cursor_probe(chat_rect, point);
    assert!(probe.hit.is_some(), "blank header is owned by Chat");
    host.editor_state_mut().editor_ui.canvas_hover_node = Some(NodeId::new("stale"));
    host.last_hover_probe = Some((point.x, point.y));

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(host.editor_state().editor_ui.canvas_hover_node, None);
    assert_eq!(host.last_hover_probe, None);
    assert!(
        !host.apply_cursor_move(point.x, point.y),
        "an unchanged Chat-owned point must stop dispatch without repainting"
    );
    assert_eq!(host.editor_state().editor_ui.canvas_hover_node, None);
    assert_eq!(host.last_hover_probe, None);
}

#[test]
fn chat_control_hover_and_lower_cleanup_happen_in_the_same_move() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    let chat_rect = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("chat rect");
    let point = Point2D::new(chat_rect.origin.x + 100.0, chat_rect.origin.y + 104.0);
    host.editor_state_mut().editor_ui.canvas_hover_node = Some(NodeId::new("stale-canvas"));
    host.last_hover_probe = Some((point.x, point.y));

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(host.editor_state().editor_ui.chat_example_hover, Some(0));
    assert_eq!(host.editor_state().editor_ui.canvas_hover_node, None);
    assert_eq!(host.last_hover_probe, None);
}

#[test]
fn ordinary_chat_surface_blocks_layer_panel_predispatch() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    host.editor_state_mut().chat.panel_position = Some((20.0, 100.0));
    host.editor_state_mut().editor_ui.hovered_layer_id = Some(NodeId::new("stale-layer"));
    let point = Point2D::new(40.0, 180.0);

    assert!(host.chat_panel_surface_contains(point.x, point.y, viewport_w, viewport_h));
    assert!(!host.cursor_over_layer_panel(point.x, point.y, viewport_w, viewport_h));
    assert!(host.update_layer_hover(point.x, point.y, viewport_w, viewport_h));
    assert_eq!(host.editor_state().editor_ui.hovered_layer_id, None);
}

#[test]
fn ordinary_chat_surface_wins_when_overlapping_variables_panel() {
    let mut host = WidgetHostNative::new();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    // The floating panel that could be dragged over the VariablesPanel
    // is retired; the chat surface that can still overlap it is the
    // composer card at the canvas floor, under a VariablesPanel the
    // user stretched tall. The point is derived from BOTH live rects.
    host.editor_state_mut().editor_ui.variables_panel_open = true;
    host.editor_state_mut().editor_ui.variables_panel_size = Some((744.0, 760.0));
    let variables_rect = host
        .variables_panel_rect(viewport_w, viewport_h)
        .expect("variables panel rect");
    let card = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("composer card");
    host.editor_state_mut().editor_ui.variables_panel_hover =
        Some(op_editor_core::VariablesPanelButton::Close);
    let point = Point2D::new(
        card.origin.x + card.size.x / 2.0,
        (variables_rect.origin.y + variables_rect.size.y + card.origin.y) / 2.0,
    );

    assert!(
        variables_rect.contains(point),
        "probe must overlap both surfaces"
    );
    assert!(host.chat_panel_surface_contains(point.x, point.y, viewport_w, viewport_h));
    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(host.editor_state().editor_ui.variables_panel_hover, None);
}

#[test]
fn align_toolbar_whole_card_wins_over_chat_surface() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[
          {"type":"rectangle","id":"a","name":"A","x":0,"y":0,"width":40,"height":40},
          {"type":"rectangle","id":"b","name":"B","x":80,"y":0,"width":40,"height":40}
        ]}"#,
    );
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    host.editor_state_mut().selection.set = vec![NodeId::new("a"), NodeId::new("b")];
    host.editor_state_mut().selection.anchor = NodeId::new("b");
    let align_rect = host
        .align_toolbar_rect(viewport_w, viewport_h)
        .expect("align toolbar rect");
    // Side padding is inside the painted card but is not an action. This proves
    // the entire higher-painted toolbar consumes hover, not only its buttons.
    let point = Point2D::new(align_rect.origin.x + 1.0, align_rect.origin.y + 20.0);
    assert!(align_rect.contains(point));
    assert_eq!(
        host.selection_toolbar_hit(point.x, point.y, viewport_w, viewport_h),
        None
    );
    // RETIRED PREMISE: the floating chat panel could be dragged under
    // the toolbar to manufacture an overlap. Both chat surfaces are
    // docked now (rail body / composer card at the canvas floor), and
    // neither can reach the toolbar — so the whole-card guarantee is
    // asserted where it still bites: blank toolbar padding owns its
    // point and clears Chat hover state in the same move.
    let chat_rect = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("chat rect");
    let chat = AIChatPlaceholder::from_editor(host.editor_state());
    assert_eq!(chat.example_hover_at(chat_rect, point), None);
    assert!(!host.chat_panel_surface_contains(point.x, point.y, viewport_w, viewport_h));
    seed_chat_and_lower_hover(&mut host);

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(
        host.editor_state().editor_ui.chat_example_hover,
        None,
        "the blank AlignToolbar card must stop Chat hover dispatch"
    );
    assert_chat_and_lower_hover_cleared(&host);
    assert!(
        !host.apply_cursor_move(point.x, point.y),
        "stable blank-card hover must not request another repaint"
    );
}

#[test]
fn moving_from_chat_into_status_clears_chat_and_lower_hover_in_one_event() {
    let mut host = WidgetHostNative::new();
    // The maximized panel that covered the status bar is retired. The
    // real overlap on the new contract is the composer card meeting
    // the status bar at the canvas floor on a NARROW viewport — both
    // dock to the floor, and the canvas is too slim to keep them apart.
    let (viewport_w, viewport_h) = (700.0, 800.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    let status_rect = host
        .status_bar_rect(viewport_w, viewport_h)
        .expect("status bar rect");
    let point = Point2D::new(status_rect.origin.x + 10.0, status_rect.origin.y + 16.0);
    let expected = StatusBar::for_editor(host.editor_state())
        .control_at(status_rect, point)
        .expect("search control");
    assert!(host.chat_panel_surface_contains(point.x, point.y, viewport_w, viewport_h));
    seed_chat_and_lower_hover(&mut host);

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(
        host.editor_state().editor_ui.statusbar_hover,
        Some(expected)
    );
    assert_chat_and_lower_hover_cleared(&host);
    assert!(
        !host.apply_cursor_move(point.x, point.y),
        "stable StatusBar hover must not request another repaint"
    );
    assert_eq!(
        host.editor_state().editor_ui.statusbar_hover,
        Some(expected)
    );
}

#[test]
fn moving_from_chat_into_align_action_clears_chat_and_preserves_align_hover() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[
          {"type":"rectangle","id":"a","name":"A","x":0,"y":0,"width":40,"height":40},
          {"type":"rectangle","id":"b","name":"B","x":80,"y":0,"width":40,"height":40}
        ]}"#,
    );
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    host.editor_state_mut().selection.set = vec![NodeId::new("a"), NodeId::new("b")];
    host.editor_state_mut().selection.anchor = NodeId::new("b");
    let align_rect = host
        .align_toolbar_rect(viewport_w, viewport_h)
        .expect("align toolbar rect");
    let mut hit = None;
    let mut y = align_rect.origin.y;
    while y <= align_rect.origin.y + align_rect.size.y && hit.is_none() {
        let mut x = align_rect.origin.x;
        while x <= align_rect.origin.x + align_rect.size.x {
            if let Some(action) = host.align_toolbar_hit(x, y, viewport_w, viewport_h) {
                hit = Some((Point2D::new(x, y), action));
                break;
            }
            x += 2.0;
        }
        y += 2.0;
    }
    let (point, expected) = hit.expect("align action point");
    // RETIRED PREMISE: the maximized panel used to cover the toolbar,
    // so the move genuinely left the chat surface. The toolbar hangs
    // over the canvas top and no chat surface reaches it any more —
    // what is still load-bearing is the ONE-EVENT semantics: a stale
    // Chat hover is cleared by the same move that sets the Align hover.
    assert!(!host.chat_panel_surface_contains(point.x, point.y, viewport_w, viewport_h));
    seed_chat_and_lower_hover(&mut host);

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(
        host.editor_state().editor_ui.align_toolbar_hover,
        Some(expected)
    );
    assert_chat_and_lower_hover_cleared(&host);
    assert!(
        !host.apply_cursor_move(point.x, point.y),
        "stable AlignToolbar hover must not request another repaint"
    );
    assert_eq!(
        host.editor_state().editor_ui.align_toolbar_hover,
        Some(expected)
    );
}

#[path = "chat_design_hover_picker_tests.rs"]
mod picker_tests;
