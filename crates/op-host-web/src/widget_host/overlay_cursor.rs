//! Cursor-move + release handling for the floating overlays the web
//! host paints (colour picker, Design-MD / Icon-picker / Component-
//! Browser panels, shape-picker + file-menu dropdowns). Mirrors the
//! corresponding branches of the native host's `widget_host/input.rs`
//! and `geometry.rs::update_dropdown_hover`; lives in a sibling module
//! so the spine file stays lean.

use op_editor_ui::Point2D;

use super::WidgetHost;

impl WidgetHost {
    /// Overlay-owned cursor movement: live colour-picker drags, the
    /// floating panels' header drags + hover washes, and the open
    /// dropdowns' row hovers. Returns `true` when the move was
    /// consumed (the caller repaints and skips lower layers).
    pub(in crate::widget_host) fn apply_overlay_cursor_move(&mut self, x: f32, y: f32) -> bool {
        // Colour-picker SvBox / HueSlider drag — live HSV updates.
        if let Some(state) = self.editor_state.ui.color_picker.clone() {
            if let Some(kind) = state.drag {
                use op_editor_core::ui_draft::ColorPickerDrag;
                use op_editor_ui::widgets::color_picker::ColorPicker;
                let picker = ColorPicker::for_state(&self.editor_state, state.clone());
                let panel = picker.rect(self.last_viewport_w, self.last_viewport_h);
                let point = Point2D::new(x, y);
                match kind {
                    ColorPickerDrag::SvBox => {
                        let (s, v) = picker.sv_at(panel, point);
                        let _ = self.editor_state.color_picker_set_hsv(state.hue, s, v);
                    }
                    ColorPickerDrag::HueSlider => {
                        let h = picker.hue_at(panel, point);
                        let _ = self
                            .editor_state
                            .color_picker_set_hsv(h, state.sat, state.val);
                    }
                }
                self.mark_dirty();
                return true;
            }
        }
        // Top-most floating panel drags own cursor movement.
        if let Some(d) = self.design_md_drag {
            self.editor_state.editor_ui.design_md_panel_pos = Some((x - d.grab_dx, y - d.grab_dy));
            self.mark_dirty();
            return true;
        }
        // Design-MD panel hover (close / import / export / remove /
        // section headers).
        if self.editor_state.editor_ui.design_md_panel_open {
            use op_editor_ui::widgets::design_md_panel::DesignMdPanel;
            if let Some(panel_rect) =
                self.design_md_panel_rect(self.last_viewport_w, self.last_viewport_h)
            {
                let new_hover = DesignMdPanel::for_editor(&self.editor_state)
                    .and_then(|p| p.hover_at(panel_rect, Point2D::new(x, y)));
                if new_hover != self.editor_state.editor_ui.design_md_hover {
                    self.editor_state.editor_ui.design_md_hover = new_hover;
                    self.mark_dirty();
                    return true;
                }
            }
        }
        if let Some(d) = self.component_browser_drag {
            self.editor_state.editor_ui.component_browser_pos =
                Some((x - d.grab_dx, y - d.grab_dy));
            self.mark_dirty();
            return true;
        }
        // Component-browser panel hover (close / category pills / cards).
        if self.editor_state.editor_ui.component_browser_open {
            use op_editor_ui::widgets::component_browser_panel::ComponentBrowserPanel;
            if let Some(panel_rect) =
                self.component_browser_panel_rect(self.last_viewport_w, self.last_viewport_h)
            {
                let new_hover = ComponentBrowserPanel::for_editor(&self.editor_state)
                    .and_then(|p| p.hover_at(panel_rect, Point2D::new(x, y)));
                if new_hover != self.editor_state.editor_ui.component_browser_hover {
                    self.editor_state.editor_ui.component_browser_hover = new_hover;
                    self.mark_dirty();
                    return true;
                }
            }
        }
        if let Some(d) = self.icon_picker_drag {
            self.editor_state.editor_ui.icon_picker_panel_pos =
                Some((x - d.grab_dx, y - d.grab_dy));
            self.mark_dirty();
            return true;
        }
        // Icon-picker panel hover (close / icon rows / load-more).
        if self.editor_state.editor_ui.icon_picker.open {
            use op_editor_ui::widgets::icon_picker_panel::IconPickerPanel;
            if let Some(panel_rect) =
                self.icon_picker_panel_rect(self.last_viewport_w, self.last_viewport_h)
            {
                let new_hover = IconPickerPanel::for_editor(&self.editor_state)
                    .and_then(|p| p.hover_at(panel_rect, Point2D::new(x, y)));
                if new_hover != self.editor_state.editor_ui.icon_picker.hover {
                    self.editor_state.editor_ui.icon_picker.hover = new_hover;
                    self.mark_dirty();
                    return true;
                }
            }
        }
        self.update_dropdown_hover(x, y)
    }

