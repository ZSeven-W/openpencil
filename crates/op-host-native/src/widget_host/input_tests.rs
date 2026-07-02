//! `#[cfg(test)]` companion to the input modules — extracted here so
//! the input module stays under the 800-line ceiling.
//!
//! `EditorState` is the host's source of truth, so the fixtures seed
//! `host.editor_state` from canonical-schema JSON and assert against
//! `editor_state` + the derived `LayoutScene` render scene.

use super::{CursorHint, WidgetHostNative};
use op_editor_core::ui_draft::PropertyFocus;
use op_editor_core::PenNodeExt;
use op_editor_core::{own_bounds, NodeId};
use op_editor_ui::widgets::{PropertyPanel, TopBar, TopBarHit, TOP_BAR_HEIGHT};

/// Seed a host's `editor_state` from a canonical `.op` JSON snippet.
fn seed(host: &mut WidgetHostNative, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.mark_paint_dirty_for_test();
}

fn point_inside_property_panel_without_target(host: &WidgetHostNative) -> op_editor_ui::Point2D {
    let panel = PropertyPanel::for_selection(host.editor_state()).expect("property panel");
    let rect = host.property_rect(host.last_viewport_w, host.last_viewport_h);
    let mut y = rect.origin.y + rect.size.y - 12.0;
    while y > rect.origin.y {
        let mut x = rect.origin.x + 12.0;
        while x < rect.origin.x + rect.size.x - 12.0 {
            let point = op_editor_ui::Point2D::new(x, y);
            let no_action = panel.hit_test_action(rect, point).is_none();
            let no_input = panel.hit_test(rect, point).is_none();
            let no_tab = panel.tab_hover_at(rect, point).is_none();
            let no_fill_type = panel.fill_type_picker_row_at(rect, point).is_none();
            if no_action && no_input && no_tab && no_fill_type {
                return point;
            }
            x += 8.0;
        }
        y -= 8.0;
    }
    panic!("no empty property-panel point found");
}

#[test]
fn layer_hover_does_not_refresh_stale_canvas_layout_scene() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    host.editor_state_mut().editor_ui.hovered_layer_id = Some(NodeId::new("n1"));
    host.mark_paint_dirty_for_test();

    let panel = op_editor_ui::widgets::LayerPanel::from_editor(host.editor_state());
    let rect = op_editor_ui::Rect {
        origin: op_editor_ui::Point2D::new(0.0, op_editor_ui::widgets::TOP_BAR_HEIGHT),
        size: op_editor_ui::Point2D::new(
            host.editor_state().editor_ui.layer_panel_width,
            800.0 - op_editor_ui::widgets::TOP_BAR_HEIGHT,
        ),
    };
    let regions = panel.regions(rect);
    let x = 48.0;
    let y = regions.layers_rows_top + 8.0;

    assert!(!host.update_layer_hover(x, y, 1200.0, 800.0));
    assert!(
        host.editor_state_dirty,
        "layer hover should not rebuild or clear the stale canvas layout scene"
    );
}

#[test]
fn file_menu_cursor_move_clears_stale_layer_hover() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    host.last_viewport_w = 1200.0;
    host.last_viewport_h = 800.0;
    host.editor_state_mut().editor_ui.file_menu_open = true;
    host.editor_state_mut().editor_ui.hovered_layer_id = Some(NodeId::new("n1"));
    let menu_rect = host
        .file_menu_rect(host.last_viewport_w)
        .expect("file menu rect");
    let menu = op_editor_ui::widgets::file_menu::FileMenu::from_editor_ui(
        &host.editor_state().editor_ui,
        0,
    );
    let x = menu_rect.origin.x + 80.0;
    let mut y = menu_rect.origin.y + 2.0;
    let mut point = None;
    while y < menu_rect.origin.y + menu_rect.size.y {
        let p = op_editor_ui::Point2D::new(x, y);
        if menu.hovered_at(menu_rect, p).is_some() {
            point = Some(p);
            break;
        }
        y += 2.0;
    }
    let point = point.expect("file menu row point");
    assert!(host.over_dropdown_overlay(
        point.x,
        point.y,
        host.last_viewport_w,
        host.last_viewport_h
    ));

    assert!(host.apply_cursor_move(point.x, point.y));

    assert_eq!(host.editor_state().editor_ui.hovered_layer_id, None);
    assert_eq!(host.editor_state().editor_ui.hovered_page_index, None);
    assert!(
        host.editor_state().editor_ui.file_menu.hover.is_some(),
        "file-menu hover itself should still be active"
    );
}

#[test]
fn property_panel_blank_hover_consumes_and_clears_lower_hover() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"group","id":"text_group","name":"Text Group",
               "children":[
                 {"type":"text","id":"label","name":"Label","content":"Hello"}
               ]}
        ]}"##,
    );
    host.last_viewport_w = 1200.0;
    host.last_viewport_h = 800.0;
    host.editor_state_mut()
        .set_single_selection(NodeId::new("text_group"));
    host.editor_state_mut().editor_ui.canvas_hover_node = Some(NodeId::new("label"));

    let point = point_inside_property_panel_without_target(&host);

    assert!(
        host.apply_cursor_move(point.x, point.y),
        "right inspector should own cursor movement inside its bounds"
    );
    assert_eq!(host.editor_state().editor_ui.canvas_hover_node, None);
}

#[test]
fn opening_file_menu_clears_stale_layer_hover() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    host.last_viewport_w = 1200.0;
    host.last_viewport_h = 800.0;
    host.editor_state_mut().editor_ui.hovered_layer_id = Some(NodeId::new("n1"));
    host.editor_state_mut().editor_ui.hovered_page_index = Some(0);

    let top_bar_rect = op_editor_ui::Rect {
        origin: op_editor_ui::Point2D::new(0.0, 0.0),
        size: op_editor_ui::Point2D::new(host.last_viewport_w, TOP_BAR_HEIGHT),
    };
    let top_bar = TopBar::for_editor_ui(&host.editor_state().editor_ui);
    let mut file_button = None;
    let mut x = top_bar_rect.origin.x;
    while x < top_bar_rect.origin.x + top_bar_rect.size.x {
        let p = op_editor_ui::Point2D::new(x, TOP_BAR_HEIGHT / 2.0);
        if top_bar.hit_test(top_bar_rect, p) == Some(TopBarHit::ToggleFileMenu) {
            file_button = Some(p);
            break;
        }
        x += 1.0;
    }
    let point = file_button.expect("top-bar file button point");

    assert!(host.apply_press(point.x, point.y, host.last_viewport_w, host.last_viewport_h));

    assert!(host.editor_state().editor_ui.file_menu_open);
    assert_eq!(host.editor_state().editor_ui.hovered_layer_id, None);
    assert_eq!(host.editor_state().editor_ui.hovered_page_index, None);
}

#[test]
fn layer_row_selection_does_not_dirty_canvas_layout_scene() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":0,"y":0,"width":100,"height":50},{"type":"rectangle","id":"n2","name":"n2","x":120,"y":0,"width":100,"height":50}]}"#,
    );
    let _ = host.layout_scene();
    assert!(!host.editor_state_dirty);

    let panel = op_editor_ui::widgets::LayerPanel::from_editor(host.editor_state());
    let rect = op_editor_ui::Rect {
        origin: op_editor_ui::Point2D::new(0.0, op_editor_ui::widgets::TOP_BAR_HEIGHT),
        size: op_editor_ui::Point2D::new(
            host.editor_state().editor_ui.layer_panel_width,
            800.0 - op_editor_ui::widgets::TOP_BAR_HEIGHT,
        ),
    };
    let regions = panel.regions(rect);
    let x = 48.0;
    let y = regions.layers_rows_top + 8.0;

    assert!(host.apply_click(x, y, 1200.0, 800.0));
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n1"));
    assert!(
        !host.editor_state_dirty,
        "selecting a layer row should not invalidate canvas layout"
    );
}

#[test]
fn layer_right_press_does_not_dirty_canvas_layout_scene() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":0,"y":0,"width":100,"height":50},{"type":"rectangle","id":"n2","name":"n2","x":120,"y":0,"width":100,"height":50}]}"#,
    );
    let _ = host.layout_scene();
    assert!(!host.editor_state_dirty);

    let panel = op_editor_ui::widgets::LayerPanel::from_editor(host.editor_state());
    let rect = op_editor_ui::Rect {
        origin: op_editor_ui::Point2D::new(0.0, op_editor_ui::widgets::TOP_BAR_HEIGHT),
        size: op_editor_ui::Point2D::new(
            host.editor_state().editor_ui.layer_panel_width,
            800.0 - op_editor_ui::widgets::TOP_BAR_HEIGHT,
        ),
    };
    let regions = panel.regions(rect);
    let x = 48.0;
    let y = regions.layers_rows_top + 8.0;

    assert!(host.apply_right_press(x, y, 1200.0, 800.0));
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n1"));
    assert!(host.editor_state().editor_ui.layer_context_menu.is_some());
    assert!(
        !host.editor_state_dirty,
        "right-clicking a layer row should not invalidate canvas layout"
    );
}

#[test]
fn layer_right_press_does_not_refresh_stale_canvas_layout_scene() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    host.mark_paint_dirty_for_test();

    let panel = op_editor_ui::widgets::LayerPanel::from_editor(host.editor_state());
    let rect = op_editor_ui::Rect {
        origin: op_editor_ui::Point2D::new(0.0, op_editor_ui::widgets::TOP_BAR_HEIGHT),
        size: op_editor_ui::Point2D::new(
            host.editor_state().editor_ui.layer_panel_width,
            800.0 - op_editor_ui::widgets::TOP_BAR_HEIGHT,
        ),
    };
    let regions = panel.regions(rect);
    let x = 48.0;
    let y = regions.layers_rows_top + 8.0;

    assert!(host.apply_right_press(x, y, 1200.0, 800.0));
    assert!(
        host.editor_state_dirty,
        "right-clicking a layer row should not rebuild or clear a stale canvas layout scene"
    );
}

