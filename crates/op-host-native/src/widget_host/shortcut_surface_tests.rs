//! `#[cfg(test)]` companion for the shortcut-surface host methods
//! added for TS parity (panel toggles, create-component, space-pan).

use super::WidgetHostNative;
use op_editor_core::{
    agent_settings::SettingsFocus, figma_import_state::ImportSource, ui_draft::ColorTarget,
    AgentSettingsTab, MissingFontSurface, NodeId, PropertyFocus,
};

fn seed(host: &mut WidgetHostNative, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.mark_paint_dirty_for_test();
}

const ONE_RECT: &str = r#"{"version":"1.0.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":400,"y":300,"width":100,"height":50}]}"#;

const ONE_FRAME: &str = r#"{"version":"1.0.0","children":[{"type":"frame","id":"f1","name":"Card","x":0,"y":0,"width":100,"height":50}]}"#;

const ONE_TEXT: &str = r##"{"version":"1.0.0","children":[{"type":"text","id":"t1","name":"Title","x":0,"y":0,"width":100,"height":24,"content":"hello","font_size":16,"fills":[{"type":"solid","color":"#111827"}]}]}"##;

#[test]
fn panel_toggle_shortcuts_flip_their_flags() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, ONE_FRAME);

    assert!(host.apply_toggle_variables_panel());
    assert!(host.editor_state().editor_ui.variables_panel_open);
    assert!(host.apply_toggle_variables_panel());
    assert!(!host.editor_state().editor_ui.variables_panel_open);

    assert!(host.apply_toggle_design_md_panel());
    assert!(host.editor_state().editor_ui.design_md_panel.open);

    assert!(host.apply_toggle_component_browser());
    assert!(host.editor_state().editor_ui.component_browser_open);
}

#[test]
fn import_shortcuts_open_the_requested_source_without_stale_state() {
    let mut host = WidgetHostNative::new();

    host.editor_state_mut().editor_ui.import_source = ImportSource::Html;
    host.editor_state_mut().editor_ui.import_menu_open = true;
    host.editor_state_mut().editor_ui.import_menu.open = true;
    assert!(host.apply_open_figma_import());
    assert!(host.editor_state().editor_ui.figma_import_open);
    assert_eq!(
        host.editor_state().editor_ui.import_source,
        ImportSource::Figma
    );
    assert!(!host.editor_state().editor_ui.import_menu_open);

    host.editor_state_mut().editor_ui.figma_import_open = false;
    host.editor_state_mut().editor_ui.import_source = ImportSource::Figma;
    assert!(host.apply_open_html_import());
    assert!(host.editor_state().editor_ui.figma_import_open);
    assert_eq!(
        host.editor_state().editor_ui.import_source,
        ImportSource::Html
    );
}

#[test]
fn import_shortcut_does_not_open_beneath_an_existing_modal() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.import_source = ImportSource::Html;
    host.editor_state_mut().editor_ui.agent_settings_open = true;
    host.editor_state_dirty = false;

    assert!(host.apply_open_figma_import(), "the chord stays consumed");

    let ui = &host.editor_state().editor_ui;
    assert!(ui.agent_settings_open);
    assert!(!ui.figma_import_open);
    assert_eq!(ui.import_source, ImportSource::Html);
    assert!(!host.editor_state_dirty);
}

#[test]
fn closing_settings_clears_focus_before_an_import_shortcut() {
    let mut host = WidgetHostNative::new();
    {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.agent_settings_open = true;
        ui.agent_settings.focus = Some(SettingsFocus::McpPort);
        ui.settings_input.set_text("4321");
    }

    assert!(host.apply_toggle_agent_settings());
    assert!(!host.editor_state().editor_ui.agent_settings_open);
    assert!(host.editor_state().editor_ui.agent_settings.focus.is_none());
    assert_eq!(
        host.editor_state().editor_ui.agent_settings.mcp_server.port,
        4321
    );

    assert!(host.apply_open_figma_import());
    assert!(host.editor_state().editor_ui.figma_import_open);
}

#[test]
fn closing_settings_closes_its_font_picker_and_releases_input_ownership() {
    let mut host = WidgetHostNative::new();
    {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.agent_settings_open = true;
        ui.agent_settings.tab = AgentSettingsTab::Fonts;
        ui.open_missing_font_picker(0, MissingFontSurface::Settings);
        ui.font_picker_search = "inter".into();
        ui.ime_preedit = Some(Default::default());
    }
    assert!(host.input_active(), "the visible picker owns text input");

    assert!(host.apply_toggle_agent_settings());

    let ui = &host.editor_state().editor_ui;
    assert!(!ui.agent_settings_open);
    assert!(!ui.font_picker.open);
    assert!(ui.font_picker_purpose.is_none());
    assert!(ui.ime_preedit.is_none());
    assert!(!host.input_active(), "no hidden Settings input owns IME");
    assert!(!host.apply_text('x'));
    assert!(host.editor_state().editor_ui.font_picker_search.is_empty());
}

