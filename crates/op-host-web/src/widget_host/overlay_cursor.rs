//! Cursor-move + release handling for the floating overlays the web
//! host paints (colour picker, Design-MD / Icon-picker / Component-
//! Browser panels, shape-picker + file-menu dropdowns). Mirrors the
//! corresponding branches of the native host's `widget_host/input.rs`
//! and `geometry.rs::update_dropdown_hover`; lives in a sibling module
//! so the spine file stays lean.

use op_editor_ui::Point2D;

use super::WidgetHost;

impl WidgetHost {
    /// Patch the live scene's resolved fill / stroke for the active
    /// colour-picker drag without a layout rebuild — the web twin of the
    /// native `try_patch_color_drag`. Returns `true` when the patch
    /// applied (caller flags the live-sync push but skips `mark_dirty`'s
    /// scene rebuild); `false` when the edit is not in the patchable set
    /// (variable-mode, gradient-stop / effect target, or a node not
    /// solid-patchable) and the caller must rebuild.
    ///
    /// Unlike the native host this path performs no instance-write
    /// redirect, so the doc write lands on the anchor directly and the
    /// anchor is exactly what is patched.
    fn try_patch_color_drag(&mut self) -> bool {
        use op_editor_core::ui_draft::ColorTarget;
        use op_editor_ui::widgets::color_picker::hsv_to_rgb;
        if self.editor_state_dirty {
            return false;
        }
        let (hue, sat, val, is_fill) = {
            let Some(state) = self.editor_state.ui.color_picker.as_ref() else {
                return false;
            };
            if state.variable.is_some() {
                return false;
            }
            let is_fill = match state.target {
                ColorTarget::Fill => true,
                ColorTarget::Stroke => false,
                _ => return false,
            };
            (state.hue, state.sat, state.val, is_fill)
        };
        let anchor = self.editor_state.selection.anchor.clone();
        if !anchor.is_real() || !self.editor_state.is_editable(&anchor) {
            return false;
        }
        let Some(node) = self.editor_state.selected_node() else {
            return false;
        };
        // The picker write only lands when the anchor carries a writable
        // solid paint slot. A Ref anchor has none, so `set_selected_color`
        // is a no-op — patching the scene would then show a colour the doc
        // never received (and snap back on release). Require the
        // freshly-written hex to be present; this is the web's guard
        // against Ref anchors (no instance redirect runs on this path) and
        // also screens out gradient-only / slotless nodes.
        //
        // The loader also bakes the paint body's own opacity into the
        // resolved scene alpha; `set_node_*` bakes node opacity on top.
        // Fold the body opacity in so a fill / stroke authored below 100 %
        // does not paint too opaque on every drag frame.
        let (wrote_paint, body_opacity) = if is_fill {
            (
                op_editor_core::fills::first_solid_fill_hex(node).is_some(),
                op_editor_core::fills::first_solid_fill_opacity(node),
            )
        } else {
            (
                op_editor_core::fills::first_solid_stroke_hex(node).is_some(),
                op_editor_core::fills::first_solid_stroke_opacity(node),
            )
        };
        if !wrote_paint {
            return false;
        }
        let mut color = hsv_to_rgb(hue, sat, val);
        color.a *= body_opacity.clamp(0.0, 1.0);
        let ids = [anchor.as_str().to_string()];
        let patched = if is_fill {
            self.layout_scene.set_node_fill(&ids, color)
        } else {
            self.layout_scene.set_node_stroke_color(&ids, color)
        };
        if patched {
            self.scene_cache.invalidate();
        }
        patched
    }

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
                // A solid Fill/Stroke change touches no layout — patch the
                // resolved scene paint in place instead of rebuilding per
                // drag frame. The doc fill still changed, so flag the
                // live-canvas push, but skip the scene rebuild (this is
                // `mark_dirty` minus `editor_state_dirty`).
                if self.try_patch_color_drag() {
                    #[cfg(feature = "canvaskit")]
                    {
                        self.doc_sync_dirty = true;
                    }
                } else {
                    self.mark_dirty();
                }
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
                let point = Point2D::new(x, y);
                let new_hover = DesignMdPanel::for_editor(&self.editor_state)
                    .and_then(|p| p.hover_at(panel_rect, point));
                let changed = new_hover != self.editor_state.editor_ui.design_md_hover;
                if changed {
                    self.editor_state.editor_ui.design_md_hover = new_hover;
                    self.mark_dirty();
                }
                if panel_rect.contains(point) {
                    self.clear_lower_overlay_hover();
                    return true;
                }
                if changed {
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
                let point = Point2D::new(x, y);
                let new_hover = ComponentBrowserPanel::for_editor(&self.editor_state)
                    .and_then(|p| p.hover_at(panel_rect, point));
                let changed = new_hover != self.editor_state.editor_ui.component_browser_hover;
                if changed {
                    self.editor_state.editor_ui.component_browser_hover = new_hover;
                    self.mark_dirty();
                }
                if panel_rect.contains(point) {
                    self.clear_lower_overlay_hover();
                    return true;
                }
                if changed {
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
                let point = Point2D::new(x, y);
                let new_hover = IconPickerPanel::for_editor(&self.editor_state)
                    .and_then(|p| p.hover_at(panel_rect, point));
                let changed = new_hover != self.editor_state.editor_ui.icon_picker.hover;
                if changed {
                    self.editor_state.editor_ui.icon_picker.hover = new_hover;
                    self.mark_dirty();
                }
                if panel_rect.contains(point) {
                    self.clear_lower_overlay_hover();
                    return true;
                }
                if changed {
                    return true;
                }
            }
        }
        let over_dropdown =
            self.over_dropdown_overlay(x, y, self.last_viewport_w, self.last_viewport_h);
        let dropdown_changed = self.update_dropdown_hover(x, y);
        let underlay_cleared = over_dropdown && self.clear_hover_under_dropdown_overlay();
        if dropdown_changed || underlay_cleared || over_dropdown {
            return true;
        }
        if self.editor_state.editor_ui.variables_panel_open {
            use op_editor_ui::widgets::variables_panel::VariablesPanel;
            if let Some(panel_rect) =
                self.variables_panel_rect(self.last_viewport_w, self.last_viewport_h)
            {
                let point = Point2D::new(x, y);
                if panel_rect.contains(point) {
                    let new_hover = VariablesPanel::for_editor_at(&self.editor_state, self.now_ms)
                        .hover_at(panel_rect, point);
                    if new_hover != self.editor_state.editor_ui.variables_panel_hover {
                        self.editor_state.editor_ui.variables_panel_hover = new_hover;
                        self.mark_dirty();
                    }
                    self.clear_lower_overlay_hover();
                    return true;
                }
            }
        }
        false
    }

    fn clear_hover_under_dropdown_overlay(&mut self) -> bool {
        let mut changed = false;
        {
            let ui = &mut self.editor_state.editor_ui;
            changed |= ui.canvas_hover_node.take().is_some();
            changed |= ui.hovered_layer_id.take().is_some();
            changed |= ui.hovered_page_index.take().is_some();
            changed |= ui.fill_type_picker.hover.take().is_some();
            changed |= ui.toolbar_hover.take().is_some();
            changed |= ui.align_toolbar_hover.take().is_some();
            changed |= ui.statusbar_hover.take().is_some();
            changed |= ui.topbar_button_hover.take().is_some();
            changed |= ui.chat_model_picker.hover.take().is_some();
            changed |= ui.chat_design_block_hover.take().is_some();
            changed |= ui.chat_footer_hover.take().is_some();
            changed |= ui.chat_example_hover.take().is_some();
            changed |= ui.export_picker_hover.take().is_some();
            changed |= ui.property_action_hover.take().is_some();
            changed |= ui.property_tab_hover.take().is_some();
            if let Some(menu) = ui.layer_context_menu.as_mut() {
                changed |= menu.menu.hover.take().is_some();
            }
        }
        if let Some(menu) = self.editor_state.ui.path_anchor_menu.as_mut() {
            changed |= menu.menu.hover.take().is_some();
        }
        changed |= self.editor_state.codegen.framework_hover.take().is_some();
        changed |= self.editor_state.codegen.action_hover.take().is_some();
        if changed {
            self.mark_dirty();
        }
        changed
    }

    /// Update the file-menu / locale / shape-picker dropdown hover highlights
    /// from the cursor. At most one is open at a time; a top-most
    /// floating panel covering the point suppresses updates. Returns
    /// `true` on change. Port of the native
    /// `geometry.rs::update_dropdown_hover`.
    fn update_dropdown_hover(&mut self, x: f32, y: f32) -> bool {
        if self.over_topmost_panel(x, y, self.last_viewport_w, self.last_viewport_h)
            && !self.over_dropdown_overlay(x, y, self.last_viewport_w, self.last_viewport_h)
        {
            return false;
        }
        if self.editor_state.editor_ui.file_menu_open {
            use op_editor_ui::widgets::file_menu::FileMenu;
            self.refresh_layout_scene();
            let top_bar_rect = self.top_bar_rect(self.last_viewport_w);
            let anchor = self.top_bar().file_menu_rect_for(top_bar_rect);
            let menu = FileMenu::from_editor_ui(&self.editor_state.editor_ui, self.wall_now_secs);
            let panel = menu.rect_at(anchor);
            let new_hover = menu.hovered_at(panel, Point2D::new(x, y));
            if new_hover != self.editor_state.editor_ui.file_menu.hover {
                self.editor_state.editor_ui.file_menu.hover = new_hover;
                self.mark_dirty();
                return true;
            }
        }
        if self.editor_state.editor_ui.locale_picker.open {
            use op_editor_ui::widgets::locale_picker::LocalePicker;
            self.refresh_layout_scene();
            let panel = self.locale_picker_rect(self.last_viewport_w);
            let picker = LocalePicker::for_editor_ui(&self.editor_state.editor_ui);
            let new_hover = match picker.hit_popup(panel, Point2D::new(x, y)) {
                op_editor_ui::widgets::locale_picker::SelectHit::Row(idx) => Some(idx),
                op_editor_ui::widgets::locale_picker::SelectHit::Inside
                | op_editor_ui::widgets::locale_picker::SelectHit::Outside => None,
            };
            if new_hover != self.editor_state.editor_ui.locale_picker.hover {
                self.editor_state.editor_ui.locale_picker.hover = new_hover;
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
        if self.editor_state.editor_ui.effect_add_picker_open {
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
                let new_hover = panel.effect_add_menu_row_at(property_rect, Point2D::new(x, y));
                if new_hover != self.editor_state.editor_ui.effect_add_menu_hover {
                    self.editor_state.editor_ui.effect_add_menu_hover = new_hover;
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