#[test]
fn canvas_node_selection_does_not_dirty_canvas_layout_scene() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (360.0, 180.0, 80.0, 40.0),
                (480.0, 180.0, 80.0, 40.0),
                (600.0, 180.0, 80.0, 40.0),
            ],
            ["n10", "n11", "n12"],
        ),
    );
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    let _ = host.layout_scene();
    assert!(!host.editor_state_dirty);

    assert!(host.apply_press(
        cx0 + 360.0 + 20.0,
        cy0 + 180.0 + 20.0,
        viewport_w,
        viewport_h
    ));

    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n10"));
    assert!(
        !host.editor_state_dirty,
        "selecting a canvas node should not invalidate canvas layout"
    );
}

#[test]
fn node_drag_smart_guides_do_not_refresh_layout_scene_on_each_move() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (360.0, 180.0, 80.0, 40.0),
                (480.0, 180.0, 80.0, 40.0),
                (600.0, 180.0, 80.0, 40.0),
            ],
            ["n10", "n11", "n12"],
        ),
    );
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    let _ = host.layout_scene();
    assert!(!host.editor_state_dirty);
    let scene_x_before = host
        .layout_scene
        .active_page()
        .and_then(|p| p.find("n10"))
        .expect("scene node")
        .bounds
        .origin
        .x;

    assert!(host.apply_press(
        cx0 + 360.0 + 20.0,
        cy0 + 180.0 + 20.0,
        viewport_w,
        viewport_h
    ));
    assert!(host.node_drag.is_some(), "press should start a node drag");
    assert!(host.apply_cursor_move(cx0 + 360.0 + 30.0, cy0 + 180.0 + 20.0));

    let moved = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("n10"),
    )
    .expect("moved node");
    assert_eq!(own_bounds(moved).x, 370.0);
    let scene_x_after = host
        .layout_scene
        .active_page()
        .and_then(|p| p.find("n10"))
        .expect("patched scene node")
        .bounds
        .origin
        .x;
    assert_eq!(
        scene_x_after,
        scene_x_before + 10.0,
        "node drag should patch the current layout scene without a rebuild"
    );
    assert!(
        !host.editor_state_dirty,
        "node drag should not force a full layout-scene rebuild on every move"
    );
    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));
    assert!(
        host.editor_state_dirty,
        "node drag release should schedule one full layout-scene reconciliation"
    );
}

/// Three top-level rect nodes at the given `(x, y, w, h)` boxes.
fn three_rects(boxes: [(f64, f64, f64, f64); 3], ids: [&str; 3]) -> String {
    let node = |id: &str, b: (f64, f64, f64, f64)| {
        format!(
            r#"{{"type":"rectangle","id":"{id}","name":"{id}",
               "x":{},"y":{},"width":{},"height":{}}}"#,
            b.0, b.1, b.2, b.3
        )
    };
    format!(
        r#"{{"version":"0.8.0","children":[{},{},{}]}}"#,
        node(ids[0], boxes[0]),
        node(ids[1], boxes[1]),
        node(ids[2], boxes[2]),
    )
}

#[test]
fn ai_chat_maximize_click_expands_panel_geometry() {
    let mut host = WidgetHostNative::new();
    let before = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    let x = before.origin.x + before.size.x - 16.0 - 50.0 + 9.0;
    let y = before.origin.y + 17.0;

    assert!(host.apply_click(x, y, 1200.0, 800.0));

    let after = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("maximized chat panel visible");
    assert!(host.editor_state().chat.maximized);
    assert!(after.size.x > before.size.x);
    assert!(after.size.y > before.size.y);
}

#[test]
fn ai_chat_new_chat_click_clears_transcript_and_queues_abort() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant("old"));
    host.editor_state_mut().chat.set_input_text("draft");
    let rect = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    let x = rect.origin.x + rect.size.x - 16.0 - 22.0 + 9.0;
    let y = rect.origin.y + 17.0;

    assert!(host.apply_click(x, y, 1200.0, 800.0));

    assert!(host.editor_state().chat.messages.is_empty());
    assert!(host.editor_state().chat.input.text().is_empty());
    assert!(host.editor_state().chat.pending_new_chat);
}

#[test]
fn dragging_user_transcript_text_selects_and_copy_queues_text() {
    let prompt = "生成一个设计精良的美食应用移动端首页";
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::user(prompt));
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    let rect = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("chat panel visible");
    // At the 360px default panel width this 18-char CJK prompt wraps to two
    // bubble lines (line 0 = first 17 chars, line 1 = the final "页"), so a
    // whole-prompt drag has to run from line 0 down into line 1. A horizontal
    // drag on line 0 alone can only reach the end of line 0, dropping "页".
    let start_x = rect.origin.x + 96.0;
    let start_y = rect.origin.y + 74.0; // line 0
    let end_x = rect.origin.x + 338.0;
    let end_y = rect.origin.y + 88.0; // line 1 (the wrapped final glyph)

    assert!(host.apply_press(start_x, start_y, viewport_w, viewport_h));
    assert!(host.apply_cursor_move(end_x, end_y));
    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));

    assert_eq!(
        host.editor_state().chat.selected_transcript_text(),
        Some(prompt)
    );
    assert!(host.apply_copy());
    assert_eq!(
        host.editor_state().chat.pending_copy_text.as_deref(),
        Some(prompt)
    );
}

#[test]
fn user_transcript_text_uses_text_cursor() {
    let prompt = "生成一个设计精良的美食应用移动端首页";
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::user(prompt));
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    let rect = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("chat panel visible");
    let x = rect.origin.x + 96.0;
    let y = rect.origin.y + 74.0;

    assert_eq!(
        host.cursor_hint(x, y, viewport_w, viewport_h),
        CursorHint::Text
    );
}

#[test]
fn chat_input_text_uses_text_cursor() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().chat.set_input_text("abcdef");
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    let rect = host
        .ai_chat_rect(viewport_w, viewport_h)
        .expect("chat panel visible");
    let x = rect.origin.x + 32.0;
    let y = rect.origin.y + textarea_center_y_for_test();

    assert_eq!(
        host.cursor_hint(x, y, viewport_w, viewport_h),
        CursorHint::Text
    );
}

#[test]
fn ai_chat_east_edge_shows_resize_cursor() {
    let host = WidgetHostNative::new();
    let rect = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    let x = rect.origin.x + rect.size.x - 2.0;
    let y = rect.origin.y + rect.size.y / 2.0;

    assert_eq!(host.cursor_hint(x, y, 1200.0, 800.0), CursorHint::ResizeEw);
}

#[test]
fn ai_chat_east_edge_drag_resizes_panel_width() {
    let mut host = WidgetHostNative::new();
    let before = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    let x = before.origin.x + before.size.x - 2.0;
    let y = before.origin.y + before.size.y / 2.0;

    assert!(host.apply_press(x, y, 1200.0, 800.0));
    assert!(host.apply_cursor_move(x + 72.0, y));

    let after = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible after resize");
    assert!(
        after.size.x > before.size.x + 60.0,
        "dragging the east edge should grow chat width; before={before:?}, after={after:?}"
    );
    assert_eq!(after.origin.x, before.origin.x);
}

#[test]
fn ai_chat_stop_click_keeps_transcript_and_queues_abort() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::user("make a dashboard"));
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant_streaming());
    let rect = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    let x = rect.origin.x + rect.size.x - 16.0 - 20.0;
    let y = rect.origin.y + toolbar_center_y_for_test();

    assert!(host.apply_click(x, y, 1200.0, 800.0));

    assert_eq!(host.editor_state().chat.messages.len(), 2);
    assert!(!host.editor_state().chat.messages[1].streaming);
    assert!(host.editor_state().chat.pending_stop_chat);
}

#[test]
fn ai_chat_agent_team_click_sets_team_size_via_parallel_agents_picker() {
    // #32: the ⚡ footer chip no longer cycles the team size on click — it
    // opens the Parallel Agents picker, and clicking a row (1x–6x) sets
    // `agent_team_size`. Drive that two-step flow through the real hit-test
    // routing (probe with `hit_test`, then click the resolved points).
    use op_editor_ui::widgets::AIChatHit;
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .available_models
        .push(op_editor_core::chat::ModelEntry::new(
            op_editor_core::chat::AgentProvider::CodexCli,
            "gpt-5",
            "GPT-5",
        ));
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    let rect = host.ai_chat_rect(viewport_w, viewport_h).unwrap();

    // Locate the ⚡ speed chip along the footer toolbar row and click it to
    // open the picker.
    let panel = op_editor_ui::widgets::AIChatPlaceholder::from_editor(host.editor_state());
    let toolbar_y = rect.origin.y + toolbar_center_y_for_test();
    let mut speed_point = None;
    let mut sx = rect.origin.x;
    while sx < rect.origin.x + rect.size.x {
        let p = op_editor_ui::Point2D::new(sx, toolbar_y);
        if panel.hit_test(rect, p) == Some(AIChatHit::ToggleParallelAgentsPicker) {
            speed_point = Some(p);
            break;
        }
        sx += 1.0;
    }
    let speed_point = speed_point.expect("footer speed chip should be hittable");
    assert!(host.apply_click(speed_point.x, speed_point.y, viewport_w, viewport_h));
    assert!(host.editor_state().editor_ui.parallel_agents_picker_open);

    // With the picker open, find and click the "2x" row.
    let panel = op_editor_ui::widgets::AIChatPlaceholder::from_editor(host.editor_state());
    let mut row_point = None;
    let mut ry = rect.origin.y;
    'scan: while ry < rect.origin.y + rect.size.y {
        let mut rx = rect.origin.x;
        while rx < rect.origin.x + rect.size.x {
            let p = op_editor_ui::Point2D::new(rx, ry);
            if panel.hit_test(rect, p) == Some(AIChatHit::SetParallelAgents(2)) {
                row_point = Some(p);
                break 'scan;
            }
            rx += 2.0;
        }
        ry += 2.0;
    }
    let row_point = row_point.expect("parallel agents picker row 2 should be hittable");
    assert!(host.apply_click(row_point.x, row_point.y, viewport_w, viewport_h));
    assert_eq!(host.editor_state().chat.agent_team_size, 2);
    assert!(!host.editor_state().editor_ui.parallel_agents_picker_open);
}

