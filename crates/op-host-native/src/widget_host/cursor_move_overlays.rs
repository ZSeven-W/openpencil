//! `apply_cursor_move` tiers 4-6 — the context-menu / Git / dropdown tier,
//! the property-popover tier, and the StatusBar / align-toolbar / chat
//! model-picker tier.
//!
//! Every block here follows one shape, and the shape is the invariant:
//!
//! ```text
//! if over_<surface> { clear lower hover; return Some(changed || below) }
//! if <surface>_changed {
//!     if chat_or_picker_owns_point { ctx.upper_hover_changed = true }  // fall through
//!     else { return Some(true) }
//! }
//! ```
//!
//! i.e. a surface that physically contains the point always consumes; a
//! surface that merely repainted only consumes when nothing painted above
//! it (the chat / its model picker) owns the point.

use super::cursor_move_ctx::CursorMoveCtx;
use super::WidgetHostNative;
use op_editor_ui::widgets::cursor_hover_flow as hover_flow;
use op_editor_ui::widgets::PropertyPanel;
use op_editor_ui::Point2D;

impl WidgetHostNative {
    /// Path-anchor menu, layer context menu, Git panel, file/locale/shape
    /// dropdowns. `None` — none of them consumed the move.
    pub(in crate::widget_host) fn cursor_move_menu_tiers(
        &mut self,
        ctx: &mut CursorMoveCtx,
    ) -> Option<bool> {
        let (x, y) = (ctx.x, ctx.y);
        let over_topmost = ctx.over_topmost;
        let chat_or_picker_owns_point = ctx.chat_or_picker_owns_point;
        // Path-anchor context menu is painted above Git and Chat. An unchanged
        // row still owns the point, so return without falling into the model
        // picker behind it.
        let over_path_menu =
            hover_flow::path_anchor_menu_contains(&self.editor_state, Point2D::new(x, y));
        let path_menu_changed = self.update_path_anchor_menu_hover(x, y);
        if over_path_menu {
            let below_changed = self.clear_chat_and_lower_hover();
            return Some(path_menu_changed || below_changed);
        }
        if path_menu_changed {
            if chat_or_picker_owns_point {
                ctx.upper_hover_changed = true;
            } else {
                return Some(true);
            }
        }
        if let Some(state) = self
            .editor_state
            .editor_ui
            .layer_context_menu
            .clone()
            .filter(|_| !over_topmost)
        {
            use op_editor_ui::widgets::layer_context_menu::LayerContextMenu;
            let menu = LayerContextMenu::for_state(&self.editor_state, state.clone());
            let over_menu = menu.rect().contains(Point2D::new(x, y));
            let new_hover = menu.hovered_row_at(Point2D::new(x, y));
            let menu_hover_changed = new_hover != state.menu.hover;
            if menu_hover_changed {
                let mut next = state;
                next.menu.hover = new_hover;
                self.editor_state.editor_ui.layer_context_menu = Some(next);
                self.mark_dirty();
            }
            if over_menu {
                let below_changed = self.clear_chat_and_lower_hover();
                return Some(menu_hover_changed || below_changed);
            }
            if menu_hover_changed {
                if chat_or_picker_owns_point {
                    ctx.upper_hover_changed = true;
                } else {
                    return Some(true);
                }
            }
        }
        // Collaboration paints above Git and ordinary dropdowns, but below
        // the path/layer context menus resolved above.
        if let Some(consumed) = self.cursor_move_collab_panel(ctx) {
            return Some(consumed);
        }
        // Git is painted and pressed above Chat. Update both Git views before
        // the picker and keep an unchanged point inside the Git footprint from
        // leaking into the picker behind it.
        let over_git = self
            .git_panel_outer_rect(self.last_viewport_w, self.last_viewport_h)
            .is_some_and(|rect| rect.contains(Point2D::new(x, y)));
        let git_hover_changed =
            self.update_git_panel_empty_hover(x, y) | self.update_git_panel_ready_hover(x, y);
        if over_git {
            let below_changed = self.clear_chat_and_lower_hover();
            return Some(git_hover_changed || below_changed);
        }
        if git_hover_changed {
            if chat_or_picker_owns_point {
                ctx.upper_hover_changed = true;
            } else {
                return Some(true);
            }
        }
        // File-menu / locale / shape dropdown hover (`geometry.rs`).
        if self.update_dropdown_hover(x, y, over_topmost) {
            if chat_or_picker_owns_point {
                ctx.upper_hover_changed = true;
            } else {
                return Some(true);
            }
        }
        None
    }

