use super::*;
use crate::clipboard::ClipboardImage;
use crate::keyboard_input::ClipboardPayload;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::SizingBehavior;
use op_editor_core::agent_settings::{AcpAgentField, SettingsFocus};
use op_editor_core::PenNodeExt;

const FIGMA_HTML: &str =
    "<html><!--(figmeta)-->eyJ2IjoxfQ==<!--(figmeta)--><!--(figma)-->T1A=<!--(figma)--></html>";

fn image(width: u32, height: u32) -> ClipboardImage {
    ClipboardImage {
        png: vec![1, 2, 3],
        width,
        height,
    }
}

fn payload(
    text: Option<&str>,
    html: Option<&str>,
    image: Option<ClipboardImage>,
) -> ClipboardPayload {
    ClipboardPayload {
        text: text.map(str::to_string),
        html: html.map(str::to_string),
        image,
    }
}

fn seed_internal_clipboard(app: &mut DesktopApp) {
    let state = app.host.editor_state_mut();
    state.set_single_selection(op_editor_core::NodeId::new("n10"));
    assert!(state.copy_selected());
    state.clear_selection();
}

fn focus_settings_input(app: &mut DesktopApp) {
    let ui = &mut app.host.editor_state_mut().editor_ui;
    ui.agent_settings_open = true;
    ui.agent_settings.add_acp_agent();
    ui.agent_settings.focus = Some(SettingsFocus::AcpAgent {
        index: 0,
        field: AcpAgentField::Command,
    });
    ui.settings_input.set_text("");
}

#[test]
fn canvas_image_paste_preserves_size_selects_and_beats_internal_nodes() {
    let mut app = DesktopApp::new(None);
    seed_internal_clipboard(&mut app);
    let state = app.host.editor_state_mut();
    state.set_single_selection(op_editor_core::NodeId::new("n10"));
    state.viewport = op_editor_core::Viewport {
        pan_x: 180.0,
        pan_y: -90.0,
        zoom: 1.25,
    };
    let expected_centre = op_editor_ui::widgets::host_canvas_geometry::canvas_centre_doc_point(
        app.host.editor_state(),
        app.viewport_width,
        app.viewport_height,
    );

    assert!(app.handle_paste_payload(payload(None, None, Some(image(400, 200)))));

    let state = app.host.editor_state();
    assert_eq!(state.active_children().len(), 2);
    let id = state.selection.anchor.clone();
    let PenNode::Image(node) = op_editor_core::walkers::find_node(state.active_children(), &id)
        .expect("selected pasted node exists")
    else {
        panic!("selected pasted node should be an Image");
    };
    assert_eq!(node.width, Some(SizingBehavior::Number(300.0)));
    assert_eq!(node.height, Some(SizingBehavior::Number(150.0)));
    assert_eq!(
        node.base.x.map(|x| x + 150.0),
        Some(expected_centre.x as f64)
    );
    assert_eq!(
        node.base.y.map(|y| y + 75.0),
        Some(expected_centre.y as f64)
    );
    assert_eq!(node.src, "data:image/png;base64,AQID");
    assert!(state.history.can_undo());

    assert!(app.host.editor_state_mut().undo());
    assert_eq!(app.host.editor_state().active_children().len(), 1);
}

#[test]
fn focused_non_chat_input_pastes_text_before_every_canvas_flavour() {
    let mut app = DesktopApp::new(None);
    seed_internal_clipboard(&mut app);
    focus_settings_input(&mut app);

    assert!(app.handle_paste_payload(payload(
        Some("codex\n"),
        Some(FIGMA_HTML),
        Some(image(400, 200)),
    )));

    assert_eq!(
        app.host.editor_state().editor_ui.settings_input.text(),
        "codex"
    );
    assert_eq!(app.host.editor_state().active_children().len(), 1);
    assert!(app.host.editor_state().chat.pending_attachments.is_empty());
    assert!(app.pending_figma_paste.is_none());
}