#[test]
fn ai_chat_streaming_textarea_click_does_not_focus_disabled_input() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant_streaming());
    let rect = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    let x = rect.origin.x + 120.0;
    let y = rect.origin.y + textarea_center_y_for_test();

    assert!(host.apply_click(x, y, 1200.0, 800.0));

    assert!(!host.editor_state().chat.focused);
}

#[test]
fn ai_chat_streaming_attachment_click_does_not_open_picker() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant_streaming());
    let rect = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    let x = rect.origin.x + op_editor_ui::widgets::AI_CHAT_WIDTH - 16.0 - 52.0;
    let y = rect.origin.y + toolbar_center_y_for_test();

    assert!(host.apply_click(x, y, 1200.0, 800.0));

    assert!(!host.editor_state().chat.pending_attachment_pick);
}

fn toolbar_action_point_for_test(
    host: &WidgetHostNative,
    action: op_editor_ui::widgets::ToolbarAction,
    viewport_w: f32,
    viewport_h: f32,
) -> (f32, f32) {
    let toolbar = op_editor_ui::widgets::Toolbar::for_editor(host.editor_state());
    let rect = host.toolbar_rect(viewport_w, viewport_h);
    let min_x = rect.origin.x.floor() as i32;
    let max_x = (rect.origin.x + rect.size.x).ceil() as i32;
    let min_y = rect.origin.y.floor() as i32;
    let max_y = (rect.origin.y + rect.size.y).ceil() as i32;
    let mut hits = Vec::new();
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if toolbar.hit_test(rect, op_editor_ui::Point2D::new(x as f32, y as f32))
                == Some(op_editor_ui::widgets::ToolbarHit::Action(action))
            {
                hits.push((x as f32, y as f32));
            }
        }
    }
    let (sum_x, sum_y) = hits
        .iter()
        .fold((0.0, 0.0), |(sx, sy), (x, y)| (sx + *x, sy + *y));
    let count = hits.len().max(1) as f32;
    assert!(
        !hits.is_empty(),
        "toolbar action {action:?} should expose a hit target"
    );
    (sum_x / count, sum_y / count)
}

#[test]
fn toolbar_panel_actions_open_variables_and_design_panels() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    host.editor_state_mut().editor_ui.property_tab = op_editor_core::PropertyTab::Design;
    host.editor_state_mut().chat.collapsed = true;
    let viewport_w = 1200.0;
    let viewport_h = 800.0;

    let (variables_x, variables_y) = toolbar_action_point_for_test(
        &host,
        op_editor_ui::widgets::ToolbarAction::ToggleVariablesPanel,
        viewport_w,
        viewport_h,
    );
    let toolbar = op_editor_ui::widgets::Toolbar::for_editor(host.editor_state());
    assert_eq!(
        toolbar.hit_test(
            host.toolbar_rect(viewport_w, viewport_h),
            op_editor_ui::Point2D::new(variables_x, variables_y)
        ),
        Some(op_editor_ui::widgets::ToolbarHit::Action(
            op_editor_ui::widgets::ToolbarAction::ToggleVariablesPanel
        ))
    );
    // The VariablesPanel is a floating canvas overlay (TS
    // `variables-panel.tsx` absolute-positioned panel), NOT a
    // right-rail tab — toggling it must not affect the rail.
    // Mirrors `right_rail_visible_true_on_selection_only` in
    // op-editor-core::tests_mutators.
    assert!(!host.editor_state().right_rail_visible());
    assert!(host.apply_press(variables_x, variables_y, viewport_w, viewport_h));
    assert!(host.editor_state().editor_ui.variables_panel_open);
    assert!(!host.editor_state().right_rail_visible());
    assert_eq!(
        host.editor_state().editor_ui.property_tab,
        op_editor_core::PropertyTab::Design
    );
    assert!(host.apply_press(variables_x, variables_y, viewport_w, viewport_h));
    assert!(!host.editor_state().editor_ui.variables_panel_open);
    assert!(!host.editor_state().right_rail_visible());

    let (design_x, design_y) = toolbar_action_point_for_test(
        &host,
        op_editor_ui::widgets::ToolbarAction::ToggleDesignPanel,
        viewport_w,
        viewport_h,
    );
    assert!(host.apply_press(design_x, design_y, viewport_w, viewport_h));
    assert!(host.editor_state().editor_ui.design_md_panel_open);
}

#[test]
fn explicit_variables_toolbar_opens_floating_variables_panel() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{"version":"0.8.0","children":[{"type":"frame","id":"frame-1","name":"Frame","x":0,"y":0,"width":100,"height":50}]}"##,
    );
    host.editor_state_mut().selection.anchor = NodeId::new("frame-1");
    host.editor_state_mut().selection.set = vec![NodeId::new("frame-1")];
    host.editor_state_mut().create_variable(
        "spacing-md",
        jian_ops_schema::variable::VariableKind::Number,
        jian_ops_schema::variable::VariableScalar::Num(16.0),
    );
    host.editor_state_mut().editor_ui.variables_panel_open = true;

    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    let vars_rect = host.variables_panel_rect(viewport_w, viewport_h).unwrap();

    assert!(
        host.editor_state().right_rail_visible(),
        "selected frame should still own the right rail"
    );
    assert!(host.apply_press(
        vars_rect.origin.x + 16.0,
        vars_rect.origin.y + 44.0 + 36.0 + 18.0,
        viewport_w,
        viewport_h
    ));
    assert!(host.editor_state().editor_ui.variables_panel_open);
    assert!(host.editor_state().editor_ui.variable_row_focus.is_some());
}

fn toolbar_center_y_for_test() -> f32 {
    op_editor_ui::widgets::AI_CHAT_HEIGHT - 19.0
}

fn textarea_center_y_for_test() -> f32 {
    const INPUT_AREA_HEIGHT: f32 = 56.0;
    const INPUT_TOOLBAR_HEIGHT: f32 = 40.0;
    op_editor_ui::widgets::AI_CHAT_HEIGHT - (INPUT_AREA_HEIGHT + INPUT_TOOLBAR_HEIGHT)
        + 1.0
        + INPUT_AREA_HEIGHT / 2.0
}

#[test]
fn chat_input_click_clears_select_all_without_erasing_text() {
    let mut host = WidgetHostNative::new();
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    let rect = host.ai_chat_rect(viewport_w, viewport_h).unwrap();
    host.editor_state_mut()
        .chat
        .set_input_text("设计一个现代的移动端登录页面");
    host.editor_state_mut().chat.focused = true;
    host.editor_state_mut().chat.select_all_input(0);

    assert!(host.apply_press(
        rect.origin.x + 80.0,
        rect.origin.y + textarea_center_y_for_test(),
        viewport_w,
        viewport_h
    ));

    assert_eq!(
        host.editor_state().chat.input.text(),
        "设计一个现代的移动端登录页面"
    );
    assert!(host.editor_state().chat.focused);
    assert!(host.editor_state().chat.input.highlight_range().is_none());
}

#[test]
fn chat_input_drag_selects_partial_text_and_replaces_it() {
    let mut host = WidgetHostNative::new();
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    let rect = host.ai_chat_rect(viewport_w, viewport_h).unwrap();
    host.editor_state_mut().chat.set_input_text("abcdef");
    host.editor_state_mut().chat.focused = true;
    let text_x = rect.origin.x + 24.0;
    let text_y = rect.origin.y + textarea_center_y_for_test();

    assert!(host.apply_press(text_x + 6.6, text_y, viewport_w, viewport_h));
    assert_eq!(
        host.editor_state().chat.input.selection(),
        jian_core::text_input::Selection {
            anchor: 1,
            focus: 1
        }
    );
    assert!(host.apply_cursor_move(text_x + 19.8, text_y));
    assert_eq!(
        host.editor_state().chat.input.selection(),
        jian_core::text_input::Selection {
            anchor: 1,
            focus: 3
        }
    );
    assert!(host.apply_release());
    assert!(host.apply_text('X'));

    assert_eq!(host.editor_state().chat.input.text(), "aXdef");
}

#[test]
fn escape_closes_one_overlay_per_press_in_priority_order() {
    // Codex CONCERN-2 regression: Escape used to clear all
    // three pickers in a single press. TS parity is one-at-a-
    // time, in the order property-focus → locale → shape →
    // fill-type → chat → selection.
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::PositionX);
    host.editor_state_mut().ui.property_input.set_text("12");
    host.editor_state_mut().editor_ui.locale_picker.open = true;
    host.editor_state_mut().editor_ui.shape_picker.open = true;
    host.editor_state_mut().editor_ui.fill_type_picker.open = true;
    host.editor_state_mut().chat.focused = true;
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n10"));

    // 1. Property focus clears first.
    assert!(host.apply_escape());
    assert!(host.editor_state().ui.property_focus.is_none());
    assert!(host.editor_state().ui.property_input.text().is_empty());
    assert!(host.editor_state().editor_ui.locale_picker.open);

    // 2. Locale picker next.
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.locale_picker.open);
    assert!(host.editor_state().editor_ui.shape_picker.open);

    // 3. Shape picker.
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.shape_picker.open);
    assert!(host.editor_state().editor_ui.fill_type_picker.open);

    // 4. Fill-type picker.
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.fill_type_picker.open);
    assert!(host.editor_state().chat.focused);

    // 5. Chat focus.
    assert!(host.apply_escape());
    assert!(!host.editor_state().chat.focused);
    assert!(!host.editor_state().selection.is_empty());

    // 6. Selection.
    assert!(host.apply_escape());
    assert!(host.editor_state().selection.is_empty());

    // 7. Nothing left — returns false.
    assert!(!host.apply_escape());
}

