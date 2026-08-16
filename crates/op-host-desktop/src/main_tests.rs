//! Redraw-scheduler tests, split from `main.rs` to keep it under the line
//! cap. The live-MCP server tests live in the sibling `main_mcp_tests.rs`.

use super::*;
use winit::keyboard::{Key, NamedKey};

#[test]
fn distinct_headless_modes_are_rejected_before_dispatch() {
    assert_eq!(
        conflicting_headless_modes([
            "--render-shots",
            "input.op",
            "shots",
            "--enrich-images",
            "input.op",
            "output.op",
        ]),
        Some(vec!["--render-shots", "--enrich-images"])
    );
    assert_eq!(
        conflicting_headless_modes(["--mcp-http", "3100", "input.op", "--tcc-selftest"]),
        Some(vec!["--mcp-http", "--tcc-selftest"])
    );
}

#[test]
fn one_headless_mode_or_repeated_same_flag_is_not_a_conflict() {
    assert_eq!(
        conflicting_headless_modes(["--render-shots", "input.op", "shots"]),
        None
    );
    assert_eq!(
        conflicting_headless_modes(["--enrich-images", "input.op", "--enrich-images"]),
        None
    );
    assert_eq!(conflicting_headless_modes(["document.op"]), None);
}

#[test]
fn launching_the_app_starts_the_chat_panel_minimized() {
    let app = DesktopApp::new(None);

    assert!(
        app.host.editor_state().chat.is_minimized(),
        "the app opens on its canvas, with the AI panel as a compact bar"
    );
}

#[test]
fn cursor_only_redraw_without_visible_state_change_skips_present() {
    let mut app = DesktopApp::new(None);
    app.redraw_pending = true;
    app.pending_cursor_move = Some((1200.0, 20.0));

    assert!(!app.prepare_redraw());
    assert!(!app.redraw_pending);
    assert!(app.pending_cursor_move.is_none());
}

#[test]
fn consumed_press_dirties_existing_cursor_redraw_without_second_request() {
    let mut app = DesktopApp::new(None);
    app.redraw_pending = true;

    assert!(!app.request_redraw(true));
    assert!(app.prepare_redraw());
}

#[test]
fn cursor_redraw_still_paints_when_layer_hover_changes() {
    let mut app = DesktopApp::new(None);
    app.redraw_pending = true;
    // Measured from where the tree actually starts, not from the top
    // bar: a page with top-level frames — which the starter document is
    // — heads the rail with the slides tab row, and a hardcoded offset
    // would land above the first layer row and hover nothing.
    let rail = op_editor_ui::widgets::host_canvas_geometry::layer_panel_rect(
        app.host.editor_state(),
        app.viewport_height,
    );
    let rows_top = op_editor_ui::widgets::slides_panel_flow::layers_content_rect(
        app.host.editor_state(),
        rail,
    )
    .origin
    .y;
    app.pending_cursor_move = Some((20.0, rows_top + 8.0 + 28.0 + 16.0));

    assert!(app.prepare_redraw());
}

