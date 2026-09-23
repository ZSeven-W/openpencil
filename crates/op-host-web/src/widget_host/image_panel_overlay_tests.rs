use super::WidgetHost;
use op_editor_core::{EditorState, NodeId, SettingsFocus};
use op_editor_ui::widgets::{LayerPanel, LayerPanelHit, TopBarHit, TOP_BAR_HEIGHT};
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 1200.0;
const VIEWPORT_H: f32 = 800.0;

#[test]
fn topbar_modal_takes_input_ownership_from_image_search() {
    let mut host = WidgetHost::new();
    let _ = host
        .editor_state
        .insert_image_node_at_viewport("Hero photo", "https://x/y.png");
    host.editor_state.editor_ui.image_panel.search_open = true;
    host.editor_state
        .editor_ui
        .image_panel
        .search_query
        .set_text("hero query");

    let rect = host.top_bar_rect(VIEWPORT_W);
    let topbar = host.top_bar();
    let mut point = None;
    let mut x = rect.origin.x;
    while x < rect.origin.x + rect.size.x {
        let candidate = Point2D::new(x, TOP_BAR_HEIGHT / 2.0);
        if topbar.hit_test(rect, candidate) == Some(TopBarHit::OpenAgentSettings) {
            point = Some(candidate);
            break;
        }
        x += 1.0;
    }
    let point = point.expect("settings button");
    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.editor_state.editor_ui.agent_settings_open);
    assert!(!host.editor_state.editor_ui.image_panel.search_open);

    host.editor_state.editor_ui.agent_settings.focus = Some(SettingsFocus::McpPort);
    host.editor_state.editor_ui.settings_input.set_text("123");
    assert!(host.apply_paste_text("4"));
    assert_eq!(host.editor_state.editor_ui.settings_input.text(), "1234");
    assert_eq!(
        host.editor_state.editor_ui.image_panel.search_query.text(),
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
    let mut host = WidgetHost::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state.editor_ui.enter_chat_tab();
    host.editor_state = EditorState::from_document(doc);
    host.editor_state.set_single_selection(NodeId::new("n2"));
    host.editor_state.editor_ui.image_panel.search_open = true;
    host.editor_state
        .editor_ui
        .image_panel
        .search_query
        .set_text("hero query");

    // The rail carries a tab row now (Agent / Layers / Slides), so the
    // tree starts below it; ask the host rather than rebuilding the rect.
    let rect = host.layers_content_rect(VIEWPORT_H);
    let panel = LayerPanel::from_editor(&host.editor_state);
    let mut point = None;
    let mut y = rect.origin.y;
    while y < rect.origin.y + rect.size.y {
        let candidate = Point2D::new(rect.origin.x + 48.0, y);
        if panel.hit_test(rect, candidate) == Some(LayerPanelHit::Layer(NodeId::new("n1"))) {
            point = Some(candidate);
            break;
        }
        y += 1.0;
    }
    let point = point.expect("other layer row");
    assert!(host.apply_right_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state.selection.anchor, NodeId::new("n1"));
    assert!(!host.editor_state.editor_ui.image_panel.search_open);
    assert!(!host.apply_text('x'));
    assert_eq!(
        host.editor_state.editor_ui.image_panel.search_query.text(),
        "hero query"
    );
}

#[test]
fn image_popover_edges_preserve_utf8_and_shift_selection() {
    let mut host = WidgetHost::new();
    let panel = &mut host.editor_state.editor_ui.image_panel;
    panel.search_open = true;
    panel.search_query.set_text("a你bc");
    panel.search_query.set_caret(1, 0);

    assert!(host.apply_image_panel_edge(true, false));
    assert_eq!(
        host.editor_state.editor_ui.image_panel.search_query.caret(),
        "a你bc".len()
    );
    assert!(host.apply_image_panel_edge(false, true));
    assert_eq!(
        host.editor_state
            .editor_ui
            .image_panel
            .search_query
            .highlight_range(),
        Some((0, "a你bc".len()))
    );
}
