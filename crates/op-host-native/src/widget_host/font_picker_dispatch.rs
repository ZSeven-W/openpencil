//! Font-family picker dispatch — system-font enumeration, search
//! keystroke routing, hover tracking, and wheel scroll for the
//! Typography section's searchable dropdown
//! (`op_editor_ui::widgets::property_panel_typography`).

use super::WidgetHostNative;
use op_editor_ui::widgets::PropertyPanel;
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
    fn property_keyboard_owner_active(&self) -> bool {
        let ui = &self.editor_state.editor_ui;
        let property_surface_visible = self.editor_state.property_panel_visible()
            && (ui.mobile_sheet == Some(op_editor_core::size_class::MobileSheetKind::Properties)
                || (ui.expanded_touch_layout() && ui.mobile_sheet.is_none()));
        if !property_surface_visible {
            return false;
        }
        let image_input_active = if ui.image_panel.search_open || ui.image_panel.generate_open {
            ui.image_panel
                .active_input(ui.agent_settings.image_generation_configured())
                .is_some()
        } else {
            false
        };
        self.editor_state.ui.property_focus.is_some()
            || ui.effect_param_focus.is_some()
            || (ui.font_picker.open
                && ui.font_picker_purpose == Some(op_editor_core::FontPickerPurpose::PropertyText))
            || image_input_active
    }

    /// Commit and release every input owned by the Property surface before
    /// touch chrome hides or replaces that surface. Without this explicit
    /// blur, the invisible property draft wins the keyboard router ahead of
    /// the newly visible AI input.
    pub(in crate::widget_host) fn release_property_keyboard_owner(&mut self) -> bool {
        let had_focus = self.editor_state.ui.property_focus.is_some()
            || self.editor_state.editor_ui.effect_param_focus.is_some();
        self.commit_property_focus_if_any();

        let property_font_open = self.editor_state.editor_ui.font_picker.open
            && self.editor_state.editor_ui.font_picker_purpose
                == Some(op_editor_core::FontPickerPurpose::PropertyText);
        if property_font_open {
            self.editor_state.editor_ui.close_font_picker();
        }

        let image_popover_open = self.editor_state.editor_ui.image_panel.search_open
            || self.editor_state.editor_ui.image_panel.generate_open;
        if image_popover_open {
            self.clear_image_input_selection_drag();
            self.editor_state.editor_ui.image_panel.close_popovers();
        }

        let changed = had_focus || property_font_open || image_popover_open;
        if changed {
            self.mark_dirty();
        }
        changed
    }

    /// Dismiss the currently open touch-editor surface and release any text
    /// input that would otherwise remain hidden after a sheet switch or a
    /// responsive size-class transition.
    pub fn dismiss_mobile_surface(&mut self) -> bool {
        let sheet = self.editor_state.editor_ui.mobile_sheet;
        let property_will_hide = sheet
            == Some(op_editor_core::size_class::MobileSheetKind::Properties)
            || (self.editor_state.editor_ui.expanded_touch_layout() && sheet.is_none());
        let mut changed = if property_will_hide {
            self.release_property_keyboard_owner()
        } else {
            false
        };
        if sheet == Some(op_editor_core::size_class::MobileSheetKind::Ai) {
            changed |= self.editor_state.editor_ui.close_chat_model_picker();
            if self.editor_state.chat.focused || !self.editor_state.chat.collapsed {
                self.editor_state.chat.blur_input(self.now_ms);
                self.editor_state.chat.collapsed = true;
                changed = true;
            }
        }
        if sheet.is_some() {
            self.editor_state.editor_ui.mobile_sheet = None;
            changed = true;
        }
        if changed {
            self.mark_dirty();
        }
        changed
    }

    /// A size-class boundary can hide every touch surface at once, even when
    /// the current sheet flag names a different overlay. Release all touch
    /// text owners before the caller installs the new responsive class.
    pub fn reset_mobile_surfaces_for_size_class_change(
        &mut self,
        next: op_editor_core::size_class::EditorSizeClass,
    ) -> bool {
        let sheet = self.editor_state.editor_ui.mobile_sheet;
        let property_remains_visible = next.is_expanded()
            && matches!(
                sheet,
                None | Some(op_editor_core::size_class::MobileSheetKind::Properties)
            );
        let mut changed = if property_remains_visible {
            false
        } else {
            self.release_property_keyboard_owner()
        };
        changed |= self.editor_state.editor_ui.close_chat_model_picker();
        if sheet == Some(op_editor_core::size_class::MobileSheetKind::Ai)
            && (self.editor_state.chat.focused || !self.editor_state.chat.collapsed)
        {
            self.editor_state.chat.blur_input(self.now_ms);
            self.editor_state.chat.collapsed = true;
            changed = true;
        }
        if self.editor_state.editor_ui.mobile_sheet.take().is_some() {
            changed = true;
        }
        if changed {
            self.mark_dirty();
        }
        changed
    }

    pub(in crate::widget_host) fn reveal_property_keyboard_owner(&mut self) -> bool {
        if !self.editor_state.editor_ui.touch_chrome()
            || !self.property_keyboard_owner_active()
            || self.keyboard_occlusion <= 0.0
            || self.last_viewport_w <= 0.0
            || self.last_viewport_h <= 0.0
        {
            return false;
        }
        let property_rect = self.property_rect(self.last_viewport_w, self.last_viewport_h);
        let Some(next) = PropertyPanel::for_selection_at(&self.editor_state, self.now_ms)
            .and_then(|panel| panel.keyboard_owner_scroll_offset(property_rect))
        else {
            return false;
        };
        let scroll = &mut self.editor_state.editor_ui.property_panel_scroll;
        if (scroll.offset - next).abs() <= f32::EPSILON {
            return false;
        }
        scroll.offset = next;
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn try_scroll_settings_font_picker(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        if !self.editor_state.editor_ui.agent_settings_open {
            return false;
        }
        let (panel, panel_rect) = self.agent_settings_geometry(viewport_width, viewport_height);
        let Some(layout) = panel.font_picker_layout(panel_rect) else {
            return false;
        };
        if !layout.popup.contains(Point2D::new(x, y)) {
            return false;
        }
        let ui = &mut self.editor_state.editor_ui;
        let next = (ui.font_picker.scroll.offset - delta_y).clamp(0.0, layout.max_scroll);
        if next != ui.font_picker.scroll.offset {
            ui.font_picker.scroll.offset = next;
            ui.font_picker.hover = None;
            ui.font_picker_import_hover = false;
            self.mark_dirty();
        }
        true
    }

    /// Enumerate installed font families once per process and cache
    /// them on the editor state (sorted, deduped against the bundled
    /// list — mirrors TS `use-system-fonts.ts` post-processing).
    pub(in crate::widget_host) fn ensure_system_fonts_loaded(&mut self) {
        if self.editor_state.editor_ui.system_fonts_loaded {
            return;
        }
        let bundled_families = jian_skia::list_bundled_families();
        let bundled: std::collections::HashSet<String> = bundled_families
            .iter()
            .map(|family| family.to_lowercase())
            .collect();
        let mut families = crate::backend::enumerate_system_font_families();
        families.retain(|f| !bundled.contains(&f.to_lowercase()));
        families.sort_by_key(|a| a.to_lowercase());
        families.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
        self.editor_state.editor_ui.system_font_families = std::sync::Arc::new(families);
        self.editor_state.editor_ui.bundled_font_families = std::sync::Arc::new(bundled_families);
        self.editor_state.editor_ui.system_fonts_loaded = true;
    }

    /// Rebuild the imported-font snapshot from the live `jian-skia`
    /// registry (call after an import / remove lands). Threads the
    /// families into `editor_ui` exactly like `system_font_families`;
    /// the picker paints them in the Imported group.
    pub fn refresh_imported_fonts(&mut self) {
        let families: Vec<String> = jian_skia::list_families()
            .into_iter()
            .map(|m| m.family)
            .collect();
        self.editor_state.editor_ui.imported_font_families = std::sync::Arc::new(families);
        self.mark_dirty();
    }

    /// Drain the pending `ImportFont` request (raised by the picker's
    /// Import row). The desktop host opens the native file dialog.
    pub fn take_font_import_request(&mut self) -> bool {
        std::mem::take(&mut self.editor_state.editor_ui.pending_font_import)
    }

    /// Drain the pending `RemoveImportedFont` request, yielding the
    /// resolved family the desktop host removes from `FontStore`.
    pub fn take_font_remove_request(&mut self) -> Option<String> {
        self.editor_state.editor_ui.pending_font_remove.take()
    }

    /// Close the picker and drop its transient search / scroll state.
    pub(in crate::widget_host) fn close_font_picker(&mut self) {
        self.editor_state.editor_ui.close_font_picker();
    }

    /// Route a printable char into the picker's search box. Returns
    /// `true` when consumed (picker open).
    pub(in crate::widget_host) fn apply_font_picker_text(&mut self, c: char) -> bool {
        if !self.editor_state.editor_ui.font_picker.open || c.is_control() {
            return false;
        }
        let ui = &mut self.editor_state.editor_ui;
        ui.font_picker_search.push(c);
        // A narrower list invalidates the old scroll / hover.
        ui.font_picker.scroll.offset = 0.0;
        ui.font_picker.hover = None;
        ui.font_picker_import_hover = false;
        self.mark_dirty();
        true
    }

    /// Backspace in the picker's search box. Consumes the key while
    /// the picker is open even when the draft is already empty (the
    /// key must not fall through to node deletion).
    pub(in crate::widget_host) fn apply_font_picker_backspace(&mut self) -> bool {
        if !self.editor_state.editor_ui.font_picker.open {
            return false;
        }
        let ui = &mut self.editor_state.editor_ui;
        if ui.font_picker_search.pop().is_some() {
            ui.font_picker.scroll.offset = 0.0;
            ui.font_picker.hover = None;
            ui.font_picker_import_hover = false;
            self.mark_dirty();
        }
        true
    }

    /// Track the picker row under the cursor (entry + import-row hover
    /// washes). The picker is a floating overlay, so while the cursor is
    /// over the popup this CONSUMES the move — otherwise
    /// the caller falls through to the canvas / layer hovers behind the
    /// popup and lets a node highlight bleed through (穿透). When the
    /// pointer is genuinely outside the popup, the picker's own hovers are
    /// cleared and lower layers run normally. The second result reports a
    /// hover-state change separately so a higher-popup exit can continue into
    /// the model picker in the same cursor event without losing its repaint.
    pub(in crate::widget_host) fn update_font_picker_hover(
        &mut self,
        x: f32,
        y: f32,
        panel: Option<&PropertyPanel>,
    ) -> (bool, bool) {
        if !self.editor_state.editor_ui.font_picker.open {
            return (false, false);
        }
        let property_rect = self.property_rect(self.last_viewport_w, self.last_viewport_h);
        let point = Point2D::new(x, y);
        // Resolve over-popup + both hovers up front so the panel's
        // immutable borrow is released before we mutate the host.
        let resolved = panel.map(|panel| {
            let over_popup = panel.font_picker_contains(property_rect, point);
            if over_popup {
                (
                    true,
                    panel.font_picker_entry_index_at(property_rect, point),
                    panel.font_picker_import_action_at(property_rect, point),
                )
            } else {
                (false, None, false)
            }
        });
        let (over_popup, entry_hover, import_hover) = resolved.unwrap_or((false, None, false));

        let ui = &mut self.editor_state.editor_ui;
        let mut changed = false;
        if ui.font_picker.hover != entry_hover {
            ui.font_picker.hover = entry_hover;
            changed = true;
        }
        if ui.font_picker_import_hover != import_hover {
            ui.font_picker_import_hover = import_hover;
            changed = true;
        }
        if over_popup {
            // Drop any stale canvas / layer hover sitting behind the popup
            // so it can't highlight through, then consume the move.
            let cleared = self.clear_lower_overlay_hover();
            if changed && !cleared {
                self.mark_dirty();
            }
            (true, changed || cleared)
        } else {
            // Pointer left the popup — clear the picker's own hovers and
            // let the caller run the lower hover layers.
            if changed {
                self.mark_dirty();
            }
            (false, changed)
        }
    }

    /// Wheel over the open picker scrolls its list viewport instead
    /// of the panel. Returns `true` when consumed.
    pub(in crate::widget_host) fn try_scroll_font_picker(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        if !self.editor_state.editor_ui.font_picker.open {
            return false;
        }
        self.refresh_layout_scene();
        let Some(panel) = PropertyPanel::for_selection(&self.editor_state) else {
            return false;
        };
        let rect = self.property_rect(viewport_width, viewport_height);
        let point = Point2D::new(x, y);
        if !panel.font_picker_contains(rect, point) {
            return false;
        }
        let max = panel.font_picker_max_scroll(rect);
        let ui = &mut self.editor_state.editor_ui;
        // A positive delta means the content travelled down (the reader
        // moved up), so the offset shrinks — the one convention every
        // other scroll surface in both hosts uses.
        let next = (ui.font_picker.scroll.offset - delta_y).clamp(0.0, max);
        if next != ui.font_picker.scroll.offset {
            ui.font_picker.scroll.offset = next;
            ui.font_picker.hover = None;
            ui.font_picker_import_hover = false;
            self.mark_dirty();
        }
        true
    }

    /// Outside-click dismiss for the font-family picker. A click on a
    /// picker row / the trigger is applied; a click inside the popup
    /// body (search box, group header) is swallowed; anything else
    /// closes the picker. Returns `true` when the press was consumed.
    pub(in crate::widget_host) fn dismiss_font_picker_on_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        use op_editor_ui::widgets::PropertyPanelAction as A;
        if !self.editor_state.editor_ui.font_picker.open {
            return false;
        }
        self.refresh_layout_scene();
        if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
            let rect = self.property_rect(viewport_width, viewport_height);
            let point = Point2D::new(x, y);
            if let Some(action) = panel.hit_test_action(rect, point) {
                if matches!(
                    action,
                    A::SetFontFamilyIndex(_)
                        | A::ToggleFontFamilyPicker
                        | A::ImportFont
                        | A::RemoveImportedFont(_)
                ) {
                    self.apply_property_action(action);
                    return true;
                }
            }
            if panel.font_picker_contains(rect, point) {
                // Inside the popup body (search box / header rows) —
                // swallow, keep it open.
                return true;
            }
        }
        self.close_font_picker();
        self.mark_dirty();
        true
    }

    /// The right-rail rect every property-panel hit-test uses.
    pub(in crate::widget_host) fn property_rect(
        &self,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Rect {
        let mut rect = op_editor_ui::widgets::host_canvas_geometry::property_panel_rect(
            &self.editor_state,
            viewport_width,
            viewport_height,
        );
        let property_input_owns_keyboard =
            self.editor_state.editor_ui.touch_chrome() && self.property_keyboard_owner_active();
        if property_input_owns_keyboard && self.keyboard_occlusion > 0.0 {
            let visible_bottom = self.keyboard_visible_bottom(viewport_height);
            if self.editor_state.editor_ui.compact_layout() {
                // A phone bottom sheet is itself a movable overlay. In
                // landscape, retaining its old bottom-anchored top can leave
                // only a few pixels (or none) above the IME. Re-anchor this
                // focused surface to the keyboard while keeping the stable
                // app bar and canvas geometry untouched.
                let top = op_editor_ui::widgets::host_canvas_geometry::touch_app_bar_height(
                    &self.editor_state,
                );
                let max_height = (visible_bottom - top).max(0.0);
                let min_height = 280.0_f32.min(max_height);
                let sheet_height = (viewport_height * 0.58).clamp(min_height, max_height);
                rect.origin.y = visible_bottom - sheet_height;
                rect.size.y = sheet_height;
            } else {
                let current_bottom = rect.origin.y + rect.size.y;
                rect.size.y = (current_bottom.min(visible_bottom) - rect.origin.y).max(0.0);
            }
        }
        rect
    }
}