#[test]
fn layer_context_menu_hover_owns_cursor_over_left_rail() {
    use op_editor_ui::widgets::layer_context_menu::LayerContextMenu;
    use op_editor_ui::widgets::{LayerPanel, LayerPanelHit};
    use op_editor_ui::Point2D;

    let mut app = DesktopApp::new(None);
    let rail = op_editor_ui::widgets::host_canvas_geometry::layer_panel_rect(
        app.host.editor_state(),
        app.viewport_height,
    );
    let layers = op_editor_ui::widgets::slides_panel_flow::layers_content_rect(
        app.host.editor_state(),
        rail,
    );
    let panel = LayerPanel::from_editor(app.host.editor_state());
    let mut row_hit = None;
    'rows: for y_offset in (0..layers.size.y.ceil() as usize).step_by(2) {
        for x_offset in (0..layers.size.x.ceil() as usize).step_by(2) {
            let point = Point2D::new(
                layers.origin.x + x_offset as f32 + 1.0,
                layers.origin.y + y_offset as f32 + 1.0,
            );
            if let Some(LayerPanelHit::Layer(id)) = panel.hit_test(layers, point) {
                row_hit = Some((point, id));
                break 'rows;
            }
        }
    }
    let (row_point, row_id) = row_hit.expect("starter document layer row");
    assert!(app.host.apply_right_press(
        row_point.x,
        row_point.y,
        app.viewport_width,
        app.viewport_height,
    ));

    let menu_state = app
        .host
        .editor_state()
        .editor_ui
        .layer_context_menu
        .clone()
        .expect("right press opens the layer context menu");
    let menu = LayerContextMenu::for_state(app.host.editor_state(), menu_state);
    let menu_rect = menu.rect();
    let menu_x = menu_rect.origin.x + menu_rect.size.x / 2.0;
    let mut menu_hit = None;
    for y_offset in 0..menu_rect.size.y.ceil() as usize {
        let point = Point2D::new(menu_x, menu_rect.origin.y + y_offset as f32 + 0.5);
        if let Some(row) = menu.hovered_row_at(point) {
            menu_hit = Some((point, row));
            break;
        }
    }
    let (menu_point, hovered_row) = menu_hit.expect("layer context menu row");
    assert!(
        menu_point.x < app.host.editor_state().editor_ui.layer_panel_width,
        "fixture must exercise the menu area that overlaps the layer rail"
    );
    assert!(app.host.cursor_over_layer_panel(
        menu_point.x,
        menu_point.y,
        app.viewport_width,
        app.viewport_height,
    ));

    app.host.editor_state_mut().editor_ui.hovered_layer_id = Some(row_id);
    app.host.editor_state_mut().editor_ui.hovered_page_index = Some(0);
    app.pending_cursor_move = Some((menu_point.x, menu_point.y));

    assert!(app.drain_pending_cursor_move());
    let ui = &app.host.editor_state().editor_ui;
    assert_eq!(
        ui.layer_context_menu
            .as_ref()
            .and_then(|state| state.menu.hover),
        Some(hovered_row),
        "the context-menu row, not the layer underneath, must own hover"
    );
    assert_eq!(ui.hovered_layer_id, None);
    assert_eq!(ui.hovered_page_index, None);

    // Moving again within the same menu row must be a no-op. In particular,
    // the layer-hover pre-pass must not transiently set the covered row only
    // for the full cursor ladder to clear it again and force another present.
    app.pending_cursor_move = Some((menu_point.x, menu_point.y));
    assert!(
        !app.drain_pending_cursor_move(),
        "unchanged context-menu hover must not report a dirty frame"
    );
    let ui = &app.host.editor_state().editor_ui;
    assert_eq!(
        ui.layer_context_menu
            .as_ref()
            .and_then(|state| state.menu.hover),
        Some(hovered_row)
    );
    assert_eq!(ui.hovered_layer_id, None);
    assert_eq!(ui.hovered_page_index, None);

    // The menu remains globally routed while open, even after the pointer
    // leaves its footprint for another point in the layer rail. That move
    // must clear the menu-owned hover instead of being swallowed by the
    // runner's layer-panel shortcut.
    let menu_exit_point = [
        Point2D::new(rail.origin.x + 1.0, rail.origin.y + 1.0),
        Point2D::new(
            rail.origin.x + rail.size.x - 1.0,
            rail.origin.y + rail.size.y - 1.0,
        ),
    ]
    .into_iter()
    .find(|point| {
        !menu_rect.contains(*point)
            && app.host.cursor_over_layer_panel(
                point.x,
                point.y,
                app.viewport_width,
                app.viewport_height,
            )
    })
    .expect("layer rail point outside the context menu");
    app.pending_cursor_move = Some((menu_exit_point.x, menu_exit_point.y));
    assert!(app.drain_pending_cursor_move());
    assert_eq!(
        app.host
            .editor_state()
            .editor_ui
            .layer_context_menu
            .as_ref()
            .and_then(|state| state.menu.hover),
        None,
        "leaving the menu must clear its row hover"
    );
}

#[test]
fn panel_resize_drag_continues_inside_left_layer_panel() {
    let mut app = DesktopApp::new(None);
    let start_width = app.host.editor_state().editor_ui.layer_panel_width;
    let y = op_editor_ui::widgets::TOP_BAR_HEIGHT + 140.0;
    assert!(app
        .host
        .apply_press(start_width, y, app.viewport_width, app.viewport_height));
    assert!(app.host.is_resizing_panel());

    app.pending_cursor_move = Some((start_width - 72.0, y));

    assert!(app.drain_pending_cursor_move());
    assert!(
        app.host.editor_state().editor_ui.layer_panel_width < start_width,
        "in-flight panel resize must keep receiving cursor moves after the cursor enters the left panel"
    );
}

#[test]
fn hidden_model_picker_is_healed_over_the_layer_panel() {
    let mut app = DesktopApp::new(None);
    app.host.editor_state_mut().chat.minimize();
    app.host.editor_state_mut().editor_ui.chat_model_picker.open = true;
    app.pending_cursor_move = Some((20.0, op_editor_ui::widgets::TOP_BAR_HEIGHT + 20.0));

    assert!(app.drain_pending_cursor_move());
    assert!(
        !app.host.editor_state().editor_ui.chat_model_picker.open,
        "a stale picker without visible bounds must not stay modal over the layer rail"
    );
}

