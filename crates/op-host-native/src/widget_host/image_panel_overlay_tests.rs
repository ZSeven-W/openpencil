use super::WidgetHostNative;
use op_editor_core::agent_settings::SettingsFocus;
use op_editor_core::chat::{AgentProvider, ModelEntry};
use op_editor_core::{EditorState, NodeId};
use op_editor_ui::widgets::{
    LayerPanel, LayerPanelHit, PropertyPanel, TopBar, TopBarHit, TOP_BAR_HEIGHT,
};
use op_editor_ui::{Point2D, Rect};

const VIEWPORT_W: f32 = 1200.0;
const VIEWPORT_H: f32 = 800.0;

fn topbar_point(host: &mut WidgetHostNative, target: TopBarHit) -> Point2D {
    let rect = Rect::xywh(0.0, 0.0, VIEWPORT_W, TOP_BAR_HEIGHT);
    let mut topbar = TopBar::for_editor_ui(&host.editor_state().editor_ui);
    topbar.chip_text_w = Some(host.topbar_chip_text_w(&topbar));
    let mut x = rect.origin.x;
    while x < rect.origin.x + rect.size.x {
        let point = Point2D::new(x, TOP_BAR_HEIGHT / 2.0);
        if host.topbar_hit_test(&topbar, rect, point) == Some(target) {
            return point;
        }
        x += 1.0;
    }
    panic!("no topbar point for {target:?}");
}

fn layer_row_point(host: &WidgetHostNative, target: &NodeId) -> Point2D {
    // The rail carries a tab row now (对话 / 图层 / 幻灯片), so the tree
    // starts below it; ask the host rather than rebuilding the rect.
    let rect = host.layers_content_rect(VIEWPORT_W, VIEWPORT_H);
    let panel = LayerPanel::from_editor(host.editor_state());
    let mut y = rect.origin.y;
    while y < rect.origin.y + rect.size.y {
        let point = Point2D::new(rect.origin.x + 48.0, y);
        if panel.hit_test(rect, point) == Some(LayerPanelHit::Layer(target.clone())) {
            return point;
        }
        y += 1.0;
    }
    panic!("no layer row for {target:?}");
}

fn image_popup_point(panel: &PropertyPanel, rect: Rect) -> Point2D {
    let mut y = 0.0;
    while y <= VIEWPORT_H {
        let mut x = 0.0;
        while x <= VIEWPORT_W {
            let point = Point2D::new(x, y);
            if panel.image_popovers_contain(rect, point) {
                return point;
            }
            x += 2.0;
        }
        y += 2.0;
    }
    panic!("no image Search popover point");
}

#[test]
fn topbar_modal_takes_input_ownership_from_image_search() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    let mut state = EditorState::sample();
    let _ = state.insert_image_node_at_viewport("Hero photo", "https://x/y.png");
    state.editor_ui.image_panel.search_open = true;
    state
        .editor_ui
        .image_panel
        .search_query
        .set_text("hero query");
    *host.editor_state_mut() = state;
    let point = topbar_point(&mut host, TopBarHit::OpenAgentSettings);

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.editor_state().editor_ui.agent_settings_open);
    assert!(!host.editor_state().editor_ui.image_panel.search_open);

    let ui = &mut host.editor_state_mut().editor_ui;
    ui.agent_settings.focus = Some(SettingsFocus::McpPort);
    ui.settings_input.set_text("123");
    assert!(host.apply_ime_commit("4"));
    assert_eq!(host.editor_state().editor_ui.settings_input.text(), "1234");
    assert_eq!(
        host.editor_state()
            .editor_ui
            .image_panel
            .search_query
            .text(),
        "hero query"
    );
}

#[test]
fn selection_changing_right_press_closes_image_search() {
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[
          {"type":"rectangle","id":"n1","name":"Other","x":0,"y":0,"width":100,"height":50},
          {"type":"image","id":"n2","name":"Hero","x":120,"y":0,"width":100,"height":100,"src":"https://x/y.png"}
        ]}"#,
    )
    .expect("fixture")
    .value;
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    *host.editor_state_mut() = EditorState::from_document(doc);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n2"));
    host.editor_state_mut().editor_ui.image_panel.search_open = true;
    host.editor_state_mut()
        .editor_ui
        .image_panel
        .search_query
        .set_text("hero query");
    let point = layer_row_point(&host, &NodeId::new("n1"));

    assert!(host.apply_right_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n1"));
    assert!(!host.editor_state().editor_ui.image_panel.search_open);
    assert!(!host.apply_text('x'));
    assert_eq!(
        host.editor_state()
            .editor_ui
            .image_panel
            .search_query
            .text(),
        "hero query"
    );
}

#[test]
fn image_search_popup_wins_above_chat_picker_and_clears_covered_hover() {
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[
          {"type":"image","id":"hero","name":"Hero","x":120,"y":0,
           "width":100,"height":100,"src":"https://x/y.png"}
        ]}"#,
    )
    .expect("fixture")
    .value;
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    *host.editor_state_mut() = EditorState::from_document(doc);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("hero"));
    host.editor_state_mut().editor_ui.image_panel.search_open = true;
    host.editor_state_mut()
        .chat
        .available_models
        .push(ModelEntry::new(AgentProvider::CodexCli, "gpt-5", "GPT-5"));
    host.last_viewport_w = VIEWPORT_W;
    host.last_viewport_h = VIEWPORT_H;
    host.mark_paint_dirty_for_test();
    let property_rect = host.property_rect(VIEWPORT_W, VIEWPORT_H);
    let panel = PropertyPanel::for_selection(host.editor_state()).expect("property panel");
    let point = image_popup_point(&panel, property_rect);
    host.editor_state_mut().editor_ui.chat_model_picker.open = true;
    {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.chat_model_picker.hover = Some(0);
        ui.chat_header_hover = Some(op_editor_core::ChatHeaderButton::NewChat);
        ui.chat_footer_hover = Some(op_editor_core::ChatFooterButton::Send);
        ui.chat_example_hover = Some(0);
        ui.canvas_hover_node = Some(NodeId::new("stale-canvas"));
        ui.property_action_hover = Some(2);
    }
    // RETIRED PREMISE: the floating chat panel could be parked under
    // the Image Search popup to manufacture a covered-picker overlap.
    // The picker now lives over the composer card at the canvas floor
    // while the popup floats in the right property rail — disjoint
    // surfaces — so the popup's precedence is asserted where it still
    // bites: it owns its point outright and clears every Chat hover in
    // the same move.
    let picker = host
        .chat_model_picker_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("model picker rect");
    assert!(!picker.contains(point));
    assert!(
        !host.chat_panel_surface_contains(point.x, point.y, VIEWPORT_W, VIEWPORT_H),
        "no chat surface can sit under the popup any more"
    );

    println!("DBG picker {picker:?} point {point:?} property {property_rect:?}");
    assert!(host.apply_cursor_move(point.x, point.y));
    let ui = &host.editor_state().editor_ui;
    assert!(ui.image_panel.search_open, "the higher popup stays open");
    assert_eq!(ui.chat_model_picker.hover, None);
    assert_eq!(ui.chat_header_hover, None);
    assert_eq!(ui.chat_footer_hover, None);
    assert_eq!(ui.chat_example_hover, None);
    assert_eq!(ui.canvas_hover_node, None);
    assert_eq!(ui.property_action_hover, None);
    assert!(
        !host.apply_cursor_move(point.x, point.y),
        "stable Image Search popup ownership must not repaint"
    );
}
