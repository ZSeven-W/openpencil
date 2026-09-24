//! Studio Home on the web host: typing, sending, the discard confirm, and
//! the keyboard / wheel ownership Home claims while it is up.

use super::{HomeReplaceIntent, WidgetHost};
use op_editor_core::scene_template_append::template_boards;
use op_editor_core::scene_template_catalog::scene_template_document;
use op_editor_core::{EntrySurface, HomeFamily, LaunchRoute, Tool};
use op_editor_ui::widgets::HomeSurface;
use op_editor_ui::{Point2D, Rect};

const W: f32 = 1440.0;
const H: f32 = 900.0;

fn center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn home_host() -> WidgetHost {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.home.visible = true;
    host.last_viewport_w = W;
    host.last_viewport_h = H;
    host
}

/// A browser whose daemon serves models (the served-key path).
fn served_home_host() -> WidgetHost {
    let mut host = home_host();
    host.editor_state.editor_ui.agent_settings.web_served_models = true;
    host
}

fn layout(host: &WidgetHost) -> op_editor_ui::widgets::HomeLayout {
    HomeSurface::for_editor(&host.editor_state)
        .expect("home visible")
        .layout(W, H)
}

fn type_brief(host: &mut WidgetHost, text: &str) {
    for c in text.chars() {
        assert!(host.apply_text(c));
    }
}

/// Put real work on the page: the slide-deck template's boards.
fn page_with_work(host: &mut WidgetHost) {
    let source = scene_template_document("slide-deck").expect("embedded on native");
    let boards = template_boards(source, "slide-deck").expect("boards");
    assert!(host.editor_state.adopt_template_boards(boards));
    assert!(!op_editor_core::blank_starter::active_page_is_blank_starter(&host.editor_state));
}

#[test]
fn typing_lands_in_the_home_draft_and_never_switches_tools() {
    let mut host = home_host();
    type_brief(&mut host, "rv");
    assert_eq!(host.editor_state.editor_ui.home.draft, "rv");
    assert_eq!(
        host.editor_state.tool,
        Tool::Select,
        "`r` typed, not a tool switch"
    );
    assert!(!host.apply_tool_shortcut("r"));
    assert!(host.apply_backspace());
    assert_eq!(host.editor_state.editor_ui.home.draft, "r");
}

#[test]
fn ime_commits_and_beforeinput_payloads_reach_the_composer() {
    let mut host = home_host();
    let commit = crate::event::ime::composition_end("取餐预约".to_string());
    assert!(host.apply_ime(&commit));
    assert_eq!(host.editor_state.editor_ui.home.draft, "取餐预约");
    assert!(host.ime_anchor_rect().is_some());
    assert!(host.text_input_focus_active());
}

#[test]
fn enter_on_a_typed_brief_opens_the_workspace_and_queues_a_pinned_run() {
    let mut host = served_home_host();
    type_brief(&mut host, "coffee pickup app");
    let expected = host
        .editor_state
        .editor_ui
        .home
        .generation_prompt()
        .expect("prompt");
    assert!(host.apply_send());
    assert!(!host.home_visible());
    let ui = &host.editor_state.editor_ui;
    assert!(ui.workspace.active && ui.workspace.visible);
    assert_eq!(host.editor_state.tool, Tool::Hand);
    assert_eq!(
        host.editor_state.chat.pending_send.as_deref(),
        Some(expected.as_str())
    );
    assert_eq!(
        host.editor_state.chat.launch_route,
        LaunchRoute::Orchestrator
    );
    assert!(host.workspace_run_generating());
    assert!(
        !host.take_daemon_file_unbind(),
        "the untouched starter is reused, so the daemon's binding is left alone"
    );
}

#[test]
fn send_on_an_empty_box_without_any_model_opens_the_connect_card() {
    let mut host = home_host();
    let send = center(layout(&host).send);
    assert!(host.apply_press(send.x, send.y, W, H));
    assert!(host.editor_state.editor_ui.home.connect_card_open);
    assert!(host.editor_state.chat.pending_send.is_none());
    assert!(host.apply_escape(), "Escape closes the connect card first");
    assert!(!host.editor_state.editor_ui.home.connect_card_open);
}

#[test]
fn daemon_served_models_count_as_a_usable_agent() {
    let host = served_home_host();
    assert!(host.editor_state.has_usable_chat_agent());
}

#[test]
fn send_on_an_empty_box_opens_the_example_as_an_instant_draft() {
    let mut host = served_home_host();
    host.editor_state
        .editor_ui
        .home
        .set_task(HomeFamily::Presentation, 0);
    let send = center(layout(&host).send);
    assert!(host.apply_press(send.x, send.y, W, H));
    let workspace = &host.editor_state.editor_ui.workspace;
    assert!(workspace.active, "the draft opens the workspace");
    assert!(workspace.draft_template.is_some());
    assert!(
        op_editor_core::preview_slideshow::active_page_boards(&host.editor_state).len() > 1,
        "the template's boards are on the page"
    );
    assert_eq!(host.editor_state.chat.launch_route, LaunchRoute::Refine);
    assert!(
        host.editor_state.chat.pending_send.is_some(),
        "refine queued"
    );
}