#[test]
fn focused_chat_prefers_image_attachment_then_falls_back_to_text() {
    let mut app = DesktopApp::new(None);
    seed_internal_clipboard(&mut app);
    app.host.editor_state_mut().chat.focused = true;

    assert!(app.handle_paste_payload(payload(
        Some("ignored text"),
        Some(FIGMA_HTML),
        Some(image(640, 480)),
    )));

    let state = app.host.editor_state();
    assert!(state.chat.input.text().is_empty());
    assert_eq!(state.chat.pending_attachments.len(), 1);
    assert_eq!(state.chat.pending_attachments[0].data, vec![1, 2, 3]);
    assert_eq!(state.active_children().len(), 1);
    assert!(app.pending_figma_paste.is_none());

    app.host.editor_state_mut().chat.pending_attachments.clear();
    assert!(app.handle_paste_payload(payload(Some("chat text"), None, None)));
    assert_eq!(app.host.editor_state().chat.input.text(), "chat text");
    assert!(app.host.editor_state().chat.pending_attachments.is_empty());
}

#[test]
fn chat_model_picker_paste_owns_keyboard_over_stale_chat_focus() {
    let mut app = DesktopApp::new(None);
    seed_internal_clipboard(&mut app);
    let state = app.host.editor_state_mut();
    state.editor_ui.chat_model_picker.open = true;
    state.chat.focused = true;

    assert!(app.handle_paste_payload(payload(Some("gp"), Some(FIGMA_HTML), Some(image(640, 480)),)));

    let state = app.host.editor_state();
    assert_eq!(state.editor_ui.chat_model_picker_input.text(), "gp");
    assert!(state.chat.input.text().is_empty());
    assert!(state.chat.pending_attachments.is_empty());
    assert_eq!(state.active_children().len(), 1);
    assert!(state.selection.is_empty());
    assert_eq!(state.clipboard.len(), 1);
    assert!(app.pending_figma_paste.is_none());
}

#[test]
fn font_picker_paste_owns_keyboard_over_stale_chat_focus() {
    let mut app = DesktopApp::new(None);
    seed_internal_clipboard(&mut app);
    let state = app.host.editor_state_mut();
    state.editor_ui.font_picker.open = true;
    state.chat.focused = true;

    assert!(app.handle_paste_payload(payload(
        Some("Inter"),
        Some(FIGMA_HTML),
        Some(image(640, 480)),
    )));

    let state = app.host.editor_state();
    assert_eq!(state.editor_ui.font_picker_search, "Inter");
    assert!(state.chat.input.text().is_empty());
    assert!(state.chat.pending_attachments.is_empty());
    assert_eq!(state.active_children().len(), 1);
    assert!(state.selection.is_empty());
    assert_eq!(state.clipboard.len(), 1);
    assert!(app.pending_figma_paste.is_none());
}

#[test]
fn image_search_paste_owns_keyboard_over_stale_chat_focus() {
    let mut app = DesktopApp::new(None);
    seed_internal_clipboard(&mut app);
    let state = app.host.editor_state_mut();
    state.editor_ui.image_panel.search_open = true;
    state.chat.focused = true;

    assert!(app.handle_paste_payload(payload(
        Some("sunset"),
        Some(FIGMA_HTML),
        Some(image(640, 480)),
    )));

    let state = app.host.editor_state();
    assert_eq!(state.editor_ui.image_panel.search_query.text(), "sunset");
    assert!(state.chat.input.text().is_empty());
    assert!(state.chat.pending_attachments.is_empty());
    assert_eq!(state.active_children().len(), 1);
    assert!(state.selection.is_empty());
    assert_eq!(state.clipboard.len(), 1);
    assert!(app.pending_figma_paste.is_none());
}

