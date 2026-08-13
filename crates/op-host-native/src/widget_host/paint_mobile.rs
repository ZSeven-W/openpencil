//! Mobile chrome painting (app bar, bottom dock, sheets, pills) —
//! split out of `paint.rs` for the 800-line ceiling.

use crate::widget_host::WidgetHostNative;
use crate::NativeFrameBackend;
use op_editor_ui::widgets::editor_state_ext::theme_for;
use op_editor_ui::widgets::host_canvas_geometry as canvas_geometry;
use op_editor_ui::widgets::{PaintCx, Widget};
use op_editor_ui::{Point2D, Rect, RenderBackend};

impl WidgetHostNative {
    pub(in crate::widget_host) fn paint_mobile_chrome(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        // Native touch chrome replaces the desktop top bar at every tablet
        // size class. Geometry branches inside the shared widgets.
        if self.editor_state.editor_ui.touch_chrome() {
            let app_bar = op_editor_ui::widgets::MobileAppBar::for_editor(&self.editor_state);
            let app_bar_rect =
                canvas_geometry::touch_app_bar_rect(&self.editor_state, viewport_width);
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            app_bar.paint(&mut cx, app_bar_rect);

            // The dock is navigation chrome, not sheet content. Hiding it
            // while any modal surface is open removes the duplicate More
            // entry and prevents invisible controls from stealing taps.
            if self.editor_state.editor_ui.mobile_sheet.is_none()
                && !self.editor_state.editor_ui.variables_panel_open
            {
                let dock = op_editor_ui::widgets::MobileDock::for_editor(&self.editor_state);
                let dock_rect = canvas_geometry::touch_dock_rect(
                    &self.editor_state,
                    viewport_width,
                    viewport_height,
                );
                dock.paint(&mut cx, dock_rect);
            }

            if self.editor_state.editor_ui.mobile_sheet
                == Some(op_editor_core::size_class::MobileSheetKind::More)
            {
                let panel = self.mobile_sheet_rect(
                    viewport_width,
                    viewport_height,
                    op_editor_core::size_class::MobileSheetKind::More,
                );
                let theme = theme_for(&self.editor_state.editor_ui);
                op_editor_ui::widgets::mobile_chrome::paint_more_panel(
                    &mut cx,
                    &self.editor_state,
                    &theme,
                    panel,
                );
            }
            // Contextual Properties and Delete actions for the selection.
            if self.selection_actions_visible() {
                let theme = theme_for(&self.editor_state.editor_ui);
                let pill = op_editor_ui::widgets::mobile_chrome::selection_actions_rect_for(
                    &self.editor_state,
                    viewport_width,
                    viewport_height,
                );
                let label = op_editor_ui::widgets::editor_state_ext::translate(
                    &self.editor_state.editor_ui,
                    "rightPanel.design",
                );
                op_editor_ui::widgets::mobile_chrome::paint_selection_actions(
                    &mut cx, &theme, pill, label,
                );
            }
            // Page navigation never paints inside or above a modal surface.
            let page_count = self
                .editor_state
                .doc
                .pages
                .as_ref()
                .map(|pages| pages.len())
                .unwrap_or(1);
            if page_count > 1
                && self.editor_state.editor_ui.mobile_sheet.is_none()
                && !self.editor_state.editor_ui.variables_panel_open
            {
                let theme = theme_for(&self.editor_state.editor_ui);
                let pill = op_editor_ui::widgets::mobile_chrome::page_pill_rect_for(
                    &self.editor_state,
                    viewport_width,
                    viewport_height,
                );
                op_editor_ui::widgets::mobile_chrome::paint_page_pill(
                    &mut cx,
                    &theme,
                    pill,
                    page_count,
                    self.editor_state.ui.active_page_index,
                );
            }
        }
    }