#[test]
fn rename_caret_arrows_move_caret_then_fall_through() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (0.0, 0.0, 10.0, 10.0),
                (20.0, 0.0, 10.0, 10.0),
                (40.0, 0.0, 10.0, 10.0),
            ],
            ["ab", "b", "c"],
        ),
    );
    assert!(host
        .editor_state_mut()
        .start_rename_layer(NodeId::new("ab")));
    // Draft "ab" seeds caret at the end (2).
    assert_eq!(
        host.editor_state()
            .ui
            .layer_rename
            .as_ref()
            .unwrap()
            .input
            .caret(),
        2
    );
    // Left arrow during rename is consumed and moves the caret.
    assert!(host.apply_rename_caret(false));
    assert_eq!(
        host.editor_state()
            .ui
            .layer_rename
            .as_ref()
            .unwrap()
            .input
            .caret(),
        1
    );
    assert!(host.apply_rename_caret(true));
    assert_eq!(
        host.editor_state()
            .ui
            .layer_rename
            .as_ref()
            .unwrap()
            .input
            .caret(),
        2
    );
    // With no rename active the arrow falls through (not consumed).
    host.editor_state_mut().rename_cancel();
    assert!(!host.apply_rename_caret(false));
}

#[test]
fn status_bar_search_click_frames_content_in_viewport() {
    // Three rects spread across doc space (union ≈ x[100,400] y[100,300]).
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (100.0, 100.0, 100.0, 100.0),
                (300.0, 200.0, 100.0, 100.0),
                (150.0, 150.0, 50.0, 50.0),
            ],
            ["a", "b", "c"],
        ),
    );
    // Pan + zoom far away so the design is off-screen.
    host.editor_state_mut().viewport.pan_x = -5000.0;
    host.editor_state_mut().viewport.pan_y = -5000.0;
    host.editor_state_mut().viewport.zoom = 0.2;

    let (vw, vh) = (1200.0, 800.0);
    let r = host
        .status_bar_rect(vw, vh)
        .expect("status bar visible at this size");
    // Click the search icon (left section of the pill).
    let consumed = host.apply_press(r.origin.x + 5.0, r.origin.y + r.size.y / 2.0, vw, vh);

    assert!(consumed, "search-icon click must be consumed");
    let v = host.editor_state().viewport;
    assert!((v.zoom - 1.0).abs() < 1e-3, "zoom {}", v.zoom);
    assert!((v.pan_x - 230.0).abs() < 1e-2, "pan_x {}", v.pan_x);
    assert!((v.pan_y - 180.0).abs() < 1e-2, "pan_y {}", v.pan_y);
}

#[test]
fn status_bar_press_sets_and_release_clears_pressed_button() {
    let mut host = WidgetHostNative::new();
    let (vw, vh) = (1200.0, 800.0);
    let r = host
        .status_bar_rect(vw, vh)
        .expect("status bar visible at this size");
    let x = r.origin.x + 5.0;
    let y = r.origin.y + r.size.y / 2.0;

    assert!(host.apply_press(x, y, vw, vh));
    assert_eq!(
        host.editor_state().editor_ui.pressed_button,
        Some(op_editor_core::ButtonPressTarget::StatusBar(
            op_editor_core::StatusBarButton::Search
        ))
    );

    assert!(host.apply_release_with_viewport(vw, vh));
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
}

#[test]
fn export_dialog_press_sets_and_release_clears_pressed_button() {
    let mut host = WidgetHostNative::new();
    let (vw, vh) = (1200.0, 800.0);
    host.editor_state_mut().editor_ui.export_dialog_open = true;
    let dlg = op_editor_ui::widgets::ExportDialog::centered(vw, vh);
    let mut point = None;
    let r = dlg.rect();
    let mut y = r.origin.y;
    while y <= r.origin.y + r.size.y && point.is_none() {
        let mut x = r.origin.x;
        while x <= r.origin.x + r.size.x {
            let p = op_editor_ui::Point2D::new(x, y);
            if dlg.hit_test(p)
                == Some(op_editor_ui::widgets::export_dialog::ExportDialogHit::Scale(1))
            {
                point = Some(p);
                break;
            }
            x += 4.0;
        }
        y += 4.0;
    }
    let point = point.expect("scale 1 pill is hittable");

    assert!(host.apply_press(point.x, point.y, vw, vh));
    assert_eq!(
        host.editor_state().editor_ui.pressed_button,
        Some(op_editor_core::ButtonPressTarget::ExportDialog(
            op_editor_core::ExportDialogButton::Scale(1)
        ))
    );

    assert!(host.apply_release_with_viewport(vw, vh));
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
}

#[test]
fn figma_import_press_sets_and_release_clears_pressed_button() {
    let mut host = WidgetHostNative::new();
    let (vw, vh) = (1200.0, 800.0);
    host.editor_state_mut().editor_ui.figma_import_open = true;
    let modal =
        op_editor_ui::widgets::figma_import::FigmaImportModal::for_editor(host.editor_state());
    let panel = modal.rect(vw, vh);
    let mut point = None;
    let mut y = panel.origin.y;
    while y <= panel.origin.y + panel.size.y && point.is_none() {
        let mut x = panel.origin.x;
        while x <= panel.origin.x + panel.size.x {
            let p = op_editor_ui::Point2D::new(x, y);
            if modal.hit_test(panel, p)
                == op_editor_ui::widgets::figma_import::FigmaImportHit::DropZone
            {
                point = Some(p);
                break;
            }
            x += 4.0;
        }
        y += 4.0;
    }
    let point = point.expect("drop zone is hittable");

    assert!(host.apply_press(point.x, point.y, vw, vh));
    assert_eq!(
        host.editor_state().editor_ui.pressed_button,
        Some(op_editor_core::ButtonPressTarget::FigmaImport(
            op_editor_core::FigmaImportButton::DropZone
        ))
    );

    assert!(host.apply_release_with_viewport(vw, vh));
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
}

#[test]
fn pick_fill_image_keeps_image_popover_open_for_mode_selection() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.image_fill_popover_open = true;

    host.apply_property_action(op_editor_ui::widgets::PropertyPanelAction::PickFillImage);

    assert_eq!(
        host.editor_state().editor_ui.pending_file_action,
        Some(op_editor_core::editor_ui_state::FileAction::PickFillImage),
    );
    assert!(
        host.editor_state().editor_ui.image_fill_popover_open,
        "the image popover must stay open so Fill/Fit/Crop/Tile remain selectable",
    );
}

#[test]
fn image_adjustment_drag_updates_live_after_press() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n60","name":"Photo fill",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"image","url":"","mode":"fill",
                 "exposure":0,"contrast":0,"saturation":0,
                 "temperature":0,"tint":0,"highlights":0,"shadows":0}]}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n60"));
    host.editor_state_mut().editor_ui.image_fill_popover_open = true;
    host.image_adjustment_drag = Some(op_editor_core::ImageAdjustmentField::Exposure);
    host.last_viewport_w = 900.0;
    host.last_viewport_h = 760.0;

    assert!(host.apply_cursor_move(0.0, 0.0));

    let node = host
        .editor_state()
        .selected_node()
        .expect("selected image-fill node");
    match op_editor_core::fills::node_fills(node)
        .unwrap()
        .first()
        .unwrap()
    {
        jian_ops_schema::style::PenFill::Image(body) => {
            assert_eq!(body.exposure, Some(-100.0));
        }
        other => panic!("expected image fill, got {other:?}"),
    }
}

#[test]
fn image_fill_actions_refresh_the_render_scene() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n61","name":"Photo fill",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"image","url":"data:image/png;base64,AA==","mode":"fill",
                 "exposure":0,"contrast":0,"saturation":0,
                 "temperature":0,"tint":0,"highlights":0,"shadows":0}]}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n61"));
    host.mark_paint_dirty_for_test();

    let initial_fit = host
        .layout_scene()
        .active_page()
        .unwrap()
        .find("n61")
        .unwrap()
        .image_fit;
    assert_eq!(initial_fit, op_editor_ui::layout_scene::SceneImageFit::Fill);

    host.apply_property_action(
        op_editor_ui::widgets::PropertyPanelAction::SetImageFillMode(
            op_editor_core::ImageFillMode::Fit,
        ),
    );
    host.apply_property_action(
        op_editor_ui::widgets::PropertyPanelAction::SetImageAdjustment {
            field: op_editor_core::ImageAdjustmentField::Exposure,
            value: 64.0,
        },
    );

    let rendered = host
        .layout_scene()
        .active_page()
        .unwrap()
        .find("n61")
        .unwrap();
    assert_eq!(
        rendered.image_fit,
        op_editor_ui::layout_scene::SceneImageFit::Fit
    );
    assert_eq!(rendered.image_adjustments.exposure, 64.0);
}

#[test]
fn corner_radius_property_focus_updates_selected_rectangle() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n62","name":"Rounded",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"solid","color":"#BDC7D9"}]}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n62"));
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::PositionR);
    host.editor_state_mut().ui.property_input.set_text("24");

    host.commit_property_focus_if_any();

    let node = host.editor_state().selected_node().unwrap();
    match node {
        jian_ops_schema::node::PenNode::Rectangle(rect) => {
            assert_eq!(
                rect.container.corner_radius,
                Some(jian_ops_schema::node::container::CornerRadius::Uniform(
                    24.0
                )),
            );
        }
        other => panic!("expected rectangle, got {other:?}"),
    }
    let rendered = host
        .layout_scene()
        .active_page()
        .unwrap()
        .find("n62")
        .unwrap();
    assert_eq!(rendered.corner_radius, 24.0);
}