#[test]
fn image_search_keyboard_does_not_leak_into_canvas_shortcuts() {
    let mut app = DesktopApp::new(None);
    let state = app.host.editor_state_mut();
    let _ = state.insert_image_node_at_viewport("Hero photo", "https://x/y.png");
    state.editor_ui.image_panel.search_open = true;
    state.editor_ui.image_panel.search_query.set_text("abcd");
    state.editor_ui.image_panel.search_query.set_caret(2, 0);

    let tool_before = app.host.editor_state().tool;
    let selected = app.host.editor_state().selection.anchor.clone();
    let image_before = app
        .host
        .editor_state()
        .selected_node()
        .expect("selected image")
        .base()
        .clone();

    app.handle_key_pressed(&Key::Character("p".into()), Some("p"));
    assert_eq!(app.host.editor_state().tool, tool_before);
    assert_eq!(
        app.host
            .editor_state()
            .editor_ui
            .image_panel
            .search_query
            .text(),
        "abpcd"
    );

    app.handle_key_pressed(&Key::Named(NamedKey::ArrowUp), None);
    let image_after_arrow = app
        .host
        .editor_state()
        .selected_node()
        .expect("image survives arrow")
        .base();
    assert_eq!(image_after_arrow.x, image_before.x);
    assert_eq!(image_after_arrow.y, image_before.y);

    app.handle_key_pressed(&Key::Named(NamedKey::Delete), None);
    assert_eq!(app.host.editor_state().selection.anchor, selected);
    assert_eq!(
        app.host
            .editor_state()
            .editor_ui
            .image_panel
            .search_query
            .text(),
        "abpd"
    );

    app.handle_key_pressed(&Key::Character("[".into()), Some("["));
    assert_eq!(
        app.host
            .editor_state()
            .editor_ui
            .image_panel
            .search_query
            .text(),
        "abp[d"
    );

    app.zoom_modifier = true;
    app.handle_key_pressed(&Key::Character("a".into()), Some("a"));
    assert_eq!(
        app.host
            .editor_state()
            .editor_ui
            .image_panel
            .search_query
            .highlight_range(),
        Some((0, 5))
    );
    assert_eq!(app.host.editor_state().selection.anchor, selected);

    let chat_was_focused = app.host.editor_state().chat.focused;
    app.handle_key_pressed(&Key::Character("j".into()), Some("j"));
    assert_eq!(app.host.editor_state().chat.focused, chat_was_focused);

    app.handle_key_pressed(&Key::Character(",".into()), Some(","));
    assert!(app.host.editor_state().editor_ui.agent_settings_open);
    assert!(!app.host.editor_state().editor_ui.image_panel.search_open);
    assert!(!app.host.editor_state().editor_ui.image_panel.generate_open);
}

#[test]
fn image_search_home_end_and_modified_arrows_move_or_extend_selection() {
    let mut app = DesktopApp::new(None);
    let panel = &mut app.host.editor_state_mut().editor_ui.image_panel;
    panel.search_open = true;
    panel.search_query.set_text("a你bc");
    panel.search_query.set_caret(1, 0);

    app.handle_key_pressed(&Key::Named(NamedKey::End), None);
    assert_eq!(
        app.host
            .editor_state()
            .editor_ui
            .image_panel
            .search_query
            .caret(),
        "a你bc".len()
    );

    app.shift_modifier = true;
    app.handle_key_pressed(&Key::Named(NamedKey::Home), None);
    assert_eq!(
        app.host
            .editor_state()
            .editor_ui
            .image_panel
            .search_query
            .highlight_range(),
        Some((0, "a你bc".len()))
    );

    app.shift_modifier = false;
    app.zoom_modifier = true;
    app.handle_key_pressed(&Key::Named(NamedKey::ArrowRight), None);
    assert_eq!(
        app.host
            .editor_state()
            .editor_ui
            .image_panel
            .search_query
            .caret(),
        "a你bc".len()
    );
    app.handle_key_pressed(&Key::Named(NamedKey::ArrowLeft), None);
    assert_eq!(
        app.host
            .editor_state()
            .editor_ui
            .image_panel
            .search_query
            .caret(),
        0
    );
}

#[test]
fn hidden_git_focus_does_not_swallow_canvas_image_paste() {
    let mut app = DesktopApp::new(None);
    seed_internal_clipboard(&mut app);
    app.host.editor_state_mut().viewport = op_editor_core::Viewport::IDENTITY;
    let git = &mut app.host.editor_state_mut().editor_ui.git_panel;
    assert!(!git.open);
    git.commit_focused = true;

    assert!(app.handle_paste_payload(payload(Some("stale git text"), None, Some(image(400, 200)),)));

    let state = app.host.editor_state();
    assert!(state.editor_ui.git_panel.commit_input.text().is_empty());
    assert_eq!(state.active_children().len(), 2);
    assert!(matches!(state.selected_node(), Some(PenNode::Image(_))));
    assert_eq!(state.clipboard.len(), 1);
}

#[test]
fn figma_html_beats_canvas_image_and_internal_nodes() {
    let mut app = DesktopApp::new(None);
    seed_internal_clipboard(&mut app);

    assert!(app.handle_paste_payload(payload(None, Some(FIGMA_HTML), Some(image(400, 200)),)));

    assert!(app.pending_figma_paste.is_some());
    assert_eq!(app.host.editor_state().active_children().len(), 1);
    assert!(app.host.editor_state().selection.is_empty());
}

