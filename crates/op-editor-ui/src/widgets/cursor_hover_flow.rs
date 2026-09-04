//! Cursor-move hover resolution shared by the native and web widget
//! hosts (their `widget_host/input.rs::apply_cursor_move` and
//! `widget_host/cursor_input.rs::apply_cursor_move` god functions
//! carried these blocks as a verbatim copy-paste run).
//!
//! Every entry point returns whether anything visible changed, so each
//! host keeps its own `mark_dirty()` / ownership-cascade tail. The
//! hosts also keep the layer ordering itself: which surface gets first
//! refusal is genuinely platform-specific (native paints Git, the
//! preview runtime, and window controls that wasm32 compiles out).

use op_editor_core::{EditorState, NodeId};

use crate::layout_scene::LayoutScene;
use crate::widgets::{CanvasViewport, PropertyPanel};
use crate::{Point2D, Rect};

/// Resolve Prompt Center hover while treating the entire floating panel
/// as cursor-owned chrome. Padding and grid gutters therefore block
/// hover from leaking into lower panels or the canvas.
pub fn prompt_center_hover(
    state: &mut EditorState,
    panel_rect: Rect,
    point: Point2D,
) -> (bool, bool) {
    if !state.editor_ui.prompt_center.open {
        return (false, false);
    }
    let owns_point = panel_rect.contains(point);
    let next = if owns_point {
        crate::widgets::PromptCenterPanel::for_editor(state)
            .and_then(|panel| panel.hover_at(panel_rect, point))
    } else {
        None
    };
    let changed = state.editor_ui.prompt_center.hover != next;
    state.editor_ui.prompt_center.hover = next;
    (owns_point, changed)
}

// ─── Modal overlays that own the cursor while open ─────────────────────

/// Missing-fonts modal hover: the per-row choose-file buttons plus the
/// dismiss action, or the embedded font picker's entry / import rows
/// once it is laid out over the modal.
pub fn missing_fonts_modal_hover(
    state: &mut EditorState,
    viewport_w: f32,
    viewport_h: f32,
    point: Point2D,
) -> bool {
    use crate::widgets::missing_fonts_panel::MissingFontsPanel;
    let viewport = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(viewport_w, viewport_h),
    };
    let (new_hover, picker_hover, import_hover) = MissingFontsPanel::for_editor(state)
        .map(|panel| {
            let rect = panel.rect(viewport_w, viewport_h);
            if panel.picker_layout(rect, viewport).is_some() {
                let (entry, import) = panel.picker_hover(rect, viewport, point);
                (None, entry, import)
            } else {
                (
                    crate::widgets::editor_state_ext::missing_fonts_button(
                        panel.hit_test(rect, viewport, point),
                    ),
                    None,
                    false,
                )
            }
        })
        .unwrap_or((None, None, false));
    let changed = new_hover != state.editor_ui.missing_fonts_hover
        || picker_hover != state.editor_ui.font_picker.hover
        || import_hover != state.editor_ui.font_picker_import_hover;
    if changed {
        let ui = &mut state.editor_ui;
        ui.missing_fonts_hover = new_hover;
        ui.font_picker.hover = picker_hover;
        ui.font_picker_import_hover = import_hover;
    }
    changed
}

/// Sign-in modal hover: the close `✕` plus the primary sign-in button.
pub fn login_modal_hover(
    state: &mut EditorState,
    viewport_w: f32,
    viewport_h: f32,
    point: Point2D,
) -> bool {
    use crate::widgets::login_modal::LoginModal;
    let modal = LoginModal::for_editor(state);
    let panel = modal.rect(viewport_w, viewport_h);
    let new_hover =
        crate::widgets::editor_state_ext::login_modal_button(modal.hit_test(panel, point));
    let changed = new_hover != state.editor_ui.login_modal_hover;
    if changed {
        state.editor_ui.login_modal_hover = new_hover;
    }
    changed
}

/// Signed-in account dropdown row hover, anchored under the TopBar
/// account button.
pub fn account_menu_hover(state: &mut EditorState, viewport_w: f32, point: Point2D) -> bool {
    use crate::widgets::account_menu::AccountMenu;
    let new_hover = AccountMenu::for_editor_ui(&state.editor_ui).and_then(|menu| {
        let panel =
            crate::widgets::touch_overlay_geometry::account_menu_rect(state, &menu, viewport_w);
        menu.row_at(panel, point)
    });
    let changed = new_hover != state.editor_ui.account_menu_hover;
    if changed {
        state.editor_ui.account_menu_hover = new_hover;
    }
    changed
}

// ─── Context menus ─────────────────────────────────────────────────────

/// Whether the open path-anchor context menu covers `point`. An
/// unchanged row still owns the point, so the caller must not fall
/// through to the surfaces painted behind the menu.
pub fn path_anchor_menu_contains(state: &EditorState, point: Point2D) -> bool {
    state
        .ui
        .path_anchor_menu
        .clone()
        .map(|menu_state| {
            crate::widgets::path_anchor_context_menu::PathAnchorContextMenu::for_state(
                state, menu_state,
            )
            .rect()
            .contains(point)
        })
        .unwrap_or(false)
}

// ─── Right rail ────────────────────────────────────────────────────────

