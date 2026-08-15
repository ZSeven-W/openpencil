use super::WidgetHost;
use op_editor_core::agent_settings::BuiltinModelMenuTarget;
use op_editor_core::{
    BuiltinAgentField, BuiltinAgentKind, BuiltinModelCatalogRefreshOutcome,
    BuiltinModelCatalogTarget, BuiltinModelOption, SettingsFocus, Tool,
};
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 1200.0;
const VIEWPORT_H: f32 = 800.0;

fn canvas_point(host: &WidgetHost, x: f32, y: f32) -> Point2D {
    let (cx0, cy0, _, _) = host.canvas_region(VIEWPORT_W, VIEWPORT_H);
    Point2D::new(cx0 + x, cy0 + y)
}

#[test]
fn hand_tool_drag_pans_the_viewport() {
    let mut host = WidgetHost::new();
    host.editor_state.tool = Tool::Hand;

    let start = canvas_point(&host, 420.0, 260.0);
    let end = canvas_point(&host, 470.0, 295.0);

    let _ = host.apply_press(start.x, start.y, VIEWPORT_W, VIEWPORT_H);
    assert!(host.apply_cursor_move(end.x, end.y));
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));

    assert_eq!(host.editor_state.viewport.pan_x, 50.0);
    assert_eq!(host.editor_state.viewport.pan_y, 35.0);
}

#[test]
fn space_pan_drag_pans_even_when_select_tool_is_active() {
    let mut host = WidgetHost::new();
    host.editor_state.tool = Tool::Select;
    host.set_space_pan(true);

    let start = canvas_point(&host, 420.0, 260.0);
    let end = canvas_point(&host, 470.0, 295.0);

    let _ = host.apply_press(start.x, start.y, VIEWPORT_W, VIEWPORT_H);
    assert!(host.apply_cursor_move(end.x, end.y));
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));

    assert_eq!(host.editor_state.viewport.pan_x, 50.0);
    assert_eq!(host.editor_state.viewport.pan_y, 35.0);
    assert!(host.marquee_drag.is_none());
}

#[test]
fn horizontal_trackpad_pan_moves_canvas_viewport() {
    let mut host = WidgetHost::new();
    let point = canvas_point(&host, 420.0, 260.0);

    assert!(host.apply_pan_gesture(point.x, point.y, -120.0, 0.0, VIEWPORT_W, VIEWPORT_H));

    assert_eq!(host.editor_state.viewport.pan_x, -120.0);
    assert_eq!(host.editor_state.viewport.pan_y, 0.0);
}

fn seed_scrollable_agent_settings(host: &mut WidgetHost) {
    host.editor_state.editor_ui.agent_settings_open = true;
    for index in 0..12 {
        host.editor_state
            .editor_ui
            .agent_settings
            .add_builtin_agent_config(
                format!("Provider {index}"),
                format!("sk-test-{index}"),
                format!("model-{index}"),
                BuiltinAgentKind::OpenAiCompat,
                "https://api.example.com/v1",
            );
    }
}

fn settings_content_point(host: &WidgetHost) -> Point2D {
    let panel = op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel::for_web_editor(
        &host.editor_state,
    );
    let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
    let content = panel.resolved_content_viewport(rect);
    Point2D::new(
        content.origin.x + content.size.x / 2.0,
        content.origin.y + 12.0,
    )
}

#[test]
fn trackpad_pan_scrolls_agent_settings_body_without_moving_canvas() {
    let mut host = WidgetHost::new();
    seed_scrollable_agent_settings(&mut host);
    let point = settings_content_point(&host);

    // Real macOS trackpad events often carry a tiny horizontal component,
    // which classifies the browser wheel as Pan instead of Wheel/Zoom.
    assert!(host.apply_pan_gesture(point.x, point.y, -0.5, -80.0, VIEWPORT_W, VIEWPORT_H,));

    assert!(host.editor_state.editor_ui.agent_settings.scroll_y.offset > 0.0);
    assert_eq!(host.editor_state.viewport.pan_x, 0.0);
    assert_eq!(host.editor_state.viewport.pan_y, 0.0);

    // A horizontal-only gesture over the opaque modal is still owned by the
    // modal and must not leak through to canvas pan.
    let body_offset = host.editor_state.editor_ui.agent_settings.scroll_y.offset;
    assert!(host.apply_pan_gesture(point.x, point.y, -90.0, 0.0, VIEWPORT_W, VIEWPORT_H,));
    assert_eq!(
        host.editor_state.editor_ui.agent_settings.scroll_y.offset,
        body_offset
    );
    assert_eq!(host.editor_state.viewport.pan_x, 0.0);
    assert_eq!(host.editor_state.viewport.pan_y, 0.0);
}