#[test]
fn opening_settings_releases_hidden_property_input_ownership() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, ONE_RECT);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n1"));
    {
        let state = host.editor_state_mut();
        state.ui.property_focus = Some(PropertyFocus::SizeW);
        state.ui.property_input.set_text("120");
    }
    assert!(host.input_active(), "the Property field owns text input");

    assert!(host.apply_toggle_agent_settings());

    assert!(host.editor_state().editor_ui.agent_settings_open);
    assert!(host.editor_state().ui.property_focus.is_none());
    assert!(
        !host.input_active(),
        "the covered Property field released IME"
    );
    let hidden_draft = host.editor_state().ui.property_input.text().to_owned();
    assert!(!host.apply_text('x'));
    assert_eq!(host.editor_state().ui.property_input.text(), hidden_draft);
}

#[test]
fn opening_a_specific_settings_tab_reveals_it_and_commits_prior_input() {
    let mut host = WidgetHostNative::new();
    {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.agent_settings.tab = AgentSettingsTab::Mcp;
        ui.agent_settings.scroll_y.offset = 84.0;
        ui.agent_settings.focus = Some(SettingsFocus::McpPort);
        ui.settings_input.set_text("4321");
        ui.design_md_panel.open = true;
        ui.component_browser_open = true;
        ui.open_icon_picker(false);
    }

    assert!(host.apply_open_agent_settings_tab(AgentSettingsTab::System));

    let ui = &host.editor_state().editor_ui;
    assert!(ui.agent_settings_open);
    assert_eq!(ui.agent_settings.tab, AgentSettingsTab::System);
    assert_eq!(ui.agent_settings.scroll_y.offset, 0.0);
    assert!(ui.agent_settings.focus.is_none());
    assert_eq!(ui.agent_settings.mcp_server.port, 4321);
    assert!(!ui.design_md_panel.open);
    assert!(!ui.component_browser_open);
    assert!(!ui.icon_picker.open);
}

#[test]
fn import_shortcut_blurs_covered_text_and_ime_owners() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, ONE_TEXT);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("t1"));
    assert!(host.editor_state_mut().start_text_edit(NodeId::new("t1")));
    assert!(host
        .editor_state_mut()
        .open_color_picker(ColorTarget::Fill, 120.0));
    {
        let state = host.editor_state_mut();
        state.ui.property_focus = Some(PropertyFocus::SizeW);
        state.chat.focused = true;
        let ui = &mut state.editor_ui;
        ui.chat_model_picker.open = true;
        ui.chat_model_picker_input.set_text("model");
        ui.font_picker.open = true;
        ui.icon_picker.open = true;
        ui.component_browser_open = true;
        ui.ime_preedit = Some(Default::default());
    }

    assert!(host.apply_open_figma_import());

    let state = host.editor_state();
    assert!(state.ui.text_editing.is_none(), "canvas text edit blurs");
    assert!(state.ui.property_focus.is_none(), "property input blurs");
    assert!(!state.chat.focused, "chat input blurs");
    assert!(!state.editor_ui.chat_model_picker.open);
    assert!(state.editor_ui.chat_model_picker_input.text().is_empty());
    assert!(!state.editor_ui.font_picker.open);
    assert!(!state.editor_ui.icon_picker.open);
    assert!(!state.editor_ui.component_browser_open);
    assert!(state.editor_ui.ime_preedit.is_none());
    assert!(state.ui.color_picker.is_none());
    assert!(!host.input_active(), "no covered input keeps IME ownership");
}

#[test]
fn import_menu_choice_uses_the_shortcut_focus_cleanup() {
    use op_editor_ui::widgets::ImportMenu;

    let (vw, vh) = (1200.0, 800.0);
    let mut host = WidgetHostNative::new();
    {
        let state = host.editor_state_mut();
        state.editor_ui.import_menu_open = true;
        state.editor_ui.import_menu.open = true;
        state.chat.focused = true;
        state.editor_ui.chat_model_picker.open = true;
    }
    let (anchor, viewport) = host.import_menu_anchor(vw, vh);
    let menu = ImportMenu::for_editor_ui(&host.editor_state().editor_ui);
    let panel = menu.popup_rect(anchor, viewport);
    let point = op_editor_ui::Point2D::new(
        panel.origin.x + panel.size.x / 2.0,
        panel.origin.y + menu.row_height() / 2.0,
    );

    assert!(host.apply_press(point.x, point.y, vw, vh));

    let state = host.editor_state();
    assert!(state.editor_ui.figma_import_open);
    assert!(!state.chat.focused);
    assert!(!state.editor_ui.chat_model_picker.open);
}

#[test]
fn import_shortcut_is_inert_while_an_import_is_in_progress() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.import_source = ImportSource::Figma;
    host.editor_state_mut().editor_ui.figma_import_in_progress = true;
    host.editor_state_mut().chat.focused = true;
    host.editor_state_mut().editor_ui.chat_model_picker.open = true;
    host.editor_state_dirty = false;

    assert!(host.apply_open_html_import(), "the chord stays consumed");

    let state = host.editor_state();
    assert_eq!(state.editor_ui.import_source, ImportSource::Figma);
    assert!(!state.editor_ui.figma_import_open);
    assert!(
        state.chat.focused,
        "rejected shortcut has no blur side effect"
    );
    assert!(state.editor_ui.chat_model_picker.open);
    assert!(!host.editor_state_dirty);
}