    /// Update the file-menu / shape-picker dropdown hover highlights
    /// from the cursor. At most one is open at a time; a top-most
    /// floating panel covering the point suppresses updates. Returns
    /// `true` on change. Port of the native
    /// `geometry.rs::update_dropdown_hover` (minus the locale picker,
    /// whose hover state web doesn't track yet).
    fn update_dropdown_hover(&mut self, x: f32, y: f32) -> bool {
        if self.over_topmost_panel(x, y, self.last_viewport_w, self.last_viewport_h) {
            return false;
        }
        if self.editor_state.editor_ui.file_menu_open {
            use op_editor_ui::widgets::file_menu::FileMenu;
            use op_editor_ui::widgets::top_bar::TopBar;
            self.refresh_layout_scene();
            let top_bar_rect = op_editor_ui::Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(self.last_viewport_w, op_editor_ui::widgets::TOP_BAR_HEIGHT),
            };
            let anchor =
                TopBar::file_menu_rect(top_bar_rect, self.editor_state.editor_ui.window_fullscreen);
            // `0` clock — no wall-clock on wasm32 and no recent files
            // to age (see `dispatch_file_menu_press`).
            let menu = FileMenu::from_editor_ui(&self.editor_state.editor_ui, 0);
            let panel = menu.rect_at(anchor);
            let new_hover = menu
                .hovered_at(panel, Point2D::new(x, y))
                .map(op_editor_ui::widgets::editor_state_ext::file_menu_choice);
            if new_hover != self.editor_state.editor_ui.file_menu_hover {
                self.editor_state.editor_ui.file_menu_hover = new_hover;
                self.mark_dirty();
                return true;
            }
        }
        if self.editor_state.editor_ui.shape_picker.open {
            use op_editor_ui::widgets::shape_picker::ShapePicker;
            self.refresh_layout_scene();
            let panel = self.shape_picker_rect(self.last_viewport_w, self.last_viewport_h);
            let picker = ShapePicker::for_editor_ui(&self.editor_state.editor_ui);
            let new_hover = match picker.hit_popup(panel, Point2D::new(x, y)) {
                op_editor_ui::widgets::shape_picker::SelectHit::Row(idx) => Some(idx),
                op_editor_ui::widgets::shape_picker::SelectHit::Inside
                | op_editor_ui::widgets::shape_picker::SelectHit::Outside => None,
            };
            if new_hover != self.editor_state.editor_ui.shape_picker.hover {
                self.editor_state.editor_ui.shape_picker.hover = new_hover;
                self.mark_dirty();
                return true;
            }
        }
        if self.editor_state.editor_ui.fill_type_picker.open {
            use op_editor_ui::widgets::{PropertyPanel, TOP_BAR_HEIGHT};
            self.refresh_layout_scene();
            if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
                let property_rect = op_editor_ui::Rect {
                    origin: Point2D::new(
                        self.last_viewport_w - self.editor_state.editor_ui.property_panel_width,
                        TOP_BAR_HEIGHT,
                    ),
                    size: Point2D::new(
                        self.editor_state.editor_ui.property_panel_width,
                        (self.last_viewport_h - TOP_BAR_HEIGHT).max(0.0),
                    ),
                };
                let new_hover = panel.fill_type_picker_row_at(property_rect, Point2D::new(x, y));
                if new_hover != self.editor_state.editor_ui.fill_type_picker.hover {
                    self.editor_state.editor_ui.fill_type_picker.hover = new_hover;
                    self.mark_dirty();
                    return true;
                }
            }
        }
        false
    }

    /// End overlay-owned drags on mouse release. The colour-picker
    /// drag drop is non-consuming (mirrors native — the release may
    /// still end other gestures); panel-header drags consume.
    pub(in crate::widget_host) fn release_overlay_drags(&mut self) -> bool {
        if self.editor_state.ui.color_picker.is_some() {
            self.editor_state.color_picker_set_drag(None);
            self.mark_dirty();
        }
        if self.design_md_drag.take().is_some() {
            // Position was updated live; release only ends the drag.
            return true;
        }
        if self.component_browser_drag.take().is_some() {
            return true;
        }
        if self.icon_picker_drag.take().is_some() {
            return true;
        }
        false
    }
}