#[test]
fn a_send_over_unsaved_work_waits_for_the_discard_confirm() {
    let mut host = served_home_host();
    page_with_work(&mut host);
    assert!(host.editor_state.is_dirty());
    type_brief(&mut host, "event poster");
    assert!(!host.apply_send());
    assert!(host.home_visible(), "nothing happened yet");
    assert!(host.editor_state.chat.pending_send.is_none());
    let intent = host.take_home_replace_confirm().expect("parked");
    assert_eq!(intent, HomeReplaceIntent::Brief);

    assert!(!host.take_daemon_file_unbind(), "nothing swapped yet");
    // Confirmed: the work is replaced by a fresh starter and the run starts.
    assert!(host.confirm_home_replace(intent));
    assert!(host.take_daemon_file_unbind(), "the fresh page is untitled");
    assert!(op_editor_core::blank_starter::active_page_is_blank_starter(
        &host.editor_state
    ));
    assert!(host.editor_state.chat.pending_send.is_some());
    assert!(host.editor_state.editor_ui.workspace.active);
    assert!(!host.home_visible());
}

#[test]
fn cancelling_the_confirm_leaves_home_and_the_document_alone() {
    let mut host = served_home_host();
    page_with_work(&mut host);
    type_brief(&mut host, "event poster");
    host.apply_send();
    // The DOM layer took the intent and the user said no: nothing else runs.
    assert!(host.take_home_replace_confirm().is_some());
    assert!(host.take_home_replace_confirm().is_none());
    assert!(!host.take_daemon_file_unbind(), "the document stays bound");
    assert!(host.home_visible());
    assert_eq!(host.editor_state.editor_ui.home.draft, "event poster");
    assert!(!op_editor_core::blank_starter::active_page_is_blank_starter(&host.editor_state));
}

#[test]
fn saved_work_is_swapped_for_a_fresh_page_without_asking() {
    let mut host = served_home_host();
    page_with_work(&mut host);
    host.editor_state.mark_saved_revision();
    host.editor_state.chat.title = "previous run".into();
    type_brief(&mut host, "event poster");
    assert!(host.apply_send());
    assert!(host.take_home_replace_confirm().is_none());
    assert!(op_editor_core::blank_starter::active_page_is_blank_starter(
        &host.editor_state
    ));
    assert_ne!(
        host.editor_state.chat.title, "previous run",
        "transcript restarted"
    );
    assert!(host.editor_state.chat.pending_send.is_some());
    // Save must now download the fresh page, not overwrite the bound file.
    assert!(host.take_daemon_file_unbind());
    assert!(!host.take_daemon_file_unbind(), "taken once");
}

#[test]
fn professional_leaves_home_for_the_canvas_and_remembers_it() {
    let mut host = home_host();
    let professional = layout(&host).professional;
    let at = Point2D::new(professional.origin.x + 4.0, professional.origin.y + 4.0);
    assert!(host.apply_press(at.x, at.y, W, H));
    assert!(!host.home_visible());
    assert_eq!(
        host.editor_state.editor_ui.entry_surface,
        EntrySurface::Canvas
    );
}

#[test]
fn home_swallows_presses_the_canvas_would_otherwise_take() {
    let mut host = home_host();
    // The bottom-right corner is blank Home page, not the canvas.
    assert!(host.apply_press(W - 4.0, H - 4.0, W, H));
    assert!(host.home_visible());
    assert!(host.drag.is_none() && host.marquee_drag.is_none());
}

#[test]
fn wheel_scrolls_the_home_page_and_never_zooms_the_canvas() {
    let mut host = home_host();
    let zoom = host.editor_state.viewport.zoom;
    host.apply_wheel_with_canvas_delta(W / 2.0, H / 2.0, -300.0, -30.0, W, H);
    assert!(host.editor_state.editor_ui.home.scroll_y > 0.0);
    assert_eq!(host.editor_state.viewport.zoom, zoom);
}

#[test]
fn opening_a_document_leaves_home_and_a_stale_workspace() {
    let mut host = served_home_host();
    type_brief(&mut host, "coffee pickup app");
    assert!(host.apply_send());
    host.editor_state.editor_ui.home.visible = true;
    host.install_ingested_state(op_editor_core::EditorState::starter());
    assert!(!host.home_visible());
    assert!(!host.editor_state.editor_ui.workspace.active);
}

#[test]
fn the_pinned_route_travels_with_the_turn_and_is_consumed_by_it() {
    let mut host = served_home_host();
    type_brief(&mut host, "coffee pickup app");
    assert!(host.apply_send());
    let prepared = crate::web_chat::prepare_turn(&mut host.editor_state).expect("turn");
    let body: serde_json::Value = serde_json::from_str(&prepared.body_json).expect("json");
    assert_eq!(body["launchRoute"], "orchestrator");
    assert_eq!(host.editor_state.chat.launch_route, LaunchRoute::Auto);

    host.editor_state.chat.set_input_text("hello".to_string());
    assert!(host.editor_state.chat.begin_send());
    let prepared = crate::web_chat::prepare_turn(&mut host.editor_state).expect("turn");
    let body: serde_json::Value = serde_json::from_str(&prepared.body_json).expect("json");
    assert!(
        body["launchRoute"].is_null(),
        "an ordinary turn pins nothing"
    );
}

#[test]
fn a_daemon_catalog_marks_served_models_and_an_empty_one_clears_it() {
    let mut state = op_editor_core::EditorState::new();
    crate::web_model_catalog::apply_models(&mut state, &["gpt-5".to_string()]);
    assert!(state.editor_ui.agent_settings.web_served_models);
    crate::web_model_catalog::apply_models(&mut state, &[]);
    assert!(!state.editor_ui.agent_settings.web_served_models);
}