/// PropertyPanel base hover wash (tab strip, fill-type and compositing
/// rows, action buttons). `panel` is `None` when the rail is not
/// eligible for this move, which clears every retained hover instead.
pub fn property_base_hover(
    state: &mut EditorState,
    panel: Option<&PropertyPanel>,
    property_rect: Rect,
    point: Point2D,
) -> bool {
    let mut changed = false;
    if let Some(panel) = panel {
        let new_tab_hover = panel.tab_hover_at(property_rect, point);
        if new_tab_hover != state.editor_ui.property_tab_hover {
            state.editor_ui.property_tab_hover = new_tab_hover;
            changed = true;
        }
        let new_fill_type_hover = panel.fill_type_picker_row_at(property_rect, point);
        if new_fill_type_hover != state.editor_ui.fill_type_picker.hover {
            state.editor_ui.fill_type_picker.hover = new_fill_type_hover;
            changed = true;
        }
        let new_compositing_hover = panel.compositing_picker_row_at(property_rect, point);
        if new_compositing_hover != state.editor_ui.compositing_picker.hover {
            state.editor_ui.compositing_picker.hover = new_compositing_hover;
            changed = true;
        }
        let new_action_hover = panel.action_hover_index(property_rect, point);
        if new_action_hover != state.editor_ui.property_action_hover {
            state.editor_ui.property_action_hover = new_action_hover;
            changed = true;
        }
    } else {
        let ui = &mut state.editor_ui;
        changed |= ui.property_tab_hover.take().is_some();
        changed |= ui.fill_type_picker.hover.take().is_some();
        changed |= ui.compositing_picker.hover.take().is_some();
        changed |= ui.property_action_hover.take().is_some();
    }
    changed
}

/// Fill / stroke colour-variable popup hover. Returns
/// `(over_popup, changed)`: the popup owns every point on its chrome, so
/// a caller seeing `over_popup` must consume the move and clear the
/// hover painted beneath it instead of letting the rail light up through
/// the overlay. `panel` is `None` when the rail can't be probed for this
/// move, which clears the row hover rather than freezing it.
pub fn color_variable_picker_hover(
    state: &mut EditorState,
    panel: Option<&PropertyPanel>,
    property_rect: Rect,
    point: Point2D,
) -> (bool, bool) {
    if state
        .editor_ui
        .property_color_variable_picker_open
        .is_none()
    {
        return (false, false);
    }
    let (over_popup, new_hover) = panel
        .map(|panel| {
            (
                panel.color_variable_picker_contains(property_rect, point),
                panel.color_variable_picker_row_at(property_rect, point),
            )
        })
        .unwrap_or((false, None));
    let changed = new_hover != state.editor_ui.property_color_variable_picker_hover;
    if changed {
        state.editor_ui.property_color_variable_picker_hover = new_hover;
    }
    (over_popup, changed)
}

/// Code-panel hover wash. Reuses the Code panel's click geometry so
/// framework chips, scroll chevrons, and body actions share hit-testing
/// with paint. `eligible` folds in each host's own suppression gates
/// (`over_topmost`, panel visibility) and clears the hover when false.
pub fn code_panel_hover(
    state: &mut EditorState,
    property_rect: Rect,
    point: Point2D,
    eligible: bool,
) -> bool {
    let panel = crate::widgets::PropertyPanel::for_selection(state);
    let on_code_tab = eligible
        && state.property_panel_visible()
        && matches!(
            state.editor_ui.effective_property_tab(),
            op_editor_core::PropertyTab::Code
        );
    let right_edge = property_rect.origin.x + property_rect.size.x;
    let (new_fw_hover, new_action_hover) =
        if on_code_tab && point.x >= property_rect.origin.x && point.x <= right_edge {
            panel
                .as_ref()
                .map(|panel| panel.code_hover_at(property_rect, point))
                .unwrap_or((None, None))
        } else {
            (None, None)
        };
    if new_fw_hover != state.codegen.framework_hover
        || new_action_hover != state.codegen.action_hover
    {
        state.codegen.framework_hover = new_fw_hover;
        state.codegen.action_hover = new_action_hover;
        return true;
    }
    false
}

// ─── Canvas ────────────────────────────────────────────────────────────

/// Canvas hierarchy hover target: the scene hit path resolved to the
/// current level's primary node. Frame labels participate as explicit
/// root targets. Shared canvas paint then outlines that node solid plus
/// all of its direct children dashed.
///
/// Reads the CURRENT layout scene without refreshing it — hover must
/// never rebuild a stale scene (same discipline as layer-row hover).
pub fn canvas_hover_target(
    state: &EditorState,
    scene: &LayoutScene,
    canvas_rect: Rect,
    point: Point2D,
) -> Option<NodeId> {
    let canvas = CanvasViewport::from_editor(state, scene);
    if let Some(root) = canvas.frame_label_at_point(canvas_rect, point) {
        return Some(NodeId::new(root));
    }
    let local = Point2D::new(
        point.x - canvas_rect.origin.x,
        point.y - canvas_rect.origin.y,
    );
    let doc = state.viewport.to_document(local);
    scene
        .node_path_at_doc_point(doc, state.viewport.zoom)
        .and_then(|path| {
            let path = path.into_iter().map(NodeId::new).collect::<Vec<_>>();
            op_editor_core::selection_resolve::resolve_canvas_depth_targets(
                &path,
                state.editor_ui.entered_container.as_ref(),
            )
            .map(|targets| targets.primary)
        })
}