#[test]
fn variable_row_input_keeps_resume_time_redraws_active() {
    // Serialize against reveal-streaming design-turn tests and start from
    // a quiescent registry so only the caret blink drives the deadline.
    let _guard = crate::agent_indicator_test_lock::LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    op_editor_core::agent_indicators::clear();
    let mut app = DesktopApp::new(None);
    app.host.set_now_ms(240);
    app.host.editor_state_mut().editor_ui.variable_row_focus =
        Some(op_editor_core::editor_ui_state::VariableRowFocus::Name(0));
    app.host
        .editor_state_mut()
        .editor_ui
        .variable_row_input
        .touch(240);

    assert!(app.resume_time_needs_redraw());
    assert_eq!(app.host.next_animation_deadline_ms(), Some(740));
}

/// The runner refreshes its clock BEFORE deciding whether a timed wake
/// needs a paint, which retires the tooltip's dwell deadline en route.
/// Without a second signal that wake would be thrown away and the
/// tooltip would never appear unless the user jiggled the mouse.
#[test]
fn a_due_top_bar_tooltip_keeps_the_wake_it_scheduled() {
    let _guard = crate::agent_indicator_test_lock::LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    op_editor_core::agent_indicators::clear();
    const DWELL: u64 = op_editor_ui::widgets::top_bar_tooltip::TOOLTIP_DWELL_MS;

    let mut app = DesktopApp::new(None);
    app.host.set_now_ms(1_000);
    app.host
        .editor_state_mut()
        .editor_ui
        .set_topbar_button_hover(Some(op_editor_core::TopBarButton::OpenAssetCenter), 1_000);

    // While dwelling, the deadline is what arms the timer.
    assert_eq!(app.host.next_animation_deadline_ms(), Some(1_000 + DWELL));

    // The wake fires; the clock advances past the deadline first.
    app.host.set_now_ms(1_000 + DWELL);
    assert_eq!(app.host.next_animation_deadline_ms(), None);
    assert!(
        app.resume_time_needs_redraw(),
        "the wake scheduled for the tooltip must still paint it"
    );
}

#[test]
fn selected_count_chip_clear_click_clears_canvas_selection() {
    let mut app = DesktopApp::new(None);
    app.host.editor_state_mut().selection.set = vec![
        op_editor_core::NodeId::new("n1"),
        op_editor_core::NodeId::new("n2"),
    ];
    app.host.editor_state_mut().chat.panel_position = Some((100.0, 100.0));
    // The app launches minimized; the selection chip lives in the
    // expanded panel.
    app.host.editor_state_mut().chat.expand();
    let chat = &app.host.editor_state().chat;
    let chat_rect = op_editor_ui::Rect::xywh(100.0, 100.0, chat.panel_width, chat.panel_height);
    let panel = op_editor_ui::widgets::AIChatPlaceholder::from_editor(app.host.editor_state());
    let input = panel.input_rect(chat_rect);
    let clear_point = (0..160)
        .flat_map(|dx| (0..28).map(move |dy| (dx, dy)))
        .map(|(dx, dy)| {
            op_editor_ui::Point2D::new(input.origin.x + dx as f32, input.origin.y + dy as f32)
        })
        .find(|point| {
            panel.hit_test(chat_rect, *point)
                == Some(op_editor_ui::widgets::AIChatHit::ClearSelection)
        })
        .expect("clear-selection hit point");

    assert!(app
        .host
        .apply_click(clear_point.x, clear_point.y, 1200.0, 800.0));
    assert!(app.host.editor_state().selection.set.is_empty());
}

#[test]
fn chat_input_arrows_move_caret_before_insert() {
    let mut app = DesktopApp::new(None);
    app.host.editor_state_mut().chat.focused = true;
    app.host.editor_state_mut().chat.set_input_text("abcd");

    app.handle_key_pressed(&Key::Named(NamedKey::ArrowLeft), None);
    app.handle_key_pressed(&Key::Named(NamedKey::ArrowLeft), None);

    assert_eq!(app.host.editor_state().chat.input_caret(), 2);
    assert!(app.host.apply_text('X'));
    assert_eq!(app.host.editor_state().chat.input.text(), "abXcd");
    assert_eq!(app.host.editor_state().chat.input_caret(), 3);
}

#[test]
fn chat_ime_anchor_tracks_input_caret() {
    let mut app = DesktopApp::new(None);
    app.host.editor_state_mut().chat.focused = true;
    app.host.editor_state_mut().chat.set_input_text("abcd");
    app.host.editor_state_mut().chat.set_input_caret(0, 0);
    let start = app
        .host
        .ime_anchor_rect(1200.0, 800.0)
        .expect("chat focus should yield ime anchor");

    app.host.editor_state_mut().chat.set_input_caret(3, 0);
    let after_three = app
        .host
        .ime_anchor_rect(1200.0, 800.0)
        .expect("chat focus should yield ime anchor");

    assert!(
        after_three.origin.x > start.origin.x + 12.0,
        "expected IME anchor to move with caret: start={start:?}, after={after_three:?}"
    );
    assert!(
        after_three.size.x <= 4.0,
        "IME anchor should describe the caret, not the whole input: {after_three:?}"
    );
}

