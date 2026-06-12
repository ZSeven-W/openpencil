use super::helpers::{TOOLBAR_INSET_X, TOOLBAR_INSET_Y};
use super::WidgetHostNative;
use op_editor_ui::widgets::variables_panel::{
    VARIABLES_PANEL_DEFAULT_HEIGHT, VARIABLES_PANEL_MIN_HEIGHT, VARIABLES_PANEL_MIN_WIDTH,
    VARIABLES_PANEL_WIDTH,
};
use op_editor_ui::widgets::TOOLBAR_WIDTH;
use op_editor_ui::{Point2D, Rect};

const VARIABLES_PANEL_GAP: f32 = 8.0;
/// TS reserves 72 px of container width / 16 px of height beyond the
/// panel when clamping a resize (`variables-panel.tsx` maxW/maxH).
const VARIABLES_PANEL_MAX_W_MARGIN: f32 = 72.0;
const VARIABLES_PANEL_MAX_H_MARGIN: f32 = 16.0;

impl WidgetHostNative {
    pub(in crate::widget_host) fn variables_panel_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<Rect> {
        if !self.editor_state.editor_ui.variables_panel_open {
            return None;
        }
        let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        let x = cx0 + TOOLBAR_INSET_X + TOOLBAR_WIDTH + VARIABLES_PANEL_GAP;
        let y = cy0 + TOOLBAR_INSET_Y;
        let max_w = (cx0 + cw - x - VARIABLES_PANEL_MAX_W_MARGIN).max(0.0);
        let max_h = (cy0 + ch - y - VARIABLES_PANEL_MAX_H_MARGIN).max(0.0);
        if max_w < VARIABLES_PANEL_MIN_WIDTH.min(240.0)
            || max_h < VARIABLES_PANEL_MIN_HEIGHT.min(120.0)
        {
            return None;
        }
        // User-resized size wins over the 820x480 default; both clamp
        // to [TS minimums, available canvas] so a viewport shrink
        // self-corrects a stale stored size.
        let (want_w, want_h) = self
            .editor_state
            .editor_ui
            .variables_panel_size
            .unwrap_or((VARIABLES_PANEL_WIDTH, VARIABLES_PANEL_DEFAULT_HEIGHT));
        Some(Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(
                want_w.clamp(VARIABLES_PANEL_MIN_WIDTH.min(max_w), max_w),
                want_h.clamp(VARIABLES_PANEL_MIN_HEIGHT.min(max_h), max_h),
            ),
        })
    }

    /// Apply an in-flight resize drag: write the new size from the
    /// cursor position (the panel is anchored top-left, so width /
    /// height derive directly from the cursor minus the origin).
    pub(in crate::widget_host) fn apply_variables_panel_resize(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        use op_editor_ui::widgets::variables_panel::VariablesResizeEdge;
        let Some(edge) = self.variables_resize else {
            return false;
        };
        let Some(rect) = self.variables_panel_rect(viewport_w, viewport_h) else {
            return false;
        };
        let (mut w, mut h) = (rect.size.x, rect.size.y);
        if matches!(
            edge,
            VariablesResizeEdge::Right | VariablesResizeEdge::Corner
        ) {
            w = (x - rect.origin.x).max(VARIABLES_PANEL_MIN_WIDTH);
        }
        if matches!(
            edge,
            VariablesResizeEdge::Bottom | VariablesResizeEdge::Corner
        ) {
            h = (y - rect.origin.y).max(VARIABLES_PANEL_MIN_HEIGHT);
        }
        if (w, h) != (rect.size.x, rect.size.y) {
            // The rect getter clamps to the canvas region, so storing
            // the raw cursor-derived size is safe.
            self.editor_state.editor_ui.variables_panel_size = Some((w, h));
            self.mark_dirty();
        }
        true
    }
}