#[test]
fn create_component_shortcut_promotes_selection() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, ONE_FRAME);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("f1"));

    assert!(host.apply_create_component());
    assert_eq!(host.editor_state().components.len(), 1);

    // No selection → no-op.
    host.editor_state_mut().clear_selection();
    assert!(!host.apply_create_component());
}

#[test]
fn space_pan_press_pans_canvas_regardless_of_tool() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, ONE_FRAME);
    let before = host.editor_state().viewport.pan_x;

    host.set_space_pan(true);
    // Press empty canvas (Select tool active) — space-pan must start
    // a pan drag instead of a marquee.
    host.apply_press(700.0, 400.0, 1200.0, 800.0);
    host.apply_cursor_move(750.0, 400.0);
    let _ = host.apply_release_with_viewport(1200.0, 800.0);
    host.set_space_pan(false);

    let after = host.editor_state().viewport.pan_x;
    assert!(
        (after - before).abs() > 25.0,
        "space-drag must pan the viewport (delta {})",
        after - before
    );
}

#[test]
fn paste_figma_nodes_centres_fresh_ids_and_selects() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, ONE_FRAME);
    let incoming = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[
            {"type":"rectangle","id":"r1","name":"A","x":0,"y":0,"width":100,"height":100},
            {"type":"rectangle","id":"r2","name":"B","x":100,"y":0,"width":100,"height":100}
        ]}"#,
    )
    .expect("fixture parses")
    .value
    .children;

    // Capture the canvas centre BEFORE the paste — the new selection
    // reveals the property panel, which narrows the canvas region.
    let (_cx0, _cy0, cw, ch) = host.canvas_region(1200.0, 800.0);
    let expected = host
        .editor_state()
        .viewport
        .to_document(op_editor_ui::Point2D::new(cw / 2.0, ch / 2.0))
        .x as f64;

    assert!(host.paste_figma_nodes(incoming, 1200.0, 800.0));

    let state = host.editor_state();
    // Both roots landed with fresh ids (originals r1/r2 not reused
    // since they don't collide here — fresh mint always renames).
    assert_eq!(state.selection.set.len(), 2, "both pasted roots selected");
    assert!(state.history.can_undo(), "paste is one undoable batch");
    // The 200x100 union is centred on the viewport centre.
    let ids: Vec<_> = state.selection.set.clone();
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    for id in &ids {
        let node = op_editor_core::walkers::find_node(state.active_children(), id)
            .expect("pasted node present");
        let b = op_editor_core::own_bounds(node);
        min_x = min_x.min(b.x);
        max_x = max_x.max(b.x + b.w);
    }
    let centre_x = (min_x + max_x) / 2.0;
    assert!(
        (centre_x - expected).abs() < 60.0,
        "pasted union roughly centres on the viewport (got {centre_x}, expected ~{expected})"
    );
}

#[test]
fn paste_figma_nodes_recomputes_missing_fonts() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, ONE_FRAME);
    host.editor_state_mut().editor_ui.system_fonts_loaded = true;
    host.editor_state_mut().editor_ui.system_font_families =
        std::sync::Arc::new(vec!["Arial".to_string()]);
    let incoming = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[
            {"type":"text","id":"label","name":"Label","x":0,"y":0,"width":100,"height":24,
             "content":"Pasted from Figma","fontFamily":"__MissingFigmaClipboardFont__"}
        ]}"#,
    )
    .expect("fixture parses")
    .value
    .children;

    assert!(host.paste_figma_nodes(incoming, 1200.0, 800.0));

    let ui = &host.editor_state().editor_ui;
    assert!(ui.missing_fonts_modal_open);
    assert_eq!(
        ui.missing_fonts_prompt
            .as_ref()
            .and_then(|prompt| prompt.entries.first())
            .map(|entry| entry.family.as_str()),
        Some("__MissingFigmaClipboardFont__")
    );
}

#[test]
fn cursor_move_tracks_canvas_hover_node() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, ONE_RECT);
    // Hover reads the CURRENT scene without refreshing — build it.
    let _ = host.layout_scene();
    // Cursor-move derives the canvas region from the CACHED viewport
    // dims (normally written by apply_press) — seed them directly.
    host.last_viewport_w = 1200.0;
    host.last_viewport_h = 800.0;

    let (cx0, cy0) =
        op_editor_ui::widgets::host_canvas_geometry::canvas_origin(host.editor_state());
    // Over the rect at doc (400, 300) — clear of the floating
    // toolbar column (over_topmost suppresses canvas hover).
    assert!(host.apply_cursor_move(cx0 + 450.0, cy0 + 325.0));
    assert_eq!(
        host.editor_state().editor_ui.canvas_hover_node,
        Some(NodeId::new("n1")),
        "hovering the node sets the canvas hover id"
    );
    // Empty canvas clears it.
    assert!(host.apply_cursor_move(cx0 + 700.0, cy0 + 600.0));
    assert_eq!(host.editor_state().editor_ui.canvas_hover_node, None);
}
