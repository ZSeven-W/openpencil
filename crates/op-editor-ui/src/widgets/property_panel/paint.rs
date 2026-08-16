//! `PropertyPanel` painting — the `Widget` impl (tab strip, section
//! stack, action feedback wash, every anchored popover) plus the
//! out-of-rail overlay pass hosts call late in composition.
//!
//! Split out of `property_panel.rs` to keep both files under the
//! openpencil 800-line cap.

use super::{action_wash_rect, PropertyPanel, PropertyPanelAction};
use crate::widgets::button::paint_button_feedback_wash;
use crate::widgets::property_panel_sections as sections;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect};

impl Widget for PropertyPanel {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        // Vertical extent is "as much as you give me" — the host
        // clips at the rail rect. Reporting 800 here is just a
        // placeholder for the abstract widget tree.
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(cx.available_width, 800.0),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let rect = self.begin_density_paint(cx.backend, rect);
        cx.backend.fill_rect(rect, self.theme.card);
        cx.backend.fill_rect(
            Rect {
                origin: rect.origin,
                size: Point2D::new(1.0, rect.size.y),
            },
            self.theme.border,
        );

        let x = rect.origin.x;
        let w = rect.size.x;
        // The Design / Code tab strip is pinned to the panel top —
        // painted fixed, above (and never scrolled with) the section
        // content.
        let tab_bottom = sections::paint_tab_strip(
            cx,
            &self.theme,
            &self.labels,
            sections::TabStripState {
                active: self.tab,
                hover: self.tab_hover,
                show_interact: self.snapshot.widget.is_some(),
                touch_controls: self.density_scale > 1.0,
            },
            x,
            rect.origin.y,
            w,
        );
        let edit_ctx = sections::EditContext {
            focus: self.focus,
            draft: self.draft.as_str(),
            input: &self.input,
            caret: self.caret_pos,
            select_all: self.select_all,
            now_ms: self.now_ms,
        };
        let caps = self.capabilities();
        if matches!(self.tab, op_editor_core::PropertyTab::Code) {
            crate::widgets::property_panel_code::paint_code_panel_in_panel_with_locale_pressed_and_touch(
                cx,
                &self.theme,
                &self.codegen,
                self.locale,
                rect,
                self.now_ms,
                self.codegen_pressed,
                self.density_scale > 1.0,
            );
            self.end_density_paint(cx.backend);
            return;
        }
        if self.page_only {
            cx.backend.save();
            cx.backend.clip_rect(Rect {
                origin: Point2D::new(x, tab_bottom),
                size: Point2D::new(w, (rect.origin.y + rect.size.y - tab_bottom).max(0.0)),
            });
            crate::widgets::property_panel_page::paint_page_inspector(
                cx,
                &self.theme,
                &edit_ctx,
                self.locale,
                &self.page_name,
                self.page_background.as_deref(),
                rect,
            );
            cx.backend.restore();
            self.end_density_paint(cx.backend);
            return;
        }
        // Section content scrolls below the pinned tab strip; clip it
        // so a scrolled-up section can't paint over the tabs or bleed
        // onto the neighbouring rail. Overlays (fill / export pickers)
        // anchor to `scrolled` — the same shifted rect the layout
        // walker uses (it adds `TAB_HEIGHT`), so paint + hit-test of
        // the sections agree.
        cx.backend.save();
        cx.backend.clip_rect(Rect {
            origin: Point2D::new(x, tab_bottom),
            size: Point2D::new(w, (rect.origin.y + rect.size.y - tab_bottom).max(0.0)),
        });
        let scroll = self.effective_scroll(rect);
        let scrolled = Rect {
            origin: Point2D::new(rect.origin.x, rect.origin.y - scroll),
            size: rect.size,
        };
        // First section sits just below the pinned tab strip:
        // `tab_bottom - scroll` == `scrolled.origin.y + TAB_HEIGHT`,
        // matching the layout walker's `+= TAB_HEIGHT` step.
        let mut y = tab_bottom - scroll;
        y = sections::paint_node_header(cx, &self.theme, &self.snapshot, x, y, w);
        if self.visible_sections().create_component {
            y = crate::widgets::property_panel_instance::paint_component_block(
                cx,
                &self.theme,
                &self.labels,
                self.visible_sections().component_button,
                &self.instance_component_options,
                self.instance_component_target.as_deref(),
                tab_bottom,
                rect.origin.y + rect.size.y,
                x,
                y,
                w,
            );
        }
        y = sections::paint_position_section(
            cx,
            &self.theme,
            &self.snapshot,
            &edit_ctx,
            &self.labels,
            self.snapshot.has_corner_radius,
            self.corner_expand_open,
            x,
            y,
            w,
        );
        let flex_section_y = y;
        if caps.flex_layout {
            y = crate::widgets::property_panel_flex::paint_flex_section(
                cx,
                &self.theme,
                &self.snapshot,
                &edit_ctx,
                &self.labels,
                self.locale,
                self.padding_edit_mode,
                self.density_scale > 1.0,
                x,
                y,
                w,
            );
        }
        if caps.size_options {
            y = sections::paint_size_section(
                cx,
                &self.theme,
                &self.snapshot,
                &edit_ctx,
                &self.labels,
                self.size_flags,
                self.snapshot.can_clip_content,
                self.density_scale > 1.0,
                x,
                y,
                w,
            );
        }
        if self.snapshot.icon.is_some() {
            y = crate::widgets::property_panel_icon::paint_icon_section(
                cx,
                &self.theme,
                &self.snapshot,
                self.locale,
                x,
                y,
                w,
            );
        }
        if caps.text && self.snapshot.text.is_some() {
            y = crate::widgets::property_panel_text::paint_text_section(
                cx,
                &self.theme,
                &self.snapshot,
                &edit_ctx,
                self.locale,
                self.density_scale > 1.0,
                x,
                y,
                w,
            );
        }
        if self.snapshot.widget.is_some() {
            y = crate::widgets::property_panel_widget::paint_widget_section(
                cx,
                &self.theme,
                &self.snapshot,
                &edit_ctx,
                self.locale,
                x,
                y,
                w,
            );
        }
        if caps.image && self.snapshot.is_image_node {
            y = crate::widgets::property_panel_image_node::paint_image_node_section(
                cx,
                &self.theme,
                &self.snapshot,
                self.image_panel_view
                    .as_ref()
                    .and_then(|v| v.warning.as_ref()),
                self.locale,
                x,
                y,
                w,
            );
        }
        if caps.opacity {
            y = sections::paint_layer_section(
                cx,
                &self.theme,
                &self.snapshot,
                &self.labels,
                &edit_ctx,
                !self.is_multi,
                self.density_scale > 1.0,
                self.locale,
                x,
                y,
                w,
            );
        }
        if caps.fill {
            y = sections::paint_fill_section(
                cx,
                &self.theme,
                &self.snapshot,
                &edit_ctx,
                &self.labels,
                self.fill_type,
                self.fill_type_picker.open,
                self.fill_variable_ref.as_deref(),
                self.color_variable_count > 0 || self.fill_variable_ref.is_some(),
                self.density_scale > 1.0,
                self.locale,
                x,
                y,
                w,
            );
        }
        let stroke_section_y = y;
        if caps.stroke {
            y = crate::widgets::property_panel_stroke::paint_stroke_section(
                cx,
                &self.theme,
                &self.snapshot,
                &edit_ctx,
                &self.labels,
                self.stroke_variable_ref.as_deref(),
                self.color_variable_count > 0 || self.stroke_variable_ref.is_some(),
                x,
                y,
                w,
                self.stroke_edit_mode,
            );
        }
        if caps.effects {
            y = sections::paint_effects_section(
                cx,
                &self.theme,
                &self.labels,
                &self.snapshot.effects,
                &edit_ctx,
                self.density_scale > 1.0,
                x,
                y,
                w,
            );
        }
        if caps.interactions {
            y = sections::paint_interactions_section(
                cx,
                &self.theme,
                &self.snapshot.interactions,
                self.locale,
                x,
                y,
                w,
            );
        }
        if caps.export {
            let _ = sections::paint_export_section(
                cx,
                &self.theme,
                &self.labels,
                self.export_format,
                self.export_scale,
                x,
                y,
                w,
            );
        }
        // Base-action feedback belongs to the scrolling panel body. Paint it
        // before every floating menu so a stale hover/press wash can never be
        // composited over popup chrome. The menu-specific hover states below
        // then remain the sole feedback on the top-most surface.
        if self.action_hover.is_some() || self.action_pressed.is_some() {
            let rects = sections::action_button_rects_with_fill_picker(
                self.scrolled_rect(rect),
                self.visible_sections(),
                &self.snapshot.effects,
                &self.snapshot.fills,
                &self.snapshot.interactions,
                self.fill_type_picker.open,
                self.fill_type_picker_index,
                self.font_picker.open,
                self.font_weight_picker_open,
                self.export_scale_picker_open,
                self.export_format_picker_open,
                self.padding_mode_popover_open,
            );
            if let Some(i) = self.action_hover {
                if let Some((action, r)) = rects.get(i) {
                    let wash = action_wash_rect(action, *r, &self.labels, self.locale, cx.backend);
                    paint_button_feedback_wash(
                        cx.backend,
                        &self.theme,
                        wash,
                        6.0,
                        true,
                        self.action_pressed == Some(i),
                    );
                    if matches!(action, PropertyPanelAction::ToggleCornerExpand) {
                        crate::widgets::property_panel_corner::paint_tooltip(
                            cx,
                            &self.theme,
                            *r,
                            self.labels.corner_per_corner,
                        );
                    }
                }
            }
            if let Some(i) = self.action_pressed {
                if self.action_hover != Some(i) {
                    if let Some((action, r)) = rects.get(i) {
                        let wash =
                            action_wash_rect(action, *r, &self.labels, self.locale, cx.backend);
                        paint_button_feedback_wash(cx.backend, &self.theme, wash, 6.0, false, true);
                    }
                }
            }
        }
        // Effects "+" add-menu overlay.
        if self.effect_add_picker_open {
            if let Some(add_rect) = self.effect_add_button_rect(scrolled) {
                crate::widgets::property_panel_effects::paint_effect_add_menu(
                    cx,
                    &self.theme,
                    &self.labels,
                    add_rect,
                    self.effect_add_menu_hover,
                );
            }
        }
        // Interactions section's Navigate/Back/Remove popover.
        if caps.interactions && self.interaction_menu_open {
            if let Some(anchor) = self.interaction_menu_anchor_rect(scrolled) {
                let rows = crate::widgets::property_panel_interactions::interaction_menu_rows(
                    self.locale,
                    &self.screen_paths,
                    self.interaction_menu_removable(),
                );
                crate::widgets::property_panel_interactions::paint_interaction_menu(
                    cx,
                    &self.theme,
                    anchor,
                    &rows,
                    self.interaction_menu_hover,
                );
            }
        }
        // Fill-type picker overlay sits on top of everything below
        // the Fill section so it can extend past the section divider.
        if caps.fill && self.fill_type_picker.open {
            let fi = self.fill_type_picker_index;
            if let Some(action_rect) = sections::fill_type_toggle_action_rect(
                scrolled,
                self.visible_sections(),
                &self.snapshot.effects,
                &self.snapshot.fills,
                fi,
            ) {
                let active = self
                    .snapshot
                    .fills
                    .get(fi)
                    .map(|f| f.fill_type)
                    .unwrap_or(self.fill_type);
                sections::paint_fill_type_picker(
                    cx,
                    &self.theme,
                    action_rect,
                    crate::widgets::property_panel_fill::fill_type_picker_viewport(rect),
                    &self.fill_type_picker,
                    active,
                    self.locale,
                );
            }
        }
        if caps.text && self.font_picker.open {
            if let Some(text) = self.snapshot.text.as_ref() {
                let entries = self.font_picker_entries();
                crate::widgets::property_panel_typography::paint_font_picker(
                    cx,
                    &self.theme,
                    scrolled,
                    self.visible_sections(),
                    self.locale,
                    &entries,
                    self.font_import_supported,
                    &self.font_picker_search,
                    &self.font_picker,
                    self.font_picker_import_hover,
                    &text.font_family,
                    self.now_ms,
                );
            }
        }
        if caps.text && self.font_weight_picker_open {
            if let Some(text) = self.snapshot.text.as_ref() {
                crate::widgets::property_panel_text::paint_font_weight_picker(
                    cx,
                    &self.theme,
                    scrolled,
                    self.visible_sections(),
                    self.locale,
                    text.font_weight,
                    self.font_weight_picker_hover,
                    self.font_weight_picker_pressed,
                );
            }
        }
        // Padding mode-selector popover — overlays the sections below
        // the gear. Anchored off the flex section's body top (after its
        // header), matching the y the action-rect walker passes to
        // `push_flex_action_rects`.
        if caps.flex_layout && self.padding_mode_popover_open {
            crate::widgets::property_panel_flex::paint_padding_mode_popover(
                cx,
                &self.theme,
                self.locale,
                self.padding_edit_mode,
                self.padding_mode_popover_hover,
                x,
                flex_section_y + crate::widgets::property_panel_inputs::SECTION_HEADER_HEIGHT,
                w,
                self.density_scale > 1.0,
            );
        }
        if caps.stroke && self.stroke_mode_popover_open {
            crate::widgets::property_panel_stroke::paint_stroke_mode_popover(
                cx,
                &self.theme,
                self.locale,
                self.stroke_edit_mode,
                self.stroke_mode_popover_hover,
                x,
                stroke_section_y,
                w,
                self.density_scale > 1.0,
            );
        }
        // Export-section inline select popups — painted last so the
        // scale / format dropdown overlays sit above every section.
        if caps.export && (self.export_scale_picker_open || self.export_format_picker_open) {
            sections::paint_export_picker(
                cx,
                &self.theme,
                scrolled,
                self.visible_sections(),
                &self.snapshot.effects,
                &self.snapshot.fills,
                &self.snapshot.interactions,
                self.export_scale_picker_open,
                self.export_format_picker_open,
                self.export_scale,
                self.export_format,
                self.export_picker_hover,
            );
        }
        // The colour-variable picker paints as a floating popover over the
        // visible rail, so a long variable list scrolls inside its own
        // capped box instead of stretching the inspector.
        if let Some(layout) = self.color_variable_picker_layout_logical(rect) {
            crate::widgets::property_panel_color_variables::paint_color_variable_picker(
                cx,
                &self.theme,
                &layout,
                &self.color_variables,
                self.bound_color_variable_ref(),
                self.color_variable_picker_hover,
                self.locale,
            );
        }
        self.paint_compositing_picker(cx, rect);
        cx.backend.restore();
        self.end_density_paint(cx.backend);
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Group);
        node.set_label(if self.page_only {
            self.page_name.clone()
        } else {
            self.snapshot.kind.clone()
        });
        node
    }
}