#[test]
fn trackpad_pan_over_model_menu_scrolls_menu_before_settings_body() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.agent_settings_open = true;
    let id = host
        .editor_state
        .editor_ui
        .agent_settings
        .add_builtin_agent_config(
            "Provider",
            "sk-test",
            "model-0",
            BuiltinAgentKind::OpenAiCompat,
            "https://api.example.com/v1",
        );
    let target = BuiltinModelCatalogTarget::Agent(id);
    let request = host
        .editor_state
        .editor_ui
        .agent_settings
        .begin_builtin_model_catalog_refresh(target, 0)
        .expect("configured provider starts discovery");
    let expected = host
        .editor_state
        .editor_ui
        .agent_settings
        .builtin_model_catalog_config_for_request(&request)
        .expect("current discovery config");
    assert!(host
        .editor_state
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
    host.editor_state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index: 0,
        field: BuiltinAgentField::Model,
    });
    host.editor_state
        .editor_ui
        .agent_settings
        .builtin_model_menu_open = Some(BuiltinModelMenuTarget::Agent(0));

    let point = {
        let panel = op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel::for_web_editor(
            &host.editor_state,
        );
        let rect = panel.rect(VIEWPORT_W, VIEWPORT_H);
        let content = panel.resolved_content_viewport(rect);
        let x = content.origin.x + content.size.x * 0.65;
        (0..content.size.y.ceil() as usize)
            .map(|row| Point2D::new(x, content.origin.y + row as f32 + 0.5))
            .find(|point| {
                panel
                    .builtin_model_scroll_max_at(rect, *point)
                    .is_some_and(|max| max > 0.0)
            })
            .expect("open model menu is inside settings viewport")
    };

    assert!(host.apply_pan_gesture(point.x, point.y, -0.5, -48.0, VIEWPORT_W, VIEWPORT_H,));

    let settings = &host.editor_state.editor_ui.agent_settings;
    assert!(settings.builtin_model_menu_scroll.offset > 0.0);
    assert_eq!(settings.scroll_y.offset, 0.0);
    assert_eq!(host.editor_state.viewport.pan_x, 0.0);
    assert_eq!(host.editor_state.viewport.pan_y, 0.0);
}

#[test]
fn browser_zoom_uses_accelerated_delta_and_keeps_pointer_anchor() {
    let mut host = WidgetHost::new();
    let point = canvas_point(&host, 420.0, 260.0);
    let (cx0, cy0, _, _) = host.canvas_region(VIEWPORT_W, VIEWPORT_H);
    let local = Point2D::new(point.x - cx0, point.y - cy0);
    let before = host.editor_state.viewport.to_document(local);

    assert!(
        host.apply_wheel_with_canvas_delta(point.x, point.y, -10.0, -40.0, VIEWPORT_W, VIEWPORT_H,)
    );

    let expected_zoom = (-40.0_f32 * 0.0015).exp();
    assert!((host.editor_state.viewport.zoom - expected_zoom).abs() < 1e-6);
    let after = host.editor_state.viewport.to_document(local);
    assert!((before.x - after.x).abs() < 1e-4);
    assert!((before.y - after.y).abs() < 1e-4);
}

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

fn seed(host: &mut WidgetHost, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.mark_dirty();
}

fn run_deep_layer_fixture(test: impl FnOnce() + Send + 'static) {
    let handle = std::thread::Builder::new()
        .name("op-host-web-deep-layer-fixture".into())
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
        let mut host = WidgetHost::new();
        seed(&mut host, &nested_frame_doc(50));
        let panel = op_editor_ui::widgets::LayerPanel::from_editor(host.editor_state());
        // Through the host's own rect, not a hand-built one: a document
        // with top-level frames shows the rail's tab row, and the tree
        // starts below it.
        let regions = panel.regions(host.layers_content_rect(VIEWPORT_H));
        assert!(regions.layers.max_horizontal_offset > 0.0);

        assert!(host.apply_pan_gesture(
            80.0,
            regions.layers_rows_top + 12.0,
            -180.0,
            0.0,
            VIEWPORT_W,
            VIEWPORT_H
        ));

        assert!(host.editor_state().editor_ui.layer_layers_h_scroll.offset > 0.0);
    });
}

