//! `apply_press` tiers 6-8 — the property-panel popover band, the
//! floating panel dispatch band, and the PropertyPanel input row itself.
//!
//! Every popover in tier 6 follows the same contract: a row hit applies, a
//! press on the popup chrome is swallowed, and the first outside press
//! dismisses and is swallowed. All of them are gated on `!in_git_panel`
//! because the Git panel paints above the rail.

use super::press_ctx::PressCtx;
use super::{CodeSelectionDragState, WidgetHostNative};
use op_editor_core::codegen::CodeSelection;
use op_editor_ui::widgets::press_flow;
use op_editor_ui::widgets::PropertyPanel;
use op_editor_ui::Point2D;

impl WidgetHostNative {
    /// `None` — no property popover claimed the press.
    pub(in crate::widget_host) fn press_property_overlay_tiers(
        &mut self,
        ctx: &PressCtx,
        allow_touch_panel_defer: bool,
    ) -> Option<bool> {
        let (x, y) = (ctx.x, ctx.y);
        let viewport_width = ctx.viewport_width;
        let viewport_height = ctx.viewport_height;
        let in_git_panel = ctx.in_git_panel;
        // 0c0. Fill-type picker — outside-click dismiss.
        if self.editor_state.editor_ui.fill_type_picker.open && !in_git_panel {
            self.refresh_layout_scene();
            let property_rect = self.property_rect(viewport_width, viewport_height);
            let press = press_flow::press_fill_type_picker_in_rect(
                &mut self.editor_state,
                property_rect,
                Point2D::new(x, y),
            );
            return Some(self.finish_property_overlay_press(press));
        }

        // 0c0a. Layer / mask / fill-blend compositing picker. The popup is
        // painted over the inspector body, so it owns both its rows and its
        // padded chrome; the first outside press dismisses and is swallowed.
        if self.editor_state.editor_ui.compositing_picker.open && !in_git_panel {
            self.refresh_layout_scene();
            let property_rect = self.property_rect(viewport_width, viewport_height);
            let press = press_flow::press_compositing_picker_in_rect(
                &mut self.editor_state,
                property_rect,
                Point2D::new(x, y),
            );
            return Some(self.finish_property_overlay_press(press));
        }

        // 0c1. Effects "+" add-menu — outside-click dismiss.
        if self.editor_state.editor_ui.effect_add_picker_open && !in_git_panel {
            self.refresh_layout_scene();
            let property_rect = self.property_rect(viewport_width, viewport_height);
            let press = press_flow::press_effect_add_menu_in_rect(
                &mut self.editor_state,
                property_rect,
                Point2D::new(x, y),
            );
            return Some(self.finish_property_overlay_press(press));
        }

        // 0c1a. Interactions Navigate/Back/Remove popover — outside-click
        // dismiss.
        if self.editor_state.editor_ui.interaction_menu_open && !in_git_panel {
            self.refresh_layout_scene();
            if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
                let property_rect = self.property_rect(viewport_width, viewport_height);
                match panel.interaction_menu_hit(property_rect, Point2D::new(x, y)) {
                    op_editor_ui::widgets::InteractionMenuHit::Row(action) => {
                        self.apply_property_action(action);
                        return Some(true);
                    }
                    op_editor_ui::widgets::InteractionMenuHit::Inside => return Some(true),
                    op_editor_ui::widgets::InteractionMenuHit::Outside => {}
                }
            }
            self.editor_state.editor_ui.close_interaction_menu();
            self.mark_dirty();
            return Some(true);
        }

        // 0c0a0. Fill/stroke colour-variable picker — outside-click dismiss.
        if self
            .editor_state
            .editor_ui
            .property_color_variable_picker_open
            .is_some()
            && !in_git_panel
        {
            self.refresh_layout_scene();
            let property_rect = self.property_rect(viewport_width, viewport_height);
            let press = press_flow::press_color_variable_picker_in_rect(
                &mut self.editor_state,
                property_rect,
                Point2D::new(x, y),
            );
            return Some(self.finish_property_overlay_press(press));
        }

        // 0c0a0a. Image-node Search / Generate popovers — overlay
        // controls win; outside clicks dismiss.
        if !in_git_panel
            && self.dismiss_image_popovers_on_press(x, y, viewport_width, viewport_height)
        {
            return Some(true);
        }