#[test]
fn internal_node_clipboard_remains_the_canvas_fallback() {
    let mut app = DesktopApp::new(None);
    seed_internal_clipboard(&mut app);

    assert!(app.handle_paste_payload(ClipboardPayload::default()));

    assert_eq!(app.host.editor_state().active_children().len(), 2);
    assert!(app.host.editor_state().selection.anchor.is_real());
    assert!(!matches!(
        app.host.editor_state().selected_node(),
        Some(PenNode::Image(_))
    ));
}

#[test]
fn stop_chat_aborts_sub_agents_and_clears_the_running_counter() {
    let _guard = crate::agent_indicator_test_lock::LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    op_editor_core::agent_indicators::clear();

    let mut app = DesktopApp::new(None);
    let (_hold_tx, rx) = std::sync::mpsc::channel();
    app.sub_agents
        .push(crate::sub_agent_session::SubAgentSession {
            session: Some(crate::chat_session::ChatSession::from_channels(rx, None)),
            identity: op_orchestrator::agent_identity::AgentIdentity {
                color: "#5B8DEF".into(),
                name: "Fern".into(),
            },
            indicator: None,
            root_seed_mobile: true,
            root_seed_continuation: false,
        });
    app.active_sub_agent = 0;
    app.host.editor_state_mut().chat.agents_running = (1, 1);
    app.host
        .editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant_streaming());

    assert!(app.host.editor_state_mut().chat.stop_streaming());
    assert!(app.drain_stop_chat());

    assert!(app.sub_agents.is_empty());
    assert_eq!(app.active_sub_agent, 0);
    assert_eq!(app.host.editor_state().chat.agents_running, (0, 0));
    assert!(
        app.host
            .editor_state()
            .chat
            .messages
            .iter()
            .all(|message| !message.streaming),
        "Stop must leave no hidden streaming bubble behind"
    );
    op_editor_core::agent_indicators::clear();
}

#[test]
fn new_chat_aborts_the_old_running_tab_without_dirtying_the_fresh_tab() {
    let _guard = crate::agent_indicator_test_lock::LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    op_editor_core::agent_indicators::clear();

    let mut app = DesktopApp::new(None);
    let (_hold_tx, rx) = std::sync::mpsc::channel();
    app.sub_agents
        .push(crate::sub_agent_session::SubAgentSession {
            session: Some(crate::chat_session::ChatSession::from_channels(rx, None)),
            identity: op_orchestrator::agent_identity::AgentIdentity {
                color: "#5B8DEF".into(),
                name: "Fern".into(),
            },
            indicator: None,
            root_seed_mobile: true,
            root_seed_continuation: false,
        });
    app.active_sub_agent = 0;
    app.chat_running_tab = Some(0);
    {
        let old_tab = app.host.editor_state_mut().chat.tab_mut(0).unwrap();
        old_tab.agents_running = (1, 1);
        old_tab
            .messages
            .push(op_editor_core::ChatMessage::assistant_streaming());
    }

    let fresh = app.host.editor_state_mut().chat.new_tab();
    app.host.editor_state_mut().chat.pending_new_chat = true;
    assert_eq!(fresh, 1);
    assert!(app.drain_new_chat());

    assert!(app.sub_agents.is_empty());
    assert_eq!(app.active_sub_agent, 0);
    assert_eq!(app.chat_running_tab, None);
    let tabs = app.host.editor_state().chat.tabs();
    assert_eq!(tabs[0].agents_running, (0, 0));
    assert!(tabs[0].messages.iter().all(|message| !message.streaming));
    assert_eq!(tabs[1].agents_running, (0, 0));
    assert!(tabs[1].messages.is_empty());
    op_editor_core::agent_indicators::clear();
}

#[test]
fn html_paste_guard() {
    assert!(!crate::keyboard_input::html_paste_should_consume("   \n"));
    assert!(crate::keyboard_input::html_paste_should_consume(
        "<div>x</div>"
    ));
}

// ── presenter keys while a deck is presenting ─────────────────────────────

/// Three 16:9 boards, tagged as a deck — what the presenting path expects.
const PRESENTING_DECK: &str = r##"{
    "version": "1.0.0",
    "children": [
        { "type": "frame", "id": "slide-1", "x": 0, "y": 0,
          "width": 1920, "height": 1080,
          "fill": [{"type":"solid","color":"#ffffff"}], "children": [] },
        { "type": "frame", "id": "slide-2", "x": 2100, "y": 0,
          "width": 1920, "height": 1080,
          "fill": [{"type":"solid","color":"#eeeeee"}], "children": [] },
        { "type": "frame", "id": "slide-3", "x": 4200, "y": 0,
          "width": 1920, "height": 1080,
          "fill": [{"type":"solid","color":"#dddddd"}], "children": [] }
    ]
}"##;

