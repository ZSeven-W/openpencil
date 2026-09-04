use super::WidgetHostNative;

fn nested_frame_doc(depth: usize) -> String {
    let mut src = String::from(r#"{"version":"1.0.0","children":["#);
    for i in 0..depth {
        src.push_str(&format!(
            r##"{{"type":"frame","id":"nest-{i:05}","name":"Nested Layer {i:05}","x":8,"y":6,"width":400,"height":220,"fill":[{{"type":"solid","color":"#ffffff20"}}],"stroke":{{"thickness":1,"fill":[{{"type":"solid","color":"#0088ff"}}]}},"children":["##
        ));
    }
    for _ in 0..depth {
        src.push_str("]}");
    }
    src.push_str("]}");
    src
}

fn seed(host: &mut WidgetHostNative, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.mark_paint_dirty_for_test();
}

fn run_deep_layer_fixture(test: impl FnOnce() + Send + 'static) {
    let handle = std::thread::Builder::new()
        .name("op-host-native-deep-layer-fixture".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(test)
        .expect("spawn deep layer fixture test");
    if let Err(payload) = handle.join() {
        std::panic::resume_unwind(payload);
    }
}

#[test]
fn layer_panel_trackpad_pan_scrolls_horizontally() {
    run_deep_layer_fixture(|| {
        let mut host = WidgetHostNative::new();
        seed(&mut host, &nested_frame_doc(50));
        let viewport_w = 1200.0;
        let viewport_h = 800.0;
        let panel = op_editor_ui::widgets::LayerPanel::from_editor(host.editor_state());
        // Through the host's own rect, not a hand-built one: a document
        // with top-level frames shows the rail's tab row, and the tree
        // starts below it.
        let regions = panel.regions(host.layers_content_rect(viewport_w, viewport_h));
        assert!(regions.layers.max_horizontal_offset > 0.0);

        assert!(host.apply_pan_gesture(
            80.0,
            regions.layers_rows_top + 12.0,
            -180.0,
            0.0,
            viewport_w,
            viewport_h
        ));

        assert!(host.editor_state().editor_ui.layer_layers_h_scroll.offset > 0.0);
    });
}

#[test]
fn design_md_panel_wheel_scrolls_content_without_zooming_canvas() {
    let mut host = WidgetHostNative::new();
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    let mut markdown = String::from("# Design System: Long\n\n## Color Palette\n");
    for index in 0..40 {
        markdown.push_str(&format!(
            "- **color-{index:02}** (#{index:02X}{index:02X}{index:02X}) - role {index}\n"
        ));
    }
    host.editor_state_mut().editor_ui.design_md_panel.open = true;
    host.editor_state_mut().doc.design_md = Some(op_editor_core::parse_design_md(&markdown));
    let panel_rect = host
        .design_md_panel_rect(viewport_w, viewport_h)
        .expect("design.md panel rect");
    let panel = op_editor_ui::widgets::DesignMdPanel::for_editor(host.editor_state())
        .expect("design.md panel");
    assert!(panel.max_scroll(panel_rect) > 0.0);
    let zoom = host.editor_state().viewport.zoom;

    assert!(host.apply_wheel(
        panel_rect.origin.x + panel_rect.size.x / 2.0,
        panel_rect.origin.y + panel_rect.size.y / 2.0,
        -120.0,
        viewport_w,
        viewport_h
    ));

    assert!(host.editor_state().editor_ui.design_md_panel.scroll.offset > 0.0);
    assert_eq!(host.editor_state().viewport.zoom, zoom);
}

#[test]
fn locale_picker_wheel_scrolls_select_state_without_zooming_canvas() {
    let mut host = WidgetHostNative::new();
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    host.editor_state_mut().editor_ui.locale_picker.open = true;
    host.editor_state_mut().editor_ui.locale_picker.hover = Some(0);
    let zoom = host.editor_state().viewport.zoom;
    let picker = host.locale_picker_rect(viewport_w);

    assert!(host.apply_wheel(
        picker.origin.x + picker.size.x / 2.0,
        picker.origin.y + op_editor_ui::widgets::LocalePicker::row_height(),
        -80.0,
        viewport_w,
        viewport_h
    ));

    let state = &host.editor_state().editor_ui.locale_picker;
    assert!(state.open);
    assert!(state.scroll.offset > 0.0);
    assert_eq!(state.hover, None);
    assert_eq!(host.editor_state().viewport.zoom, zoom);
}

#[test]
fn model_menu_wheel_recomputes_the_stationary_pointer_hover() {
    use op_editor_core::agent_settings::BuiltinModelMenuTarget;
    use op_editor_core::{
        BuiltinAgentField, BuiltinAgentKind, BuiltinModelCatalogRefreshOutcome,
        BuiltinModelCatalogTarget, BuiltinModelOption, SettingsFocus,
    };

    const VIEWPORT_W: f32 = 1200.0;
    const VIEWPORT_H: f32 = 800.0;
    let mut host = WidgetHostNative::new();
    host.publish_viewport_geometry(VIEWPORT_W, VIEWPORT_H);
    host.editor_state_mut().editor_ui.agent_settings_open = true;
    let id = host
        .editor_state_mut()
        .editor_ui
        .agent_settings
        .add_builtin_agent_config(
            "Provider",
            "sk-test",
            "model-0",
            BuiltinAgentKind::OpenAiCompat,
            "https://api.example.com/v1",
        );
    let request = host
        .editor_state_mut()
        .editor_ui
        .agent_settings
        .begin_builtin_model_catalog_refresh(BuiltinModelCatalogTarget::Agent(id), 0)
        .expect("configured provider starts discovery");
    let expected = host
        .editor_state()
        .editor_ui
        .agent_settings
        .builtin_model_catalog_config_for_request(&request)
        .expect("current discovery config");
    assert!(host
        .editor_state_mut()
        .editor_ui
        .agent_settings
        .apply_builtin_model_catalog_refresh_outcome_if_current(
            &expected,
            &request,
            BuiltinModelCatalogRefreshOutcome::Success {
                models: (0..12)
                    .map(|index| {
                        BuiltinModelOption::new(format!("model-{index}"), format!("Model {index}"))
                    })
                    .collect(),
            },
        ));
    {
        let settings = &mut host.editor_state_mut().editor_ui.agent_settings;
        settings.focus = Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::Model,
        });
        settings.builtin_model_menu_open = Some(BuiltinModelMenuTarget::Agent(0));
    }

    let point = {
        let panel = op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel::for_editor(
            host.editor_state(),
        );
        let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
        let content = panel.resolved_content_viewport(rect);
        let x = content.origin.x + content.size.x * 0.65;
        (0..content.size.y.ceil() as usize)
            .map(|row| op_editor_ui::Point2D::new(x, content.origin.y + row as f32 + 0.5))
            .find(|point| panel.builtin_model_hover_at(rect, *point) == Some(0))
            .expect("first model row is visible")
    };
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .builtin_model_menu_hover = Some(0);

    let outer_before = host.editor_state().editor_ui.agent_settings.scroll_y.offset;
    let zoom_before = host.editor_state().viewport.zoom;
    assert!(host.apply_wheel(point.x, point.y, 48.0, VIEWPORT_W, VIEWPORT_H));
    let settings = &host.editor_state().editor_ui.agent_settings;
    assert_eq!(settings.builtin_model_menu_scroll.offset, 0.0);
    assert_eq!(settings.scroll_y.offset, outer_before);
    assert_eq!(host.editor_state().viewport.zoom, zoom_before);

    assert!(host.apply_wheel(point.x, point.y, -48.0, VIEWPORT_W, VIEWPORT_H));
    let settings = &host.editor_state().editor_ui.agent_settings;
    assert_eq!(settings.builtin_model_menu_scroll.offset, 48.0);
    assert_eq!(
        settings.builtin_model_menu_hover,
        Some(2),
        "the same screen point now covers the third model row"
    );

    host.editor_state_mut().editor_ui.touch = true;
    host.refresh_agent_settings_hover_after_scroll(point.x, point.y);
    assert_eq!(
        host.editor_state()
            .editor_ui
            .agent_settings
            .builtin_model_menu_hover,
        None,
        "touch scrolling must not leave a persistent hover wash"
    );
}

#[test]
fn canvas_pan_gesture_opens_and_closes_the_interactive_degrade_window() {
    let mut host = WidgetHostNative::new();
    host.set_now_ms(1_000);
    assert!(!host.fast_interaction_active());

    // A trackpad pan over the canvas marks the gesture hot: the canvas
    // paints interactive-degraded and the scheduler wakes exactly when
    // the window closes so the release frame restores full quality.
    assert!(host.apply_pan_gesture(600.0, 400.0, 5.0, 0.0, 1200.0, 800.0));
    assert!(host.fast_interaction_active());
    let deadline = host
        .next_animation_deadline_ms()
        .expect("hot gesture schedules a wake-up");
    assert!(deadline <= 1_000 + super::host_requests::INTERACTION_HOT_MS);

    host.set_now_ms(1_000 + super::host_requests::INTERACTION_HOT_MS);
    assert!(!host.fast_interaction_active());
}

#[test]
fn canvas_zoom_wheel_marks_the_gesture_hot() {
    let mut host = WidgetHostNative::new();
    host.set_now_ms(2_000);
    assert!(host.apply_wheel(600.0, 400.0, -40.0, 1200.0, 800.0));
    assert!(host.fast_interaction_active());
}