        // 0c0a1. Text font-family picker — a touch inside the popup is
        // release-delayed so a row press can promote to one-finger scrolling.
        // Mouse presses and touch presses outside the popup keep the existing
        // immediate select / outside-dismiss behaviour.
        if allow_touch_panel_defer && !in_git_panel && self.begin_font_picker_touch_gesture(ctx) {
            return Some(true);
        }
        // Text font-family picker — outside-click dismiss.
        if !in_git_panel && self.dismiss_font_picker_on_press(x, y, viewport_width, viewport_height)
        {
            return Some(true);
        }

        // 0c0a2. Text font-weight picker — outside-click dismiss.
        if !in_git_panel
            && self.dismiss_font_weight_picker_on_press(x, y, viewport_width, viewport_height)
        {
            return Some(true);
        }

        // 0c0a3. Padding mode-selector popover — outside-click dismiss.
        if !in_git_panel
            && self.dismiss_padding_mode_popover_on_press(x, y, viewport_width, viewport_height)
        {
            return Some(true);
        }

        // 0c0b. Export scale / format inline select popup —
        //       outside-click dismiss (`property_dispatch.rs`).
        if !in_git_panel
            && self.dismiss_export_picker_on_press(x, y, viewport_width, viewport_height)
        {
            return Some(true);
        }
        None
    }

    /// Chat model picker, theme-preset dropdown, VariablesPanel.
    /// `None` — none of them claimed the press.
    pub(in crate::widget_host) fn press_panel_dispatch_tiers(
        &mut self,
        ctx: &PressCtx,
    ) -> Option<bool> {
        let (x, y) = (ctx.x, ctx.y);
        let viewport_width = ctx.viewport_width;
        let viewport_height = ctx.viewport_height;
        let in_git_panel = ctx.in_git_panel;
        // The model dropdown is painted with the chat above the base rails
        // and can extend beyond the chat rect. Route its actual visible bounds
        // before Variables/Property/Layer dispatch, while leaving the Git and
        // property popovers painted above it in control of their overlap.
        if !in_git_panel
            && self.apply_chat_model_picker_overlay_press(x, y, viewport_width, viewport_height)
        {
            return Some(true);
        }

        // 0b0. Theme-preset dropdown (#20) — floats above the
        //       variables panel, so it must win before the panel's
        //       stub menu mapping (variables_preset_press.rs).
        if !in_git_panel
            && self.dispatch_variables_preset_press(x, y, viewport_width, viewport_height)
        {
            return Some(true);
        }

        // 0b1. VariablesPanel — tested before PropertyPanel.
        if !in_git_panel
            && self.dispatch_variables_panel_press(x, y, viewport_width, viewport_height)
        {
            return Some(true);
        }
        None
    }

    /// PropertyPanel code selection / action / input-row focus.
    /// `None` — the press was not inside the panel.
    pub(in crate::widget_host) fn press_property_panel_tier(
        &mut self,
        ctx: &PressCtx,
        allow_touch_panel_defer: bool,
    ) -> Option<bool> {
        let (x, y) = (ctx.x, ctx.y);
        let viewport_width = ctx.viewport_width;
        let viewport_height = ctx.viewport_height;
        let in_git_panel = ctx.in_git_panel;
        let inspector_visible = !self.editor_state.editor_ui.touch_chrome()
            || self.editor_state.editor_ui.expanded_touch_layout()
            || self.editor_state.editor_ui.mobile_sheet
                == Some(op_editor_core::size_class::MobileSheetKind::Properties);
        if !inspector_visible {
            return None;
        }
        // Touch inspector close hides the surface without destroying the
        // selection. Selection and inspector visibility are separate state.
        if self.editor_state.editor_ui.touch_chrome()
            && !self.editor_state.editor_ui.expanded_touch_layout()
        {
            if let Some(panel) =
                PropertyPanel::for_selection_with_scene(&self.editor_state, &self.layout_scene)
            {
                let sheet = self.property_rect(ctx.viewport_width, ctx.viewport_height);
                let close = op_editor_ui::widgets::mobile_chrome::sheet_close_rect(sheet);
                if close.contains(Point2D::new(x, y)) {
                    self.cancel_native_touch_gestures();
                    self.dismiss_mobile_surface();
                    self.mark_dirty();
                    return Some(true);
                }
                let _ = panel;
            }
        }
        if allow_touch_panel_defer && self.begin_property_touch_gesture(ctx) {
            return Some(true);
        }
        // 0c. PropertyPanel input row.
        self.refresh_layout_scene();
        if let Some(panel) =
            PropertyPanel::for_selection_with_scene(&self.editor_state, &self.layout_scene)
                .filter(|_| !in_git_panel)
        {
            let property_rect = self.property_rect(viewport_width, viewport_height);
            if let Some(anchor) = self.code_text_offset_at_screen(x, y) {
                self.commit_property_focus_if_any();
                self.editor_state.codegen.code_selection = Some(CodeSelection {
                    anchor,
                    focus: anchor,
                });
                self.code_selection_drag = Some(CodeSelectionDragState { anchor });
                self.editor_state.chat.transcript_selection = None;
                self.editor_state.codegen.framework_hover = None;
                self.editor_state.codegen.action_hover = None;
                self.editor_state.chat.focused = false;
                self.mark_dirty();
                return Some(true);
            }
            // Button / checkbox click first (flex modes + size flags).
            let point = Point2D::new(x, y);
            if let Some(action) = panel.hit_test_action(property_rect, point) {
                self.editor_state.editor_ui.pressed_button =
                    if let op_editor_ui::widgets::PropertyPanelAction::Codegen(codegen_action) =
                        action
                    {
                        op_editor_ui::widgets::property_panel_code::codegen_hover_for_action(
                            codegen_action,
                        )
                        .map(op_editor_core::ButtonPressTarget::Codegen)
                    } else {
                        panel
                            .action_hover_index(property_rect, point)
                            .map(op_editor_core::ButtonPressTarget::PropertyPanel)
                    };
                self.commit_property_focus_if_any();
                if let op_editor_ui::widgets::PropertyPanelAction::OpenColorPicker(target) = action
                {
                    let _ = self
                        .editor_state
                        .open_color_picker(super::press_helpers::color_target(target), y);
                    self.mark_dirty();
                } else if let op_editor_ui::widgets::PropertyPanelAction::OpenFillColorPicker(
                    index,
                ) = action
                {
                    // Non-primary fill swatch — bind the picker to this
                    // fill so HSV writes back to `fills[index]`.
                    self.editor_state.editor_ui.close_color_variable_picker();
                    let _ = self.editor_state.open_color_picker_for_fill(
                        op_editor_core::ui_draft::ColorTarget::Fill,
                        index,
                        y,
                    );
                    self.mark_dirty();
                } else if let op_editor_ui::widgets::PropertyPanelAction::OpenEffectColorPicker(
                    index,
                ) = action
                {
                    // Anchor the picker at the clicked swatch so it
                    // pops next to the row, not at the panel top.
                    let _ = self.editor_state.open_color_picker(
                        op_editor_core::ui_draft::ColorTarget::EffectColor(index),
                        y,
                    );
                    self.mark_dirty();
                } else {
                    self.apply_property_action(action);
                }
                return Some(true);
            }
            if let Some(focus) = panel.hit_test(property_rect, Point2D::new(x, y)) {
                self.commit_property_focus_if_any();
                // Committing the previous W/H field can change layout. Rebuild
                // the scene before seeding the newly focused field so Fill/Hug
                // reads the concrete post-commit canvas size, not a stale one.
                self.refresh_layout_scene();
                let resolved_panel =
                    PropertyPanel::for_selection_with_scene(&self.editor_state, &self.layout_scene);
                let initial = resolved_panel
                    .as_ref()
                    .map(|panel| super::press_helpers::property_focus_initial(focus, panel))
                    .unwrap_or_default();
                // shell-core `PropertyFocus` → op-editor-core.
                self.editor_state.ui.property_focus = Some(focus);
                self.editor_state
                    .ui
                    .property_input
                    .set_text(initial.clone());
                self.editor_state.ui.property_input.touch(self.now_ms);
                self.editor_state.ui.property_input_draft = initial;
                // Caret starts at the end of the seeded draft.
                self.editor_state.ui.property_caret_pos =
                    self.editor_state.ui.property_input_draft.len();
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.editor_state.ui.property_draft_select_all = false;
                self.editor_state.chat.focused = false;
                self.reveal_property_keyboard_owner();
                self.mark_dirty();
                return Some(true);
            }
            if (property_rect).contains(point) {
                self.blur_text_inputs_on_blank_press();
                return Some(true);
            }
        }
        None
    }
}
