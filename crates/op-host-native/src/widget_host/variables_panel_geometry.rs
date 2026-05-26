use super::helpers::{rect_contains, TOOLBAR_INSET_X, TOOLBAR_INSET_Y};
use super::WidgetHostNative;
use op_editor_ui::widgets::TOOLBAR_WIDTH;
use op_editor_ui::{Point2D, Rect};

const VARIABLES_PANEL_DEFAULT_W: f32 = 820.0;
const VARIABLES_PANEL_DEFAULT_H: f32 = 480.0;
const VARIABLES_PANEL_MIN_W: f32 = 240.0;
const VARIABLES_PANEL_MIN_H: f32 = 120.0;
const VARIABLES_PANEL_GAP: f32 = 8.0;

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
        let max_w = cx0 + cw - x - VARIABLES_PANEL_GAP;
        let max_h = cy0 + ch - y - VARIABLES_PANEL_GAP;
        if max_w < VARIABLES_PANEL_MIN_W || max_h < VARIABLES_PANEL_MIN_H {
            return None;
        }
        Some(Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(
                VARIABLES_PANEL_DEFAULT_W.min(max_w),
                VARIABLES_PANEL_DEFAULT_H.min(max_h),
            ),
        })
    }

    pub(in crate::widget_host) fn over_variables_panel(
        &self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        self.variables_panel_rect(viewport_w, viewport_h)
            .is_some_and(|r| rect_contains(r, Point2D::new(x, y)))
    }
}
