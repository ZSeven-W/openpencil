//! The narrow-window chat drawer, driven through the host's press and
//! Escape paths.

use super::WidgetHostNative;
use op_editor_core::{HomeFamily, Tool, WorkspacePhase, WORKSPACE_HEADER_H, WORKSPACE_TOOLBAR_H};

const W: f32 = 1440.0;
const H: f32 = 900.0;
const NARROW_W: f32 = 820.0;

fn deck_host() -> WidgetHostNative {
    let source = r#"{ "version": "1.0.0", "children": [
        { "type": "frame", "id": "b0", "x": 0, "y": 0, "width": 1920, "height": 1080, "children": [] },
        { "type": "frame", "id": "b1", "x": 2000, "y": 0, "width": 1920, "height": 1080, "children": [] }
    ] }"#;
    let document = jian_ops_schema::load_str(source).expect("fixture").value;
    let mut host = WidgetHostNative::new();
    host.install_imported_state(op_editor_core::EditorState::from_document(document));
    host.set_now_ms(2_000);
    let editor = host.editor_state_mut();
    editor.tool = Tool::Hand;
    editor.editor_ui.open_workspace_for_generation(
        HomeFamily::Presentation,
        "deck",
        op_editor_core::TaskDraft::default(),
        0,
        1_000,
        Some(Tool::Select),
    );
    editor.editor_ui.workspace.phase = WorkspacePhase::Done;
    host
}

/// Press the toolbar's chat toggle (its leading icon slot).
fn press_toggle(host: &mut WidgetHostNative, viewport_w: f32) -> Option<bool> {
    let (canvas_x, _, _, _) = host.canvas_region(viewport_w, H);
    host.press_workspace(
        canvas_x + 12.0 + 14.0,
        WORKSPACE_HEADER_H + WORKSPACE_TOOLBAR_H / 2.0,
        viewport_w,
        H,
    )
}

#[test]
fn a_narrow_window_gives_the_canvas_the_width_and_the_chat_a_drawer() {
    let mut host = deck_host();
    host.sync_workspace_drawer(NARROW_W);
    let (x, _, w, _) = host.canvas_region(NARROW_W, H);
    assert_eq!((x, w), (0.0, NARROW_W), "the design gets the full width");
    assert_eq!(
        host.ai_chat_rect(NARROW_W, H),
        None,
        "the drawer starts shut"
    );

    // The toolbar toggle opens the drawer over the canvas.
    assert_eq!(press_toggle(&mut host, NARROW_W), Some(true));
    assert!(host.editor_state().editor_ui.workspace.drawer_open);
    let chat = host.ai_chat_rect(NARROW_W, H).expect("drawer chat");
    assert_eq!(chat.origin.x, 0.0);
    let (x, _, _, _) = host.canvas_region(NARROW_W, H);
    assert_eq!(
        x, 0.0,
        "the drawer overlays the canvas instead of pushing it"
    );
    assert!(
        host.editor_state().editor_ui.sidebar_open,
        "the docked preference is untouched"
    );

    // A press beside the drawer shuts it and does nothing else.
    let zoom_before = host.editor_state().viewport.zoom;
    assert_eq!(
        host.press_workspace(NARROW_W - 20.0, H / 2.0, NARROW_W, H),
        Some(true)
    );
    assert!(!host.editor_state().editor_ui.workspace.drawer_open);
    assert_eq!(host.editor_state().viewport.zoom, zoom_before);

    // Escape shuts it too, and releases the chat input it held.
    host.set_workspace_drawer_open(true);
    host.editor_state_mut().chat.focused = true;
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.workspace.drawer_open);
    assert!(!host.editor_state().chat.focused);
}

#[test]
fn a_press_inside_the_open_drawer_belongs_to_the_chat() {
    let mut host = deck_host();
    host.sync_workspace_drawer(NARROW_W);
    host.set_workspace_drawer_open(true);
    assert_eq!(host.press_workspace(40.0, H / 2.0, NARROW_W, H), None);
    assert!(host.editor_state().editor_ui.workspace.drawer_open);
}

#[test]
fn widening_the_window_restores_the_docked_column() {
    let mut host = deck_host();
    host.sync_workspace_drawer(NARROW_W);
    host.set_workspace_drawer_open(true);
    host.sync_workspace_drawer(W);
    assert!(!host.editor_state().editor_ui.workspace_drawer_active());
    let (x, _, _, _) = host.canvas_region(W, H);
    assert_eq!(x, host.editor_state().editor_ui.layer_panel_width);
    // In a wide window the same toggle collapses the docked column.
    assert_eq!(press_toggle(&mut host, W), Some(true));
    assert!(!host.editor_state().editor_ui.sidebar_open);
}