#[test]
fn property_focus_commit_reads_text_input_state() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n62","name":"Wide",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"solid","color":"#BDC7D9"}]}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n62"));
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::SizeW);
    host.editor_state_mut().ui.property_input.set_text("321");

    host.commit_property_focus_if_any();

    let bounds = own_bounds(host.editor_state().selected_node().unwrap());
    assert_eq!(bounds.w, 321.0);
    assert!(host.editor_state().ui.property_input.text().is_empty());
}

#[test]
fn widget_leading_icon_focus_commits_onto_text_input() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"text_input","id":"email","name":"Email",
               "x":24,"y":32,"width":220,"height":40,"placeholder":"Email"}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("email"));
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::WidgetLeadingIcon);
    // Type the glyph name char-by-char to exercise the free-text gate
    // (letters must NOT be rejected as non-numeric).
    for c in "mail".chars() {
        assert!(host.apply_text(c), "letter '{c}' must be accepted");
    }
    host.commit_property_focus_if_any();

    match host.editor_state().selected_node().unwrap() {
        jian_ops_schema::node::PenNode::TextInput(t) => {
            assert_eq!(t.leading_icon.as_deref(), Some("mail"));
        }
        other => panic!("expected text_input, got {other:?}"),
    }
}

#[test]
fn widget_bind_key_focus_writes_state_binding() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"text_input","id":"email","name":"Email",
               "x":24,"y":32,"width":220,"height":40,"placeholder":"Email"}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("email"));
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::WidgetBindKey);
    host.editor_state_mut().ui.property_input.set_text("email");
    host.commit_property_focus_if_any();

    match host.editor_state().selected_node().unwrap() {
        jian_ops_schema::node::PenNode::TextInput(t) => {
            let bindings = t.bindings.as_ref().expect("bindings written");
            assert_eq!(
                bindings.get("bind:value").map(|e| e.0.as_str()),
                Some("$state.email"),
            );
        }
        other => panic!("expected text_input, got {other:?}"),
    }
}

#[test]
fn property_focus_commit_is_undoable() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n62","name":"Wide",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"solid","color":"#BDC7D9"}]}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n62"));
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::SizeW);
    host.editor_state_mut().ui.property_input.set_text("321");

    host.commit_property_focus_if_any();

    let bounds = own_bounds(host.editor_state().selected_node().unwrap());
    assert_eq!(bounds.w, 321.0);
    assert!(host.editor_state().history.can_undo());

    assert!(host.editor_state_mut().undo());
    let bounds = own_bounds(host.editor_state().selected_node().unwrap());
    assert_eq!(bounds.w, 180.0);
}

#[test]
fn property_press_seeds_text_input_state() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n62","name":"Wide",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"solid","color":"#BDC7D9"}]}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n62"));

    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    let property_rect = host.property_rect(viewport_w, viewport_h);
    let panel = op_editor_ui::widgets::PropertyPanel::for_selection(host.editor_state())
        .expect("selection should show the property panel");
    let mut point = None;
    'outer: for y in 0..property_rect.size.y as i32 {
        for x in 0..property_rect.size.x as i32 {
            let candidate = op_editor_ui::Point2D::new(
                property_rect.origin.x + x as f32 + 0.5,
                property_rect.origin.y + y as f32 + 0.5,
            );
            if panel.hit_test(property_rect, candidate) == Some(PropertyFocus::SizeW) {
                point = Some(candidate);
                break 'outer;
            }
        }
    }
    let point = point.expect("width input should be hit-testable");

    assert!(host.apply_press(point.x, point.y, viewport_w, viewport_h));

    assert_eq!(
        host.editor_state().ui.property_focus,
        Some(PropertyFocus::SizeW)
    );
    assert_eq!(host.editor_state().ui.property_input.text(), "180");
    assert_eq!(host.editor_state().ui.property_input.caret(), 3);
}

#[test]
fn effect_param_focus_seeds_text_input_state() {
    let mut host = WidgetHostNative::new();

    host.apply_property_action(
        op_editor_ui::widgets::PropertyPanelAction::FocusEffectParam {
            effect: 0,
            field: op_editor_core::EffectField::OffsetX,
            value: 12.5,
        },
    );

    assert_eq!(
        host.editor_state().editor_ui.effect_param_focus,
        Some(op_editor_core::editor_ui_state::EffectParamFocus {
            effect: 0,
            field: op_editor_core::EffectField::OffsetX,
        })
    );
    assert_eq!(host.editor_state().ui.property_input.text(), "12.5");
    assert_eq!(host.editor_state().ui.property_input.caret(), 4);
}

#[test]
fn effect_param_input_uses_text_input_state_for_editing() {
    let mut host = WidgetHostNative::new();
    {
        let editor = host.editor_state_mut();
        editor.editor_ui.effect_param_focus =
            Some(op_editor_core::editor_ui_state::EffectParamFocus {
                effect: 0,
                field: op_editor_core::EffectField::OffsetX,
            });
        editor.ui.property_input.set_text("1234");
    }

    assert!(host.apply_property_caret(false));
    assert!(host.apply_property_caret(false));
    assert_eq!(host.editor_state().ui.property_input.caret(), 2);

    assert!(host.apply_text('9'));
    assert_eq!(host.editor_state().ui.property_input.text(), "12934");
    assert_eq!(host.editor_state().ui.property_input.caret(), 3);
}

#[test]
fn property_delete_uses_text_input_state() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n10"));
    {
        let ui = &mut host.editor_state_mut().ui;
        ui.property_focus = Some(PropertyFocus::PositionX);
        ui.property_input.set_text("123");
    }
    assert!(host.apply_property_caret(false));

    assert!(host.apply_delete());

    assert_eq!(host.editor_state().ui.property_input.text(), "12");
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n10"));
}

#[test]
fn property_step_reads_and_updates_text_input_state() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n62","name":"Wide",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"solid","color":"#BDC7D9"}]}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n62"));
    {
        let ui = &mut host.editor_state_mut().ui;
        ui.property_focus = Some(PropertyFocus::SizeW);
        ui.property_input.set_text("180");
    }

    assert!(host.apply_property_step(5.0));

    let bounds = own_bounds(host.editor_state().selected_node().unwrap());
    assert_eq!(bounds.w, 185.0);
    assert_eq!(host.editor_state().ui.property_input.text(), "185");
    assert_eq!(host.editor_state().ui.property_input.caret(), 3);
}

#[test]
fn property_escape_clears_text_input_state() {
    let mut host = WidgetHostNative::new();
    {
        let ui = &mut host.editor_state_mut().ui;
        ui.property_focus = Some(PropertyFocus::PositionX);
        ui.property_input.set_text("123");
    }

    assert!(host.apply_escape());

    assert!(host.editor_state().ui.property_focus.is_none());
    assert!(host.editor_state().ui.property_input.text().is_empty());
}

#[test]
fn polygon_sides_property_focus_updates_selected_polygon() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"polygon","id":"poly","name":"Polygon",
               "x":40,"y":40,"width":120,"height":120,
               "polygonCount":3}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("poly"));
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::PolygonSides);
    host.editor_state_mut().ui.property_input.set_text("7");

    host.commit_property_focus_if_any();

    let node = host.editor_state().selected_node().unwrap();
    match node {
        jian_ops_schema::node::PenNode::Polygon(poly) => {
            assert_eq!(poly.polygon_count, 7);
        }
        other => panic!("expected polygon, got {other:?}"),
    }
    let rendered = host
        .layout_scene()
        .active_page()
        .unwrap()
        .find("poly")
        .unwrap();
    assert_eq!(rendered.polygon_sides, 7);
}

#[test]
fn ellipse_arc_property_focus_updates_selected_ellipse() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"ellipse","id":"ell","name":"Ellipse",
               "x":40,"y":40,"width":120,"height":100}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("ell"));

    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::EllipseStart);
    host.editor_state_mut().ui.property_input.set_text("45");
    host.commit_property_focus_if_any();

    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::EllipseSweep);
    host.editor_state_mut().ui.property_input.set_text("180");
    host.commit_property_focus_if_any();

    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::EllipseInnerRadius);
    host.editor_state_mut().ui.property_input.set_text("25");
    host.commit_property_focus_if_any();

    let node = host.editor_state().selected_node().unwrap();
    match node {
        jian_ops_schema::node::PenNode::Ellipse(ell) => {
            assert_eq!(ell.start_angle, Some(45.0));
            assert_eq!(ell.sweep_angle, Some(180.0));
            assert_eq!(ell.inner_radius, Some(0.25));
        }
        other => panic!("expected ellipse, got {other:?}"),
    }
    let rendered = host
        .layout_scene()
        .active_page()
        .unwrap()
        .find("ell")
        .unwrap();
    assert_eq!(rendered.arc_start_angle, Some(45.0));
    assert_eq!(rendered.arc_sweep_angle, Some(180.0));
    assert_eq!(rendered.arc_inner_radius, Some(0.25));
}

#[test]
fn backspace_with_property_input_does_not_delete_selected() {
    // With a non-empty property input, Backspace must pop a char
    // from the input, not delete the selected node.
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n10"));
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::PositionX);
    host.editor_state_mut().ui.property_input.set_text("123");
    // Caret at the input's end, as a real focus seeds it — Backspace
    // deletes the char *before* the caret.

    assert!(host.apply_backspace());
    assert_eq!(host.editor_state().ui.property_input.text(), "12");
    assert_eq!(host.editor_state().ui.property_input.caret(), 2);
    // Selection must be untouched.
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n10"));
}

#[test]
fn backspace_without_focus_deletes_selected() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n10"));
    host.editor_state_mut().ui.property_focus = None;
    host.editor_state_mut().chat.focused = false;

    assert!(host.apply_backspace());
    assert!(host.editor_state().selection.is_empty());
}

#[test]
fn delete_with_model_picker_open_does_not_delete_selected() {
    // The chat model-picker search owns the keyboard while open, so
    // Delete must be swallowed instead of dropping the canvas node
    // behind the dropdown.
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n10"));
    host.editor_state_mut().editor_ui.chat_model_picker.open = true;

    assert!(!host.apply_delete());
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n10"));
}

