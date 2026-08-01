//! Shared Scene Template Center press transitions for native and web hosts.

use op_editor_core::{ButtonPressTarget, EditorState};

use crate::widgets::{SceneTemplateHit, SceneTemplatePanel};
use crate::{Point2D, Rect};

/// Route one pointer press to the non-modal Scene Template Center.
///
/// `None` means the press was outside the panel and must fall through.
/// `Some(changed)` means the panel swallowed it; hosts repaint when `changed`
/// is true.
///
/// Choosing a card does not open the document here. It raises
/// `scene_template_center.pending_open`, which the host drains: loading a
/// document is a host capability (it may have to prompt about unsaved work,
/// and on native it touches the recent-files list), and a widget that reached
/// into that would have to be reimplemented per host — the exact fork this
/// shared layer exists to prevent.
pub fn press_scene_template_center(
    state: &mut EditorState,
    panel_rect: Rect,
    point: Point2D,
    now_ms: u64,
) -> Option<bool> {
    let panel = SceneTemplatePanel::for_editor(state)?;
    let hover = panel.hover_at(panel_rect, point);
    let hit = panel.hit_test(panel_rect, point)?;
    let pressed = hover.map(ButtonPressTarget::SceneTemplate);
    let pressed_changed = state.editor_ui.pressed_button != pressed;
    state.editor_ui.pressed_button = pressed;

    let changed = match hit {
        SceneTemplateHit::Close => state.editor_ui.close_scene_template_center(),
        SceneTemplateHit::FocusSearch(offset) => {
            let center = &mut state.editor_ui.scene_template_center;
            let changed = center.search.caret() != offset;
            center.search.set_caret(offset, now_ms);
            changed
        }
        SceneTemplateHit::SelectFilter(filter) => {
            let center = &mut state.editor_ui.scene_template_center;
            let changed =
                center.filter != filter || center.scroll.offset != 0.0 || center.hover.is_some();
            center.filter = filter;
            // A filter change reorders the grid, so a retained scroll offset
            // or hover index would point at a different card than the one the
            // pointer is over.
            center.scroll.offset = 0.0;
            center.hover = None;
            changed
        }
        SceneTemplateHit::SelectTemplate(id) => {
            state.editor_ui.scene_template_center.request_open(id);
            true
        }
        SceneTemplateHit::Inside => false,
    };
    Some(changed || pressed_changed)
}

/// Route a wheel/trackpad scroll to the card grid.
///
/// `None` when the pointer is outside the panel, so the host falls through to
/// canvas zoom/pan.
pub fn scroll_scene_template_center(
    state: &mut EditorState,
    panel_rect: Rect,
    point: Point2D,
    delta_y: f32,
) -> Option<bool> {
    let panel = SceneTemplatePanel::for_editor(state)?;
    if !panel_rect.contains(point) {
        return None;
    }
    let max_scroll = panel.max_scroll(panel_rect);
    let center = &mut state.editor_ui.scene_template_center;
    let previous = center.scroll.offset;
    center.scroll.offset = (previous - delta_y).clamp(0.0, max_scroll);
    // Swallow the event either way: a grid that has nothing left to scroll
    // must not hand the wheel to the canvas underneath it.
    Some(center.scroll.offset != previous)
}

/// Resolve hover for a pointer move.
///
/// Returns `(over_panel, changed)` — hosts need the first to suppress
/// canvas-level hover while the pointer is over the panel, which is the gate
/// a floating overlay is easiest to forget (measured 2026-07-29 on the colour
/// variable popover: hover fell straight through to the layer underneath).
pub fn hover_scene_template_center(
    state: &mut EditorState,
    panel_rect: Rect,
    point: Point2D,
) -> (bool, bool) {
    let Some(panel) = SceneTemplatePanel::for_editor(state) else {
        return (false, false);
    };
    let hover = panel.hover_at(panel_rect, point);
    let over_panel = panel_rect.contains(point);
    let center = &mut state.editor_ui.scene_template_center;
    let changed = center.hover != hover;
    center.hover = hover;
    (over_panel, changed)
}

#[cfg(test)]
#[path = "scene_template_press_flow_tests.rs"]
mod scene_template_press_flow_tests;