    /// Paint the modal dimming layer after the canvas and before any touch
    /// sheet. Tablet popovers use their own outlined surface and keep the
    /// surrounding editor readable; phone sheets get a restrained scrim.
    pub(in crate::widget_host) fn paint_mobile_sheet_scrim(
        &self,
        frame: &mut NativeFrameBackend<'_>,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        if !self.editor_state.editor_ui.compact_layout()
            || self.editor_state.editor_ui.mobile_sheet.is_none()
        {
            return;
        }
        frame.fill_rect(
            Rect {
                origin: Point2D::new(
                    0.0,
                    canvas_geometry::touch_app_bar_height(&self.editor_state),
                ),
                size: Point2D::new(
                    viewport_width,
                    (viewport_height - canvas_geometry::touch_app_bar_height(&self.editor_state))
                        .max(0.0),
                ),
            },
            op_editor_ui::widgets::mobile_more_panel::more_scrim_color(),
        );
    }
}

/// Mobile property bottom sheet: rounded-top clip + drag handle +
/// close button over the panel content. A free function so callers can
/// keep an immutable `ui` borrow alive.
pub(in crate::widget_host) fn paint_property_sheet(
    state: &op_editor_core::EditorState,
    panel: &op_editor_ui::widgets::PropertyPanel,
    cx: &mut PaintCx<'_>,
    property_rect: Rect,
) {
    let compact = state.editor_ui.compact_layout();
    if compact {
        cx.backend.save();
        cx.backend
            .clip_round_rect_per_corner(property_rect, [20.0, 20.0, 0.0, 0.0]);
    } else if state.editor_ui.medium_layout() {
        cx.backend.save();
        cx.backend.clip_round_rect(property_rect, 18.0);
    }
    panel.paint(cx, property_rect);
    if compact || state.editor_ui.medium_layout() {
        cx.backend.restore();
    }
    let theme = theme_for(&state.editor_ui);
    if state.editor_ui.medium_layout() {
        cx.backend
            .stroke_round_rect(property_rect, 18.0, theme.border, 1.0);
    }
    if compact {
        let handle = canvas_geometry::mobile_sheet_handle_rect(property_rect);
        cx.backend.fill_round_rect(handle, 2.0, theme.border);
    }
    if compact || state.editor_ui.medium_layout() {
        let close_target = op_editor_ui::widgets::mobile_chrome::sheet_close_rect(property_rect);
        let icon = op_editor_ui::widgets::icons::Icon::from_name("x")
            .unwrap_or(op_editor_ui::widgets::icons::Icon::Close);
        op_editor_ui::widgets::mobile_chrome::paint_touch_icon(
            cx,
            close_target,
            icon,
            18.0,
            theme.muted_foreground,
        );
    }
}

impl WidgetHostNative {
    /// Mobile Layers sheet: the rail content + the shared sheet chrome.
    pub(in crate::widget_host) fn paint_mobile_layers_sheet(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        let sheet = self.mobile_sheet_rect(
            viewport_width,
            viewport_height,
            op_editor_core::size_class::MobileSheetKind::Layers,
        );
        let theme = theme_for(&self.editor_state.editor_ui);
        frame.save();
        if self.editor_state.editor_ui.compact_layout() {
            frame.fill_round_rect_per_corner(sheet, [20.0, 20.0, 0.0, 0.0], theme.card);
            frame.clip_round_rect_per_corner(sheet, [20.0, 20.0, 0.0, 0.0]);
        } else {
            frame.fill_round_rect(sheet, 18.0, theme.card);
            frame.stroke_round_rect(sheet, 18.0, theme.border, 1.0);
            frame.clip_round_rect(sheet, 18.0);
        }
        self.paint_left_rail(frame, viewport_width, viewport_height);
        frame.restore();
        let mut cx = PaintCx {
            backend: &mut *frame,
        };
        op_editor_ui::widgets::mobile_chrome::paint_sheet_header(
            &mut cx,
            &theme,
            sheet,
            op_editor_ui::widgets::editor_state_ext::translate(
                &self.editor_state.editor_ui,
                "layers.title",
            ),
            self.editor_state.editor_ui.compact_layout(),
        );
    }
}