#[cfg(test)]
mod tests {
    use op_editor_core::{EditorState, NodeId};

    fn host_with_text_selected() -> crate::widget_host::WidgetHostNative {
        let mut host = crate::widget_host::WidgetHostNative::new();
        *host.editor_state_mut() = EditorState::sample();
        host.editor_state_mut()
            .set_single_selection(NodeId::new("n11"));
        host
    }

    #[test]
    fn toggle_populates_system_fonts_once() {
        let mut host = host_with_text_selected();
        assert!(!host.editor_state().editor_ui.system_fonts_loaded);
        host.apply_property_action(
            op_editor_ui::widgets::PropertyPanelAction::ToggleFontFamilyPicker,
        );
        let ui = &host.editor_state().editor_ui;
        assert!(ui.font_picker.open);
        assert!(ui.system_fonts_loaded);
        // macOS / Linux / Windows CI all have at least one installed
        // family that isn't in the bundled list.
        assert!(!ui.system_font_families.is_empty());
        // Bundled names never appear in the system group.
        assert!(!ui
            .system_font_families
            .iter()
            .any(|f| f.eq_ignore_ascii_case("Inter")));
    }

    #[test]
    fn set_font_family_index_writes_font_family_and_closes() {
        let mut host = host_with_text_selected();
        // This unit test exercises dispatch against the runtime availability
        // snapshot, so seed that boundary explicitly instead of relying on an
        // unrelated Skia test having registered the desktop's bundled fonts in
        // the process-global registry first. Windows intentionally skips those
        // DirectWrite-backed registration tests.
        host.editor_state_mut().editor_ui.system_fonts_loaded = true;
        host.editor_state_mut().editor_ui.bundled_font_families =
            std::sync::Arc::new(vec!["Inter".into()]);
        host.apply_property_action(
            op_editor_ui::widgets::PropertyPanelAction::ToggleFontFamilyPicker,
        );
        // Search for a registered bundled family so index 0 is deterministic.
        for c in "inter".chars() {
            assert!(host.apply_font_picker_text(c));
        }
        host.apply_property_action(
            op_editor_ui::widgets::PropertyPanelAction::SetFontFamilyIndex(0),
        );
        let ui = &host.editor_state().editor_ui;
        assert!(!ui.font_picker.open);
        assert!(ui.font_picker_search.is_empty());
        let node = host.editor_state().selected_node().expect("text node");
        let jian_ops_schema::node::PenNode::Text(text) = node else {
            panic!("n11 must be a text node");
        };
        assert_eq!(text.font_family.as_deref(), Some("Inter"));
    }

    #[test]
    fn search_keystrokes_route_only_while_open() {
        let mut host = host_with_text_selected();
        assert!(!host.apply_font_picker_text('a'));
        host.apply_property_action(
            op_editor_ui::widgets::PropertyPanelAction::ToggleFontFamilyPicker,
        );
        assert!(host.apply_font_picker_text('a'));
        assert_eq!(host.editor_state().editor_ui.font_picker_search, "a");
        assert!(host.apply_font_picker_backspace());
        assert!(host.editor_state().editor_ui.font_picker_search.is_empty());
        // Backspace is still swallowed on an empty draft (no node
        // deletion fall-through while the picker is open).
        assert!(host.apply_font_picker_backspace());
    }
}