#[test]
fn backspace_with_model_picker_open_edits_search_not_selection() {
    // Backspace pops from the model-picker search query, never the
    // selected node.
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n10"));
    host.editor_state_mut().editor_ui.chat_model_picker.open = true;
    host.editor_state_mut()
        .editor_ui
        .chat_model_picker_input
        .set_text("gp");

    assert!(host.apply_backspace());
    assert_eq!(
        host.editor_state().editor_ui.chat_model_picker_input.text(),
        "g"
    );
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n10"));
}

#[test]
fn shortcuts_gated_while_model_picker_open() {
    // Nudge / duplicate / reorder all route through `input_active`,
    // which must report the model picker as owning the keyboard.
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n10"));
    host.editor_state_mut().editor_ui.chat_model_picker.open = true;

    assert!(!host.apply_nudge(1.0, 0.0));
    assert!(!host.apply_duplicate());
    assert!(!host.apply_reorder(op_editor_core::ReorderDirection::Up));
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n10"));
}

#[test]
fn marquee_drag_replaces_selection_with_intersecting_nodes() {
    let mut host = WidgetHostNative::new();
    // 3 rects: two close together near origin, one far away.
    seed(
        &mut host,
        &three_rects(
            [
                (50.0, 10.0, 20.0, 20.0),
                (90.0, 10.0, 20.0, 20.0),
                (200.0, 200.0, 20.0, 20.0),
            ],
            ["n50", "n51", "n52"],
        ),
    );
    host.editor_state_mut().clear_selection();
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    let press_x = cx0 + 5.0;
    let press_y = cy0 + 5.0;
    host.apply_press(press_x, press_y, viewport_w, viewport_h);
    assert!(
        host.marquee_drag.is_some(),
        "empty-canvas press should start a marquee"
    );
    host.apply_cursor_move(cx0 + 130.0, cy0 + 50.0);
    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));
    assert!(host.marquee_drag.is_none(), "marquee consumed on release");
    let mut hits: Vec<String> = host
        .editor_state()
        .selection
        .set
        .iter()
        .map(|i| i.as_str().to_string())
        .collect();
    hits.sort();
    assert_eq!(hits, vec!["n50", "n51"]);
}

#[test]
fn marquee_drag_with_shift_preserves_already_selected_hit() {
    // Codex CONCERN-Q2 regression: shift-marquee must be ADD-only.
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (50.0, 50.0, 20.0, 20.0),
                (300.0, 300.0, 20.0, 20.0),
                (900.0, 900.0, 20.0, 20.0),
            ],
            ["n70", "n71", "n72"],
        ),
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n70"));
    host.set_modifier_shift(true);
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    host.apply_press(cx0 + 5.0, cy0 + 5.0, viewport_w, viewport_h);
    host.apply_cursor_move(cx0 + 90.0, cy0 + 90.0);
    host.apply_release_with_viewport(viewport_w, viewport_h);
    // "n70" stays in the set (shift-marquee is ADD-only).
    assert!(host.editor_state().is_selected(&NodeId::new("n70")));
    assert_eq!(host.editor_state().selection.set.len(), 1);
}

#[test]
fn marquee_drag_below_screen_threshold_is_a_no_op() {
    // Codex CONCERN-Q5 regression: threshold is screen-px.
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (0.0, 0.0, 100.0, 100.0),
                (5000.0, 5000.0, 10.0, 10.0),
                (6000.0, 6000.0, 10.0, 10.0),
            ],
            ["n80", "n81", "n82"],
        ),
    );
    host.editor_state_mut().viewport.zoom = 0.1;
    host.editor_state_mut().clear_selection();
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    host.apply_press(cx0 + 100.0, cy0 + 50.0, viewport_w, viewport_h);
    assert!(host.marquee_drag.is_some());
    // Tiny drag: 1 screen-px — below the 2-px threshold.
    host.apply_cursor_move(cx0 + 101.0, cy0 + 50.0);
    host.apply_release_with_viewport(viewport_w, viewport_h);
    assert!(host.editor_state().selection.set.is_empty());
}

#[test]
fn marquee_drag_with_shift_extends_existing_selection() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (10.0, 10.0, 20.0, 20.0),
                (50.0, 10.0, 20.0, 20.0),
                (300.0, 300.0, 20.0, 20.0),
            ],
            ["n60", "n61", "n62"],
        ),
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n62"));
    host.set_modifier_shift(true);
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    host.apply_press(cx0 + 5.0, cy0 + 5.0, viewport_w, viewport_h);
    assert!(host.marquee_drag.is_some());
    host.apply_cursor_move(cx0 + 130.0, cy0 + 50.0);
    host.apply_release_with_viewport(viewport_w, viewport_h);
    let mut ids: Vec<String> = host
        .editor_state()
        .selection
        .set
        .iter()
        .map(|i| i.as_str().to_string())
        .collect();
    ids.sort();
    assert_eq!(ids, vec!["n60", "n61", "n62"]);
}

#[test]
fn layer_drag_to_reorder_commits_on_release_with_threshold_move() {
    use op_editor_ui::widgets::TOP_BAR_HEIGHT;
    let mut host = WidgetHostNative::new();
    // Three top-level nodes painted as flat layer rows.
    seed(
        &mut host,
        &three_rects(
            [
                (0.0, 0.0, 10.0, 10.0),
                (0.0, 0.0, 10.0, 10.0),
                (0.0, 0.0, 10.0, 10.0),
            ],
            ["n70", "n71", "n72"],
        ),
    );
    host.editor_state_mut().clear_selection();
    let row_h = 28.0; // LAYER_ROW_HEIGHT
    let page_row_h = 32.0; // PAGE_ROW_HEIGHT
    let section_header_h = 28.0;
    let section_gap = 8.0;
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let layers_top =
        TOP_BAR_HEIGHT + 8.0 + section_header_h + page_row_h + section_gap + section_header_h;
    let row_y = |i: usize| layers_top + (i as f32) * row_h + row_h / 2.0;
    let row_x = host.editor_state().editor_ui.layer_panel_width / 2.0;
    host.apply_press(row_x, row_y(0), viewport_w, viewport_h);
    assert!(host.layer_drag.is_some());
    assert!(!host.layer_drag.as_ref().unwrap().active);
    host.apply_cursor_move(row_x, row_y(2) + row_h / 2.0 - 4.0);
    assert!(host.layer_drag.as_ref().unwrap().active);
    host.apply_release_with_viewport(viewport_w, viewport_h);
    assert!(host.layer_drag.is_none(), "drag must be cleared on release");
    // A moved after C → final order [B, C, A].
    let order: Vec<String> = host
        .editor_state()
        .doc
        .children
        .iter()
        .map(|n| n.base().id.clone())
        .collect();
    assert_eq!(order, vec!["n71", "n72", "n70"]);
}

#[test]
fn layer_drag_below_activation_threshold_is_a_click_not_a_reorder() {
    use op_editor_ui::widgets::TOP_BAR_HEIGHT;
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (0.0, 0.0, 10.0, 10.0),
                (0.0, 0.0, 10.0, 10.0),
                (5000.0, 5000.0, 10.0, 10.0),
            ],
            ["n80", "n81", "n82"],
        ),
    );
    host.editor_state_mut().clear_selection();
    let row_y_first = TOP_BAR_HEIGHT + 8.0 + 28.0 + 32.0 + 8.0 + 28.0 + 14.0;
    let row_x = host.editor_state().editor_ui.layer_panel_width / 2.0;
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    host.apply_press(row_x, row_y_first, viewport_w, viewport_h);
    host.apply_cursor_move(row_x, row_y_first + 2.0);
    assert!(
        host.layer_drag.is_some() && !host.layer_drag.as_ref().unwrap().active,
        "sub-threshold move must not activate"
    );
    host.apply_release_with_viewport(viewport_w, viewport_h);
    let order: Vec<String> = host
        .editor_state()
        .doc
        .children
        .iter()
        .map(|n| n.base().id.clone())
        .collect();
    assert_eq!(order, vec!["n80", "n81", "n82"]);
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n80"));
}

#[test]
fn layer_context_create_component_click_promotes_frame() {
    use op_editor_core::editor_ui_state::LayerContextMenuState;
    use op_editor_core::ui_draft::LayerContextTarget;
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[
          {"type":"frame","id":"n10","name":"Card","x":0,"y":0,"width":100,"height":80,
           "children":[]}
        ]}"#,
    );
    host.editor_state_mut().editor_ui.layer_context_menu = Some(LayerContextMenuState {
        target: LayerContextTarget::Layer(NodeId::new("n10")),
        anchor_x: 100.0,
        anchor_y: 100.0,
        menu: Default::default(),
    });

    let create_row_y = 100.0 + 6.0 + 32.0 * 2.0 + 16.0;
    assert!(host.apply_press(120.0, create_row_y, 1440.0, 900.0));
    assert!(host
        .editor_state()
        .components
        .find_by_id(&NodeId::new("n10"))
        .is_some());
    match &host.editor_state().doc.children[0] {
        jian_ops_schema::node::PenNode::Frame(f) => assert_eq!(f.reusable, Some(true)),
        _ => panic!("expected frame"),
    }
}

#[test]
fn property_panel_create_component_click_promotes_selected_frame() {
    use op_editor_ui::widgets::TOP_BAR_HEIGHT;
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[
          {"type":"frame","id":"n20","name":"Hero","x":0,"y":0,"width":120,"height":90,
           "children":[]}
        ]}"#,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n20"));

    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let panel_left = viewport_w - host.editor_state().editor_ui.property_panel_width;
    let button_x = panel_left + 24.0;
    let button_y = TOP_BAR_HEIGHT + 36.0 + 30.0 + 8.0 + 18.0;
    assert!(host.apply_press(button_x, button_y, viewport_w, viewport_h));
    assert!(host
        .editor_state()
        .components
        .find_by_id(&NodeId::new("n20"))
        .is_some());
}

