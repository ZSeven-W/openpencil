//! Font-family picker dispatch — system-font enumeration, search
//! keystroke routing, hover tracking, and wheel scroll for the
//! Typography section's searchable dropdown
//! (`op_editor_ui::widgets::property_panel_typography`).

use super::WidgetHostNative;
use op_editor_ui::widgets::property_panel_typography::{
    font_picker_entries, BUNDLED_FONT_FAMILIES,
};
use op_editor_ui::widgets::{PropertyPanel, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
    /// Enumerate installed font families once per process and cache
    /// them on the editor state (sorted, deduped against the bundled
    /// list — mirrors TS `use-system-fonts.ts` post-processing).
    pub(in crate::widget_host) fn ensure_system_fonts_loaded(&mut self) {
        if self.editor_state.editor_ui.system_fonts_loaded {
            return;
        }
        let bundled: std::collections::HashSet<String> = BUNDLED_FONT_FAMILIES
            .iter()
            .map(|f| f.to_lowercase())
            .collect();
        let mut families = crate::backend::enumerate_system_font_families();
        families.retain(|f| !bundled.contains(&f.to_lowercase()));
        families.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        families.dedup();
        self.editor_state.editor_ui.system_font_families = std::sync::Arc::new(families);
        self.editor_state.editor_ui.system_fonts_loaded = true;
    }

    /// Resolve a `SetFontFamilyIndex` against the SAME entries list
    /// the panel painted / hit-tested (system list + live search).
    pub(in crate::widget_host) fn font_picker_family_at(&self, index: usize) -> Option<String> {
        let ui = &self.editor_state.editor_ui;
        font_picker_entries(&ui.system_font_families, &ui.font_picker_search)
            .get(index)
            .map(|e| e.family.to_string())
    }

    /// Close the picker and drop its transient search / scroll state.
    pub(in crate::widget_host) fn close_font_picker(&mut self) {
        let ui = &mut self.editor_state.editor_ui;
        ui.font_family_picker_open = false;
        ui.font_picker_search.clear();
        ui.font_picker_scroll = 0.0;
        ui.font_picker_hover = None;
    }

    /// Route a printable char into the picker's search box. Returns
    /// `true` when consumed (picker open).
    pub(in crate::widget_host) fn apply_font_picker_text(&mut self, c: char) -> bool {
        if !self.editor_state.editor_ui.font_family_picker_open || c.is_control() {
            return false;
        }
        let ui = &mut self.editor_state.editor_ui;
        ui.font_picker_search.push(c);
        // A narrower list invalidates the old scroll / hover.
        ui.font_picker_scroll = 0.0;
        ui.font_picker_hover = None;
        self.mark_dirty();
        true
    }

    /// Backspace in the picker's search box. Consumes the key while
    /// the picker is open even when the draft is already empty (the
    /// key must not fall through to node deletion).
    pub(in crate::widget_host) fn apply_font_picker_backspace(&mut self) -> bool {
        if !self.editor_state.editor_ui.font_family_picker_open {
            return false;
        }
        let ui = &mut self.editor_state.editor_ui;
        if ui.font_picker_search.pop().is_some() {
            ui.font_picker_scroll = 0.0;
            ui.font_picker_hover = None;
            self.mark_dirty();
        }
        true
    }

    /// Track the picker row under the cursor (hover wash). Returns
    /// `true` when the hovered entry changed.
    pub(in crate::widget_host) fn update_font_picker_hover(&mut self, x: f32, y: f32) -> bool {
        if !self.editor_state.editor_ui.font_family_picker_open {
            return false;
        }
        self.refresh_layout_scene();
        let new_hover = PropertyPanel::for_selection(&self.editor_state).and_then(|panel| {
            panel.font_picker_entry_index_at(
                self.property_rect(self.last_viewport_w, self.last_viewport_h),
                Point2D::new(x, y),
            )
        });
        if new_hover != self.editor_state.editor_ui.font_picker_hover {
            self.editor_state.editor_ui.font_picker_hover = new_hover;
            self.mark_dirty();
            return true;
        }
        false
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
        if !self.editor_state.editor_ui.font_family_picker_open {
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
        let next = (ui.font_picker_scroll + delta_y).clamp(0.0, max);
        if next != ui.font_picker_scroll {
            ui.font_picker_scroll = next;
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
        if !self.editor_state.editor_ui.font_family_picker_open {
            return false;
        }
        self.refresh_layout_scene();
        if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
            let rect = self.property_rect(viewport_width, viewport_height);
            let point = Point2D::new(x, y);
            if let Some(action) = panel.hit_test_action(rect, point) {
                if matches!(action, A::SetFontFamilyIndex(_) | A::ToggleFontFamilyPicker) {
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
        Rect {
            origin: Point2D::new(
                viewport_width - self.editor_state.editor_ui.property_panel_width,
                TOP_BAR_HEIGHT,
            ),
            size: Point2D::new(
                self.editor_state.editor_ui.property_panel_width,
                (viewport_height - TOP_BAR_HEIGHT).max(0.0),
            ),
        }
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
        assert!(ui.font_family_picker_open);
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
        host.apply_property_action(
            op_editor_ui::widgets::PropertyPanelAction::ToggleFontFamilyPicker,
        );
        // Search for a bundled family so index 0 is deterministic.
        for c in "playfair".chars() {
            assert!(host.apply_font_picker_text(c));
        }
        host.apply_property_action(
            op_editor_ui::widgets::PropertyPanelAction::SetFontFamilyIndex(0),
        );
        let ui = &host.editor_state().editor_ui;
        assert!(!ui.font_family_picker_open);
        assert!(ui.font_picker_search.is_empty());
        let node = host.editor_state().selected_node().expect("text node");
        let jian_ops_schema::node::PenNode::Text(text) = node else {
            panic!("n11 must be a text node");
        };
        assert_eq!(text.font_family.as_deref(), Some("Playfair Display"));
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
