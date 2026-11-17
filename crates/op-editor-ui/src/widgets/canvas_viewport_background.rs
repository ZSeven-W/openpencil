//! Active-page background resolution for the editor canvas.

use crate::util::parse_hex_color;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, Theme};
use op_editor_core::EditorState;

/// Resolve the active page's authored background. The boolean reports
/// whether the normal editor grid remains visible.
pub(super) fn resolve(state: &EditorState, fallback: Color) -> (Color, bool) {
    let Some(pages) = state.doc.pages.as_ref().filter(|pages| !pages.is_empty()) else {
        return (fallback, true);
    };
    let page = &pages[state.ui.active_page_index.min(pages.len() - 1)];
    match page.background_color.as_deref().and_then(parse_hex_color) {
        Some(color) => (color, false),
        None => (fallback, true),
    }
}

/// Theme canvas color shared by the editor and its noninteractive margins.
pub(super) fn editor_canvas_surface(state: &EditorState, theme: &Theme) -> Color {
    if state.editor_ui.workspace.active {
        super::home_surface::StudioPalette::for_mode(state.editor_ui.effective_theme_mode()).canvas
    } else {
        theme.canvas_surface
    }
}

/// Paint only the canvas surface and grid into a platform-owned side band.
pub fn paint_canvas_background_extension(
    cx: &mut PaintCx<'_>,
    state: &EditorState,
    rect: Rect,
    canvas_origin: Point2D,
) {
    let theme = super::editor_state_ext::theme_for(&state.editor_ui);
    let surface = editor_canvas_surface(state, &theme);
    // An authored page color can conflict with system glyphs. Keep the
    // theme surface in system-owned space and match the grid's visibility.
    let (_, show_grid) = resolve(state, surface);
    cx.backend.fill_rect(rect, surface);
    if show_grid && !state.editor_ui.preview.mode {
        cx.backend.save();
        cx.backend.clip_rect(rect);
        super::canvas_viewport_grid::paint_grid_from(
            cx,
            rect,
            canvas_origin,
            &state.viewport,
            &theme,
        );
        cx.backend.restore();
    }
}