fn presenting_app() -> DesktopApp {
    let document = jian_ops_schema::load_str(PRESENTING_DECK)
        .expect("parse deck fixture")
        .value;
    let mut app = DesktopApp::new(None);
    let mut state = op_editor_core::EditorState::from_document(document);
    state.editor_ui.scenario = Some(op_editor_core::scene_template_catalog::TemplateScene::Slides);
    app.host.install_imported_state(state);
    app.host.enter_preview((1200.0, 800.0));
    assert!(app.host.preview_slideshow_active(), "fixture presents");
    app
}

fn board(app: &DesktopApp) -> String {
    app.host
        .editor_state()
        .preview_slideshow()
        .and_then(|slideshow| slideshow.current_board())
        .expect("presenting")
        .to_string()
}

fn press(app: &mut DesktopApp, key: NamedKey) {
    app.handle_key_pressed(&Key::Named(key), None);
}

/// Enter, Backspace, Home and End all have an established editor or chat
/// meaning, and each is matched EARLIER in the key ladder than the arms
/// added here. This is the test that the presenting arms really do come
/// first — reordering them silently sends Backspace back to node deletion
/// mid-presentation.
#[test]
fn presenting_claims_enter_backspace_home_and_end_from_the_editor() {
    let mut app = presenting_app();

    press(&mut app, NamedKey::Enter);
    assert_eq!(board(&app), "slide-2");
    press(&mut app, NamedKey::Backspace);
    assert_eq!(board(&app), "slide-1");
    assert_eq!(
        app.host.editor_state().active_children().len(),
        3,
        "Backspace must not have deleted a board"
    );

    press(&mut app, NamedKey::End);
    assert_eq!(board(&app), "slide-3");
    press(&mut app, NamedKey::Home);
    assert_eq!(board(&app), "slide-1");

    // Space and Page Down stay on the deck too, and never start a space-pan.
    press(&mut app, NamedKey::Space);
    assert_eq!(board(&app), "slide-2");
    press(&mut app, NamedKey::PageDown);
    assert_eq!(board(&app), "slide-3");
    press(&mut app, NamedKey::PageUp);
    assert_eq!(board(&app), "slide-2");
}

/// The same keys keep their editor meaning when nothing is presenting.
#[test]
fn the_editor_keeps_those_keys_when_no_deck_is_presenting() {
    let mut app = DesktopApp::new(None);
    let state = app.host.editor_state_mut();
    state.set_single_selection(op_editor_core::NodeId::new("n10"));
    assert_eq!(state.active_children().len(), 1);

    press(&mut app, NamedKey::Backspace);

    assert!(
        app.host.editor_state().active_children().is_empty(),
        "Backspace still deletes the selection outside a presentation"
    );
}

/// The Delete key must forward-delete in a focused settings input and
/// never delete the canvas selection behind the modal. Before the fix
/// the dispatcher dropped the key entirely for settings fields, so
/// select-all + Delete could not clear a saved API key.
#[test]
fn delete_key_edits_focused_settings_input_and_keeps_nodes() {
    let mut app = DesktopApp::new(None);
    let node_count = app.host.editor_state().active_children().len();
    app.host
        .editor_state_mut()
        .set_single_selection(op_editor_core::NodeId::new("n10"));
    focus_settings_input(&mut app);
    {
        let ui = &mut app.host.editor_state_mut().editor_ui;
        ui.settings_input.set_text("op-agent");
        ui.settings_input.set_caret(0, 0);
    }

    app.handle_key_pressed(&Key::Named(NamedKey::Delete), None);
    assert_eq!(
        app.host.editor_state().editor_ui.settings_input.text(),
        "p-agent"
    );
    assert_eq!(
        app.host.editor_state().active_children().len(),
        node_count,
        "Delete in a settings field must never remove canvas nodes"
    );

    // Select-all + Delete clears the whole draft.
    assert!(app.host.apply_select_all());
    app.handle_key_pressed(&Key::Named(NamedKey::Delete), None);
    assert_eq!(app.host.editor_state().editor_ui.settings_input.text(), "");
    assert_eq!(app.host.editor_state().active_children().len(), node_count);
}