#[test]
fn middle_pan_press_starts_canvas_pan_without_primary_press_dispatch() {
    let mut host = WidgetHost::new();
    host.editor_state.tool = Tool::Select;

    let start = canvas_point(&host, 420.0, 260.0);
    let end = canvas_point(&host, 450.0, 285.0);

    assert!(host.apply_pan_press(start.x, start.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.apply_cursor_move(end.x, end.y));
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));

    assert_eq!(host.editor_state.viewport.pan_x, 30.0);
    assert_eq!(host.editor_state.viewport.pan_y, 25.0);
    assert!(host.marquee_drag.is_none());
}

fn assert_panel_blocks_middle_pan(host: &mut WidgetHost, panel: op_editor_ui::Rect, label: &str) {
    let point = Point2D::new(
        panel.origin.x + panel.size.x / 2.0,
        panel.origin.y + panel.size.y / 2.0,
    );
    assert!(
        host.over_canvas(point.x, point.y, VIEWPORT_W, VIEWPORT_H),
        "{label} fixture point must otherwise be eligible for canvas pan"
    );
    assert!(
        host.over_topmost_panel(point.x, point.y, VIEWPORT_W, VIEWPORT_H),
        "{label} must participate in the shared topmost-panel predicate"
    );
    assert!(
        !host.apply_pan_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H),
        "middle-button pan must not start through {label}"
    );
    assert!(host.drag.is_none());
}

#[test]
fn middle_pan_press_is_blocked_by_every_topmost_floating_panel() {
    let mut design = WidgetHost::new();
    design.editor_state.editor_ui.design_md_panel.open = true;
    let rect = design
        .design_md_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("Design-MD rect");
    assert_panel_blocks_middle_pan(&mut design, rect, "Design-MD");

    let mut variables = WidgetHost::new();
    variables.editor_state.editor_ui.variables_panel_open = true;
    let rect = variables
        .variables_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("Variables rect");
    assert_panel_blocks_middle_pan(&mut variables, rect, "Variables");

    let mut icon = WidgetHost::new();
    icon.editor_state.editor_ui.icon_picker.open = true;
    let rect = icon
        .icon_picker_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("Icon Picker rect");
    assert_panel_blocks_middle_pan(&mut icon, rect, "Icon Picker");

    let mut prompt = WidgetHost::new();
    prompt.editor_state.editor_ui.open_prompt_center(1);
    let rect = prompt
        .prompt_center_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("Prompt Center rect");
    assert_panel_blocks_middle_pan(&mut prompt, rect, "Prompt Center");

    let mut components = WidgetHost::new();
    components.editor_state.editor_ui.component_browser_open = true;
    let rect = components
        .component_browser_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("Component Browser rect");
    assert_panel_blocks_middle_pan(&mut components, rect, "Component Browser");

    // The HTML-import diagnostics card is non-modal but opaque: a wheel or
    // pan started on its header must not reach the canvas beneath it.
    let mut diagnostics = WidgetHost::new();
    diagnostics.show_html_import_diagnostics(vec![
        op_editor_core::html_import_diagnostics::HtmlImportDiagnostic::new(
            "layout.float_ignored",
            "htmlImport.warn.layout.float_ignored",
            Vec::new(),
            "CSS float ignored during structured HTML import",
        ),
    ]);
    let rect = diagnostics
        .html_import_diagnostics_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("diagnostics card rect");
    assert_panel_blocks_middle_pan(&mut diagnostics, rect, "HTML import diagnostics");
}

#[test]
fn a_dismissed_diagnostics_card_stops_owning_its_rect() {
    let mut host = WidgetHost::new();
    host.show_html_import_diagnostics(vec![
        op_editor_core::html_import_diagnostics::HtmlImportDiagnostic::new(
            "layout.float_ignored",
            "htmlImport.warn.layout.float_ignored",
            Vec::new(),
            "CSS float ignored during structured HTML import",
        ),
    ]);
    let rect = host
        .html_import_diagnostics_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("diagnostics card rect");
    let point = Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    );
    assert!(host.over_topmost_panel(point.x, point.y, VIEWPORT_W, VIEWPORT_H));

    op_editor_ui::widgets::html_import_diagnostics_flow::dismiss(&mut host.editor_state);
    assert!(host
        .html_import_diagnostics_rect(VIEWPORT_W, VIEWPORT_H)
        .is_none());
    assert!(!host.over_topmost_panel(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
}