    /// Property-panel popovers (image, export, effects, compositing,
    /// interactions, padding / stroke, font weight, font family).
    /// `None` — no popover consumed the move.
    pub(in crate::widget_host) fn cursor_move_property_overlay_tiers(
        &mut self,
        ctx: &mut CursorMoveCtx,
    ) -> Option<bool> {
        let (x, y) = (ctx.x, ctx.y);
        let point = ctx.point;
        let property_rect = ctx.property_rect;
        let over_topmost = ctx.over_topmost;
        let chat_or_picker_owns_point = ctx.chat_or_picker_owns_point;
        let property_popover_open = self.editor_state.editor_ui.export_scale_picker_open
            || self.editor_state.editor_ui.export_format_picker_open
            || self.editor_state.editor_ui.compositing_picker.open
            || self.editor_state.editor_ui.effect_add_picker_open
            || self.editor_state.editor_ui.interaction_menu_open
            || self.editor_state.editor_ui.padding_mode_popover_open
            || self.editor_state.editor_ui.stroke_mode_popover_open
            || self.editor_state.editor_ui.font_weight_picker_open
            || self.editor_state.editor_ui.font_picker.open
            || self
                .editor_state
                .editor_ui
                .property_color_variable_picker_open
                .is_some()
            || self.editor_state.editor_ui.image_fill_popover_open
            || self.editor_state.editor_ui.image_panel.search_open
            || self.editor_state.editor_ui.image_panel.generate_open;
        if property_popover_open && ctx.property_panel_probe.is_none() {
            // The old per-popover helpers each refreshed and rebuilt the same
            // panel. Refresh once, then share one immutable snapshot across
            // every overlay and the eventual base-property hover pass.
            self.refresh_layout_scene();
            ctx.property_panel_probe = Some(PropertyPanel::for_selection(&self.editor_state));
        }
        let property_panel = ctx.property_panel_probe.as_ref().and_then(Option::as_ref);
        // Image Search / Generate and image-fill adjustment popovers are
        // painted and pressed above Chat. Their popup body receives first
        // refusal even while the Chat model picker is open; reuse the same
        // PropertyPanel snapshot as every other property overlay.
        let over_image_popup = property_panel.is_some_and(|panel| {
            panel.image_popovers_contain(property_rect, point)
                || panel.image_fill_popover_contains(property_rect, point)
        });
        if over_image_popup {
            return Some(self.clear_chat_and_lower_hover());
        }
        // Export-section select-popup row hover (no-op when closed).
        let over_export_popup = !over_topmost
            && property_panel
                .is_some_and(|panel| panel.export_picker_contains(property_rect, point));
        let export_hover_changed = !over_topmost
            && self.update_export_picker_hover(
                x,
                y,
                self.last_viewport_w,
                self.last_viewport_h,
                property_panel,
            );
        if over_export_popup {
            let below_changed = self.clear_chat_and_lower_hover();
            return Some(export_hover_changed || below_changed);
        }
        if export_hover_changed {
            if chat_or_picker_owns_point {
                ctx.upper_hover_changed = true;
            } else {
                return Some(true);
            }
        }
        // Effects "+" add-menu row hover (no-op when closed).
        let over_effect_popup = !over_topmost
            && property_panel
                .is_some_and(|panel| panel.effect_add_menu_contains(property_rect, point));
        let effect_hover_changed = !over_topmost
            && self.update_effect_add_menu_hover(
                x,
                y,
                self.last_viewport_w,
                self.last_viewport_h,
                property_panel,
            );
        if over_effect_popup {
            let below_changed = self.clear_chat_and_lower_hover();
            return Some(effect_hover_changed || below_changed);
        }
        if effect_hover_changed {
            if chat_or_picker_owns_point {
                ctx.upper_hover_changed = true;
            } else {
                return Some(true);
            }
        }
        // Layer / mask / per-fill blend dropdown hover. Blend choices use a
        // compact two-column popup, but the SelectState still stores the
        // flattened row index so paint and both hosts remain identical.
        let over_compositing_popup = !over_topmost
            && self.editor_state.editor_ui.compositing_picker.open
            && property_panel
                .is_some_and(|panel| panel.compositing_picker_contains(property_rect, point));
        let new_compositing_hover = (!over_topmost
            && self.editor_state.editor_ui.compositing_picker.open)
            .then(|| {
                property_panel
                    .and_then(|panel| panel.compositing_picker_row_at(property_rect, point))
            })
            .flatten();
        let compositing_hover_changed =
            new_compositing_hover != self.editor_state.editor_ui.compositing_picker.hover;
        if compositing_hover_changed {
            self.editor_state.editor_ui.compositing_picker.hover = new_compositing_hover;
            self.mark_dirty();
        }
        if over_compositing_popup {
            let below_changed = self.clear_chat_and_lower_hover();
            return Some(compositing_hover_changed || below_changed);
        }
        if compositing_hover_changed {
            if chat_or_picker_owns_point {
                ctx.upper_hover_changed = true;
            } else {
                return Some(true);
            }
        }
        // Interactions Navigate/Back/Remove popover row hover (no-op
        // when closed).
        let over_interaction_popup = !over_topmost
            && property_panel.is_some_and(|panel| {
                !matches!(
                    panel.interaction_menu_hit(property_rect, point),
                    op_editor_ui::widgets::InteractionMenuHit::Outside
                )
            });
        let interaction_hover_changed = !over_topmost
            && self.update_interaction_menu_hover(
                x,
                y,
                self.last_viewport_w,
                self.last_viewport_h,
                property_panel,
            );
        if over_interaction_popup {
            let below_changed = self.clear_chat_and_lower_hover();
            return Some(interaction_hover_changed || below_changed);
        }
        if interaction_hover_changed {
            if chat_or_picker_owns_point {
                ctx.upper_hover_changed = true;
            } else {
                return Some(true);
            }
        }
        // Fill / stroke colour-variable popup row hover (no-op when
        // closed). Same press order as `press_property_overlay_tiers`:
        // this popup sits above the padding / font popovers below it.
        let (over_color_variable_popup, color_variable_hover_changed) =
            hover_flow::color_variable_picker_hover(
                &mut self.editor_state,
                property_panel,
                property_rect,
                point,
            );
        if color_variable_hover_changed {
            self.mark_dirty();
        }
        if over_color_variable_popup {
            let below_changed = self.clear_chat_and_lower_hover();
            return Some(color_variable_hover_changed || below_changed);
        }
        if color_variable_hover_changed {
            if chat_or_picker_owns_point {
                ctx.upper_hover_changed = true;
            } else {
                return Some(true);
            }
        }
        // Padding-mode gear popover row hover (no-op when closed).
        let over_padding_or_stroke_popup = property_panel.is_some_and(|panel| {
            (self.editor_state.editor_ui.padding_mode_popover_open
                && panel.padding_mode_popover_contains(property_rect, point))
                || (self.editor_state.editor_ui.stroke_mode_popover_open
                    && panel.stroke_mode_popover_contains(property_rect, point))
        });
        let padding_hover_changed = self.update_padding_mode_popover_hover(x, y, property_panel);
        if over_padding_or_stroke_popup {
            let below_changed = self.clear_chat_and_lower_hover();
            return Some(padding_hover_changed || below_changed);
        }
        if padding_hover_changed {
            if chat_or_picker_owns_point {
                ctx.upper_hover_changed = true;
            } else {
                return Some(true);
            }
        }
        // Font-weight dropdown row hover (no-op when closed).
        let over_font_weight_popup = property_panel
            .is_some_and(|panel| panel.font_weight_picker_contains(property_rect, point));
        let font_weight_hover_changed = self.update_font_weight_picker_hover(x, y, property_panel);
        if over_font_weight_popup {
            let below_changed = self.clear_chat_and_lower_hover();
            return Some(font_weight_hover_changed || below_changed);
        }
        if font_weight_hover_changed {
            if chat_or_picker_owns_point {
                ctx.upper_hover_changed = true;
            } else {
                return Some(true);
            }
        }
        // Font-family picker entry hover (no-op when closed).
        let (over_font_picker, font_picker_hover_changed) =
            self.update_font_picker_hover(x, y, property_panel);
        if over_font_picker {
            let below_changed = self.clear_chat_and_lower_hover();
            return Some(font_picker_hover_changed || below_changed);
        }
        if font_picker_hover_changed {
            if chat_or_picker_owns_point {
                ctx.upper_hover_changed = true;
            } else {
                return Some(true);
            }
        }
        None
    }