#[test]
fn layer_context_group_preserves_multi_selection_and_groups() {
    use op_editor_ui::widgets::TOP_BAR_HEIGHT;
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (0.0, 0.0, 20.0, 20.0),
                (40.0, 0.0, 20.0, 20.0),
                (120.0, 0.0, 20.0, 20.0),
            ],
            ["n30", "n31", "n32"],
        ),
    );
    host.editor_state_mut().selection.set = vec![NodeId::new("n30"), NodeId::new("n31")];
    host.editor_state_mut().selection.anchor = NodeId::new("n30");

    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let row_x = host.editor_state().editor_ui.layer_panel_width / 2.0;
    let first_row_y = TOP_BAR_HEIGHT + 8.0 + 28.0 + 32.0 + 8.0 + 28.0 + 14.0;
    assert!(host.apply_right_press(row_x, first_row_y, viewport_w, viewport_h));
    assert_eq!(
        host.editor_state().selection.set.len(),
        2,
        "right-clicking an already-selected layer must keep the multi-selection"
    );

    let group_row_y = first_row_y + 6.0 + 32.0 * 2.0 + 16.0;
    assert!(host.apply_press(row_x + 20.0, group_row_y, viewport_w, viewport_h));
    assert!(matches!(
        host.editor_state().doc.children.first(),
        Some(jian_ops_schema::node::PenNode::Group(_))
    ));
}

#[test]
fn component_browser_open_owns_keyboard_search() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.component_browser_open = true;

    assert!(host.input_active_pub());
    assert!(host.apply_text('b'));
    assert!(host.apply_text('a'));
    assert_eq!(host.editor_state().editor_ui.component_browser_search, "ba");
    assert!(host.apply_backspace());
    assert_eq!(host.editor_state().editor_ui.component_browser_search, "b");
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.component_browser_open);
}

#[test]
fn component_browser_header_buttons_queue_kit_io_requests() {
    // Press → dispatch → `component_browser_kit_request` seam: the
    // desktop runner drains the queued request into the rfd dialogs
    // (`op-host-desktop/src/kit_io.rs`).
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.component_browser_open = true;
    let (vw, vh) = (1440.0, 900.0);
    let rect = host
        .component_browser_panel_rect(vw, vh)
        .expect("open panel rect");
    let right = rect.origin.x + rect.size.x;
    let y = rect.origin.y + 20.0; // header-button row centre (HEADER_H / 2)
                                  // Header buttons right-aligned: ✕ centre at right-26, Upload
                                  // (import) at right-54, Download (export) at right-82 — from
                                  // PAD 14 + 24-px buttons + 4-px gaps.
    assert!(host.apply_press(right - 54.0, y, vw, vh));
    assert_eq!(
        host.editor_state().editor_ui.component_browser_kit_request,
        Some(op_editor_core::KitIoRequest::Import)
    );
    host.editor_state_mut()
        .editor_ui
        .component_browser_kit_request = None;
    assert!(host.apply_press(right - 82.0, y, vw, vh));
    assert_eq!(
        host.editor_state().editor_ui.component_browser_kit_request,
        Some(op_editor_core::KitIoRequest::Export)
    );
}

#[test]
fn component_browser_header_press_sets_and_release_clears_pressed_button() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.component_browser_open = true;
    let (vw, vh) = (1440.0, 900.0);
    let rect = host
        .component_browser_panel_rect(vw, vh)
        .expect("open panel rect");
    let x = rect.origin.x + rect.size.x - 54.0;
    let y = rect.origin.y + 20.0;

    assert!(host.apply_press(x, y, vw, vh));
    assert_eq!(
        host.editor_state().editor_ui.pressed_button,
        Some(op_editor_core::ButtonPressTarget::ComponentBrowser(
            op_editor_core::ComponentBrowserButton::ImportKit
        ))
    );

    assert!(host.apply_release_with_viewport(vw, vh));
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
}

#[test]
fn chat_model_picker_open_owns_keyboard_search() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.chat_model_picker.open = true;
    host.editor_state_mut().chat.focused = true;

    assert!(host.input_active_pub());
    assert!(host.apply_text('g'));
    assert!(host.apply_text('p'));
    assert_eq!(
        host.editor_state().editor_ui.chat_model_picker_input.text(),
        "gp"
    );
    assert!(host.editor_state().chat.input.text().is_empty());
    assert!(host.apply_backspace());
    assert_eq!(
        host.editor_state().editor_ui.chat_model_picker_input.text(),
        "g"
    );
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.chat_model_picker.open);
}

#[test]
fn select_all_in_chat_input_replaces_next_typed_text() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().chat.focused = true;
    host.editor_state_mut().chat.set_input_text("abcdef");

    assert!(host.apply_select_all());
    assert_eq!(host.editor_state().chat.input.text(), "abcdef");
    assert!(host.apply_text('X'));
    assert_eq!(host.editor_state().chat.input.text(), "X");
}

#[test]
fn select_all_in_settings_input_replaces_next_typed_text() {
    let mut host = WidgetHostNative::new();
    {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.agent_settings.focus = Some(
            op_editor_core::agent_settings::SettingsFocus::BuiltinAgentDraft(
                op_editor_core::agent_settings::BuiltinAgentField::BaseUrl,
            ),
        );
        ui.settings_input.set_text("https://example.invalid");
    }

    assert!(host.apply_select_all());
    assert!(host.apply_text('x'));
    assert_eq!(host.editor_state().editor_ui.settings_input.text(), "x");
    assert_eq!(host.editor_state().editor_ui.settings_input.caret(), 1);
}

#[test]
fn select_all_in_property_input_replaces_next_typed_text() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::PositionX);
    host.editor_state_mut().ui.property_input.set_text("123");

    assert!(host.apply_select_all());
    assert!(host.apply_text('4'));
    assert_eq!(host.editor_state().ui.property_input.text(), "4");
    assert_eq!(host.editor_state().ui.property_input.caret(), 1);
}

#[test]
fn property_input_uses_text_input_state_for_editing() {
    let mut host = WidgetHostNative::new();
    {
        let ui = &mut host.editor_state_mut().ui;
        ui.property_focus = Some(PropertyFocus::PositionX);
        ui.property_input.set_text("1234");
    }

    assert!(host.apply_property_caret(false));
    assert!(host.apply_property_caret(false));
    assert_eq!(host.editor_state().ui.property_input.caret(), 2);

    assert!(host.apply_text('9'));
    assert_eq!(host.editor_state().ui.property_input.text(), "12934");
    assert_eq!(host.editor_state().ui.property_input.caret(), 3);

    assert!(host.apply_backspace());
    assert_eq!(host.editor_state().ui.property_input.text(), "1234");
    assert_eq!(host.editor_state().ui.property_input.caret(), 2);

    assert!(host.apply_select_all());
    assert!(host.apply_text('5'));
    assert_eq!(host.editor_state().ui.property_input.text(), "5");
    assert_eq!(host.editor_state().ui.property_input.caret(), 1);
}

#[test]
fn select_all_in_chat_model_picker_replaces_next_typed_text() {
    let mut host = WidgetHostNative::new();
    {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.chat_model_picker.open = true;
        ui.chat_model_picker_input.set_text("gpt");
    }

    assert!(host.apply_select_all());
    assert!(host.apply_text('x'));
    assert_eq!(
        host.editor_state().editor_ui.chat_model_picker_input.text(),
        "x"
    );
    assert_eq!(
        host.editor_state()
            .editor_ui
            .chat_model_picker_input
            .caret(),
        1
    );
}

#[test]
fn shape_picker_icon_row_opens_icon_picker() {
    let mut host = WidgetHostNative::new();
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    host.editor_state_mut().editor_ui.shape_picker.open = true;

    let panel = host.shape_picker_rect(viewport_w, viewport_h);
    let picker = op_editor_ui::widgets::ShapePicker::for_editor_ui(&host.editor_state().editor_ui);
    let x = panel.origin.x + 24.0;
    let mut probe = panel.origin.y + 2.0;
    let mut icon_row_y = None;
    while probe < panel.origin.y + panel.size.y {
        if matches!(
            picker.hit_test(panel, op_editor_ui::Point2D::new(x, probe)),
            Some(op_editor_ui::widgets::ShapeChoice::OpenIconPicker)
        ) {
            icon_row_y = Some(probe);
            break;
        }
        probe += 2.0;
    }
    let icon_row_y = icon_row_y.expect("icon row present in the shape picker");
    assert!(host.apply_press(x, icon_row_y, viewport_w, viewport_h));

    assert!(!host.editor_state().editor_ui.shape_picker.open);
    assert!(host.editor_state().editor_ui.icon_picker.open);
}

#[test]
fn icon_picker_open_owns_keyboard_search() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.icon_picker.open = true;

    assert!(host.input_active_pub());
    assert!(host.apply_text('h'));
    assert!(host.apply_text('o'));
    assert_eq!(host.editor_state().editor_ui.icon_picker_search, "ho");
    assert!(host.apply_backspace());
    assert_eq!(host.editor_state().editor_ui.icon_picker_search, "h");
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.icon_picker.open);
}

#[test]
fn icon_picker_click_inserts_icon_font_node() {
    let mut host = WidgetHostNative::new();
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    host.editor_state_mut().editor_ui.icon_picker.open = true;
    host.editor_state_mut().editor_ui.icon_picker_search = "home".to_string();

    let panel = host
        .icon_picker_panel_rect(viewport_w, viewport_h)
        .expect("icon picker rect");
    let row_y = panel.origin.y + 40.0 + 42.0 + 20.0;
    assert!(host.apply_press(panel.origin.x + 40.0, row_y, viewport_w, viewport_h));

    assert!(!host.editor_state().editor_ui.icon_picker.open);
    let icon = host
        .editor_state()
        .doc
        .children
        .iter()
        .find_map(|node| match node {
            jian_ops_schema::node::PenNode::IconFont(icon) => Some(icon),
            _ => None,
        })
        .expect("inserted icon_font node");
    assert_eq!(icon.icon_font_name, "home");
    assert_eq!(icon.icon_font_family.as_deref(), Some("lucide"));
    assert_eq!(host.editor_state().selection.anchor.as_str(), icon.base.id);
}