#[test]
fn fresh_app_fits_blank_frame_like_ts_canvas_init() {
    let app = DesktopApp::new(None);
    assert!(app.host.editor_state().selection.is_empty());
    let v = app.host.editor_state().viewport;

    // No implicit page inspector without a selection: fit uses the full canvas width.
    assert!((v.zoom - 0.8933333).abs() < 1e-3, "zoom {}", v.zoom);
    assert!((v.pan_x - 64.0).abs() < 1e-2, "pan_x {}", v.pan_x);
    assert!((v.pan_y - 72.66669).abs() < 1e-2, "pan_y {}", v.pan_y);
}

#[test]
fn fresh_app_refits_blank_frame_to_actual_window_size_once() {
    let mut app = DesktopApp::new(None);
    app.viewport_width = 1000.0;
    app.viewport_height = 700.0;

    assert!(app.fit_initial_blank_frame_to_actual_viewport());
    let v = app.host.editor_state().viewport;
    assert!((v.zoom - 0.52666664).abs() < 1e-3, "zoom {}", v.zoom);
    assert!((v.pan_x - 64.0).abs() < 1e-2, "pan_x {}", v.pan_x);
    assert!((v.pan_y - 119.33334).abs() < 1e-2, "pan_y {}", v.pan_y);

    app.viewport_width = 1200.0;
    app.viewport_height = 800.0;
    assert!(!app.fit_initial_blank_frame_to_actual_viewport());
    let unchanged = app.host.editor_state().viewport;
    assert_eq!(v, unchanged);
}

#[test]
fn design_md_auto_generate_does_not_fall_back_to_local_extraction() {
    use jian_ops_schema::variable::{
        VariableDefinition, VariableKind, VariableScalar, VariableValue,
    };
    use op_ai::chat_provider::{ChatDelta, EchoProvider, StopReason};
    use std::collections::BTreeMap;

    let mut app = DesktopApp::new(None);
    let mut variables = BTreeMap::new();
    variables.insert(
        "$color-brand".to_string(),
        VariableDefinition {
            kind: VariableKind::Color,
            value: VariableValue::Scalar(VariableScalar::Str("#2563eb".to_string())),
        },
    );
    {
        let state = app.host.editor_state_mut();
        state.doc.name = Some("Generated Brief".to_string());
        state.doc.variables = Some(variables);
        state.doc.design_md = Some(op_editor_core::parse_design_md(
            "# Design System: Existing\n\n## Visual Theme\nOld brief",
        ));
        state.editor_ui.design_md_panel.request =
            Some(op_editor_core::DesignMdRequest::AutoGenerate);
    }
    app.set_design_md_test_provider(Box::new(EchoProvider {
        script: vec![
            ChatDelta::TextDelta(
                "# Design System: LLM Brief\n\n\
                 ## 1. Visual Theme & Atmosphere\n\
                 Model-authored brief.\n\n\
                 ## 2. Color Palette & Roles\n\
                 **AI Orange** (#F97316) — Primary accent"
                    .into(),
            ),
            ChatDelta::Done {
                stop_reason: StopReason::EndTurn,
            },
        ],
    }));

    assert!(app.drain_design_md_action());
    assert!(app.host.editor_state().editor_ui.design_md_panel.generating);

    let spec = app
        .host
        .editor_state()
        .doc
        .design_md
        .as_ref()
        .expect("existing design.md should remain until an LLM result lands");
    assert_eq!(spec.project_name.as_deref(), Some("Existing"));
    assert!(
        !spec.raw.contains("#2563EB"),
        "auto-generate must not masquerade as AI by using local extraction: {}",
        spec.raw
    );

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !app.poll_design_md_generation() {
        assert!(
            std::time::Instant::now() < deadline,
            "design.md generation worker did not finish"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    let spec = app
        .host
        .editor_state()
        .doc
        .design_md
        .as_ref()
        .expect("LLM-generated design.md");
    assert_eq!(spec.project_name.as_deref(), Some("LLM Brief"));
    assert!(spec.raw.contains("#F97316"));
    assert!(!spec.raw.contains("#2563EB"));
    assert!(!app.host.editor_state().editor_ui.design_md_panel.generating);

    assert!(app.host.editor_state_mut().undo());
    let restored = app
        .host
        .editor_state()
        .doc
        .design_md
        .as_ref()
        .expect("previous design.md restored");
    assert_eq!(restored.project_name.as_deref(), Some("Existing"));
}