impl PropertyPanel {
    /// Paint inspector overlays that are allowed to extend out of the
    /// right rail. Hosts call this late in their composition pass so
    /// the image-fill / search / generate popovers sit above floating
    /// canvas controls.
    pub fn paint_overlays(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        if self.page_only {
            return;
        }
        let caps = self.capabilities();
        if !(caps.fill || caps.image) {
            return;
        }
        // The Code tab paints no Design sections — none of the
        // Design-anchored popovers may float over it.
        if matches!(self.tab, op_editor_core::PropertyTab::Code) {
            return;
        }
        let rect = self.begin_density_paint(cx.backend, rect);
        let scroll = self.effective_scroll(rect);
        let scrolled = Rect {
            origin: Point2D::new(rect.origin.x, rect.origin.y - scroll),
            size: rect.size,
        };
        if self.image_fill_popover_open {
            let edit_ctx = sections::EditContext {
                focus: self.focus,
                draft: self.draft.as_str(),
                input: &self.input,
                caret: self.caret_pos,
                select_all: self.select_all,
                now_ms: self.now_ms,
            };
            sections::paint_image_fill_popover(
                cx,
                &self.theme,
                scrolled,
                self.visible_sections(),
                &self.snapshot,
                &edit_ctx,
                self.locale,
            );
        }
        if caps.image && self.image_panel.search_open {
            crate::widgets::property_panel_image_popovers::paint_search_popover(
                cx,
                &self.theme,
                scrolled,
                self.visible_sections(),
                &self.image_panel,
                self.locale,
                self.now_ms,
            );
        }
        if caps.image && self.image_panel.generate_open {
            crate::widgets::property_panel_image_popovers::paint_generate_popover(
                cx,
                &self.theme,
                scrolled,
                self.visible_sections(),
                &self.image_panel,
                self.image_gen_profile.as_ref(),
                self.locale,
                self.now_ms,
            );
        }
        self.end_density_paint(cx.backend);
    }
}