#[test]
fn icon_picker_header_drag_moves_the_panel() {
    let mut host = WidgetHostNative::new();
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    host.editor_state_mut().editor_ui.icon_picker.open = true;

    let start = host
        .icon_picker_panel_rect(viewport_w, viewport_h)
        .expect("icon picker rect");
    let press_x = start.origin.x + 72.0;
    let press_y = start.origin.y + 20.0;
    assert!(host.apply_press(press_x, press_y, viewport_w, viewport_h));

    assert!(host.apply_cursor_move(press_x + 96.0, press_y + 44.0));
    let moved = host
        .icon_picker_panel_rect(viewport_w, viewport_h)
        .expect("icon picker rect after drag");

    assert_eq!(moved.origin.x, start.origin.x + 96.0);
    assert_eq!(moved.origin.y, start.origin.y + 44.0);
    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));
}

#[test]
fn codegen_select_framework_updates_state() {
    use op_editor_core::codegen::Framework;
    use op_editor_ui::widgets::property_panel_action::CodegenAction;
    use op_editor_ui::widgets::PropertyPanelAction;
    let mut host = WidgetHostNative::new();

    host.apply_property_action(PropertyPanelAction::Codegen(
        CodegenAction::SelectFramework(Framework::Vue),
    ));

    assert_eq!(host.editor_state().codegen.framework, Framework::Vue);
}

#[test]
fn codegen_generate_raises_pending_and_generating_phase() {
    use op_editor_core::codegen::CodegenPhase;
    use op_editor_ui::widgets::property_panel_action::CodegenAction;
    use op_editor_ui::widgets::PropertyPanelAction;
    let mut host = WidgetHostNative::new();

    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Generate));

    assert!(host.editor_state().codegen.pending_generate);
    assert_eq!(host.editor_state().codegen.phase, CodegenPhase::Generating);
}

#[test]
fn codegen_copy_queues_code_to_system_clipboard() {
    use op_editor_ui::widgets::property_panel_action::CodegenAction;
    use op_editor_ui::widgets::PropertyPanelAction;
    let mut host = WidgetHostNative::new();
    host.set_now_ms(4242);
    host.editor_state_mut().codegen.code = "export const App = () => null;".to_string();

    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Copy));

    assert_eq!(host.editor_state().codegen.copied_at, Some(4242));
    assert_eq!(
        host.editor_state().chat.pending_copy_text.as_deref(),
        Some("export const App = () => null;"),
    );
}

#[test]
fn codegen_copy_button_queues_full_code_even_with_selection() {
    use op_editor_core::codegen::CodeSelection;
    use op_editor_ui::widgets::property_panel_action::CodegenAction;
    use op_editor_ui::widgets::PropertyPanelAction;
    let mut host = WidgetHostNative::new();
    host.set_now_ms(4242);
    host.editor_state_mut().codegen.code = "export const App = () => null;".to_string();
    host.editor_state_mut().codegen.code_selection = Some(CodeSelection {
        anchor: 7,
        focus: 12,
    });

    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Copy));

    assert_eq!(host.editor_state().codegen.copied_at, Some(4242));
    assert_eq!(
        host.editor_state().chat.pending_copy_text.as_deref(),
        Some("export const App = () => null;"),
    );
}

#[test]
fn shortcut_copy_prefers_selected_code_text() {
    use op_editor_core::codegen::CodeSelection;
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().codegen.code = "export const App = () => null;".to_string();
    host.editor_state_mut().codegen.code_selection = Some(CodeSelection {
        anchor: 7,
        focus: 12,
    });

    assert!(host.apply_copy());
    assert_eq!(
        host.editor_state().chat.pending_copy_text.as_deref(),
        Some("const"),
    );
}

#[test]
fn codegen_hover_tracks_idle_generate_button() {
    use op_editor_core::codegen::CodegenHover;
    use op_editor_core::PropertyTab;
    use op_editor_ui::widgets::property_panel_action::CodegenAction;
    use op_editor_ui::widgets::property_panel_inputs::TAB_HEIGHT;
    use op_editor_ui::widgets::{property_panel_code, TOP_BAR_HEIGHT};
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[{"type":"rectangle","id":"n-code","name":"n-code","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    host.last_viewport_w = 1200.0;
    host.last_viewport_h = 800.0;
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n-code"));
    host.editor_state_mut().editor_ui.property_tab = PropertyTab::Code;

    let pw = host.editor_state().editor_ui.property_panel_width;
    let panel_x = host.last_viewport_w - pw;
    let (_, generate_rect) = property_panel_code::code_action_rects(
        panel_x,
        TOP_BAR_HEIGHT + TAB_HEIGHT,
        pw,
        &host.editor_state().codegen,
    )
    .into_iter()
    .find(|(action, _)| matches!(action, CodegenAction::Generate))
    .expect("Generate rect present");
    let point = op_editor_ui::Point2D::new(
        generate_rect.origin.x + generate_rect.size.x / 2.0,
        generate_rect.origin.y + generate_rect.size.y / 2.0,
    );

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(
        host.editor_state().codegen.action_hover,
        Some(CodegenHover::Generate)
    );

    assert!(host.apply_cursor_move(panel_x - 12.0, TOP_BAR_HEIGHT + TAB_HEIGHT));
    assert_eq!(host.editor_state().codegen.action_hover, None);
}

#[test]
fn codegen_preview_drag_selects_code_text() {
    use op_editor_core::codegen::{CodeSelection, CodegenPhase};
    use op_editor_core::PropertyTab;
    use op_editor_ui::widgets::TOP_BAR_HEIGHT;
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[{"type":"rectangle","id":"n-code","name":"n-code","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n-code"));
    host.editor_state_mut().editor_ui.property_tab = PropertyTab::Code;
    host.editor_state_mut().codegen.phase = CodegenPhase::Complete;
    host.editor_state_mut().codegen.code = "import React\nconst n = 1".into();

    let panel_x = viewport_w - host.editor_state().editor_ui.property_panel_width;
    let char_w = 11.0 * 0.55;
    let start = op_editor_ui::Point2D::new(panel_x + 66.0, TOP_BAR_HEIGHT + 112.0);
    let end = op_editor_ui::Point2D::new(panel_x + 66.0 + char_w * 6.0, TOP_BAR_HEIGHT + 112.0);

    assert!(host.apply_press(start.x, start.y, viewport_w, viewport_h));
    assert!(host.apply_cursor_move(end.x, end.y));
    assert_eq!(
        host.editor_state().codegen.code_selection,
        Some(CodeSelection {
            anchor: 0,
            focus: 6
        })
    );
    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));
}

#[test]
fn codegen_preview_wheel_scrolls_code_not_property_panel() {
    use op_editor_core::codegen::CodegenPhase;
    use op_editor_core::PropertyTab;
    use op_editor_ui::widgets::TOP_BAR_HEIGHT;
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[{"type":"rectangle","id":"n-code","name":"n-code","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n-code"));
    host.editor_state_mut().editor_ui.property_tab = PropertyTab::Code;
    host.editor_state_mut().codegen.phase = CodegenPhase::Complete;
    host.editor_state_mut().codegen.code = (0..100)
        .map(|i| format!("line-{i:02}"))
        .collect::<Vec<_>>()
        .join("\n");

    let panel_x = viewport_w - host.editor_state().editor_ui.property_panel_width;
    assert!(host.apply_wheel(
        panel_x + 80.0,
        TOP_BAR_HEIGHT + 112.0,
        -160.0,
        viewport_w,
        viewport_h
    ));

    assert!(host.editor_state().codegen.code_scroll.offset > 0.0);
    assert_eq!(
        host.editor_state().editor_ui.property_panel_scroll.offset,
        0.0
    );
}

#[test]
fn property_tab_hover_tracks_inactive_design_tab() {
    use op_editor_core::PropertyTab;
    use op_editor_ui::widgets::TOP_BAR_HEIGHT;
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[{"type":"rectangle","id":"n-tabs","name":"n-tabs","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    host.last_viewport_w = 1200.0;
    host.last_viewport_h = 800.0;
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n-tabs"));
    host.editor_state_mut().editor_ui.property_tab = PropertyTab::Code;

    let panel_x = host.last_viewport_w - host.editor_state().editor_ui.property_panel_width;
    assert!(host.apply_cursor_move(panel_x + 40.0, TOP_BAR_HEIGHT + 18.0));
    assert_eq!(
        host.editor_state().editor_ui.property_tab_hover,
        Some(PropertyTab::Design)
    );

    assert!(host.apply_cursor_move(panel_x - 12.0, TOP_BAR_HEIGHT + 18.0));
    assert_eq!(host.editor_state().editor_ui.property_tab_hover, None);
}

#[test]
fn codegen_cancel_resets_phase_by_code_presence() {
    use op_editor_core::codegen::CodegenPhase;
    use op_editor_ui::widgets::property_panel_action::CodegenAction;
    use op_editor_ui::widgets::PropertyPanelAction;
    let mut host = WidgetHostNative::new();
    // No code yet → Cancel falls back to Idle.
    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Generate));
    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Cancel));
    assert!(!host.editor_state().codegen.pending_generate);
    assert_eq!(host.editor_state().codegen.phase, CodegenPhase::Idle);

    // With code present → Cancel returns to Complete.
    host.editor_state_mut().codegen.code = "rendered".to_string();
    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Generate));
    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Cancel));
    assert_eq!(host.editor_state().codegen.phase, CodegenPhase::Complete);
}