    /// StatusBar, align toolbar, and the chat model picker — the last
    /// surfaces with first refusal over the chat itself.
    /// `None` — none of them consumed the move.
    pub(in crate::widget_host) fn cursor_move_status_align_picker_tiers(
        &mut self,
        ctx: &mut CursorMoveCtx,
    ) -> Option<bool> {
        let (x, y) = (ctx.x, ctx.y);
        let point = ctx.point;
        let over_topmost = ctx.over_topmost;
        let chat_or_picker_owns_point = ctx.chat_or_picker_owns_point;
        // StatusBar paints and presses above Chat. Preserve its hover/cursor
        // ownership before the model picker globally truncates lower layers.
        if let Some(status_rect) = self.status_bar_rect(self.last_viewport_w, self.last_viewport_h)
        {
            let point = Point2D::new(x, y);
            let over_status = !over_topmost && status_rect.contains(point);
            let new_hover = if over_topmost {
                None
            } else {
                op_editor_ui::widgets::StatusBar::for_editor(&self.editor_state)
                    .control_at(status_rect, point)
            };
            let status_hover_changed = new_hover != self.editor_state.editor_ui.statusbar_hover;
            if status_hover_changed {
                self.editor_state.editor_ui.statusbar_hover = new_hover;
                self.mark_dirty();
            }
            if over_status {
                let below_changed = self.clear_chat_and_lower_hover();
                return Some(status_hover_changed || below_changed);
            }
            if status_hover_changed {
                if chat_or_picker_owns_point {
                    ctx.upper_hover_changed = true;
                } else {
                    return Some(true);
                }
            }
        }
        // AlignToolbar is painted after Chat, so its entire card (including
        // gaps between buttons) receives first refusal before either the model
        // picker or the ordinary Chat surface truncates lower hover dispatch.
        let late_pointer_capture_active = self.rotate_drag.is_some()
            || self.handle_drag.is_some()
            || self.create_drag.is_some()
            || self.path_anchor_drag.is_some()
            || self.arc_handle_drag.is_some()
            || self.marquee_drag.is_some()
            || self.layer_drag.is_some()
            || self.panel_resize.is_some()
            || self.chat_resize.is_some()
            || self.chat_drag.is_some()
            || self.drag.is_some();
        if !late_pointer_capture_active && !self.editor_state.editor_ui.touch_chrome() {
            let over_align = !over_topmost
                && self
                    .align_toolbar_rect(self.last_viewport_w, self.last_viewport_h)
                    .is_some_and(|rect| rect.contains(point));
            let new_align_hover = if !over_topmost {
                self.align_toolbar_hit(x, y, self.last_viewport_w, self.last_viewport_h)
            } else {
                None
            };
            let align_hover_changed =
                new_align_hover != self.editor_state.editor_ui.align_toolbar_hover;
            if align_hover_changed {
                self.editor_state.editor_ui.align_toolbar_hover = new_align_hover;
                self.mark_dirty();
            }
            if over_align {
                let below_changed = self.clear_chat_and_lower_hover();
                return Some(align_hover_changed || below_changed);
            }
            if align_hover_changed {
                if chat_or_picker_owns_point {
                    ctx.upper_hover_changed = true;
                } else {
                    return Some(true);
                }
            }
        }
        // The chat model picker is modal only below overlays painted and
        // pressed above the chat (Git, context menus, and property popovers).
        // Once those surfaces have had first refusal, update the picker row,
        // clear stale lower highlights, and stop before constructing the base
        // chat/property/canvas probes.
        if self.editor_state.editor_ui.chat_model_picker.open {
            use op_editor_ui::widgets::ai_chat_model_picker::{model_picker_hit, SelectHit};
            let Some(picker) =
                self.chat_model_picker_rect(self.last_viewport_w, self.last_viewport_h)
            else {
                // A collapsed/hidden chat or a viewport too small to lay it
                // out must not leave an invisible modal hover lock behind.
                self.editor_state.editor_ui.close_chat_model_picker();
                self.mark_dirty();
                return Some(true);
            };
            let new_hover = match model_picker_hit(
                &self.editor_state.editor_ui.chat_model_picker,
                picker,
                Point2D::new(x, y),
                &self.editor_state.chat.available_models,
                self.editor_state.editor_ui.chat_model_picker_input.text(),
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .has_discovery_ready_builtin_agent(),
            ) {
                SelectHit::Row(index) => Some(index),
                SelectHit::Inside | SelectHit::Outside => None,
            };
            let hover_changed = new_hover != self.editor_state.editor_ui.chat_model_picker.hover;
            if hover_changed {
                self.editor_state.editor_ui.chat_model_picker.hover = new_hover;
                self.mark_dirty();
            }
            let lower_changed = self.clear_hover_below_chat_model_picker();
            return Some(ctx.upper_hover_changed || hover_changed || lower_changed);
        }
        None
    }
}
