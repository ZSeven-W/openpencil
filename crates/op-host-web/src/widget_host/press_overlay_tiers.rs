//! Web `apply_press` tiers 1-3 — top-most overlays, the import + locale
//! dropdown tier, and the shape-picker / modal band.
//!
//! Tier 1 runs before any other hit-test: the missing-font prompt,
//! floating panels, colour picker, path-anchor menu, and layer context
//! menu own the press outright. Order within each helper is paint Z-order
//! and is load-bearing.

use super::press_ctx::PressCtx;
use super::WidgetHost;
use op_editor_core::host_press_transitions as core_press;
use op_editor_ui::widgets::press_flow::{self, LocalePickerPress, OpenLayerMenuPress};
use op_editor_ui::{Point2D, Rect};

impl WidgetHost {
    /// `None` — no top-most overlay claimed the press.
    pub(in crate::widget_host) fn press_topmost_overlay_tiers(
        &mut self,
        ctx: &PressCtx,
    ) -> Option<bool> {
        let (x, y) = (ctx.x, ctx.y);
        let viewport_width = ctx.viewport_width;
        let viewport_height = ctx.viewport_height;
        let missing_fonts_rect =
            op_editor_ui::widgets::MissingFontsPanel::for_editor(&self.editor_state)
                .map(|panel| panel.rect(viewport_width, viewport_height));
        if let Some(panel_rect) = missing_fonts_rect {
            if self.dispatch_missing_fonts_press(
                panel_rect,
                Rect {
                    origin: Point2D::new(0.0, 0.0),
                    size: Point2D::new(viewport_width, viewport_height),
                },
                Point2D::new(x, y),
            ) {
                self.close_image_popovers_for_higher_overlay();
                return Some(true);
            }
        }
        // Floating Design-MD panel — painted top-most, so it
        // hit-tests first: a click on its rect is the panel's before
        // any lower layer can claim it (mirrors native press order).
        if self.dispatch_design_md_press(x, y, viewport_width, viewport_height) {
            self.close_image_popovers_for_higher_overlay();
            return Some(true);
        }
        if self.dispatch_icon_picker_press(x, y, viewport_width, viewport_height) {
            self.close_image_popovers_for_higher_overlay();
            return Some(true);
        }
        // Floating Component-Browser panel — painted just under the
        // Design-MD panel; hit-tests right after it. A consumed press
        // may queue an insert — drain it against this viewport (web
        // has no per-frame runner drain like the desktop loop).
        if self.dispatch_component_browser_press(x, y, viewport_width, viewport_height) {
            self.close_image_popovers_for_higher_overlay();
            let _ = self.drain_component_browser_insert(viewport_width, viewport_height);
            return Some(true);
        }
        if self.editor_state.editor_ui.agent_settings_open
            && self.dispatch_agent_settings_press(x, y, viewport_width, viewport_height)
        {
            self.close_image_popovers_for_higher_overlay();
            return Some(true);
        }
        // Colour-picker overlay — top-most when open. Falls through
        // on an outside click (the picker closes as a side effect).
        if self.dispatch_color_picker_press(x, y, viewport_width, viewport_height) {
            self.close_image_popovers_for_higher_overlay();
            return Some(true);
        }
        if self.dispatch_path_anchor_menu_press(x, y) {
            return Some(true);
        }
        // 0. Layer context menu — top-most overlay when open.
        if let Some(press) =
            press_flow::press_open_layer_context_menu(&self.editor_state, Point2D::new(x, y))
        {
            match press {
                OpenLayerMenuPress::Action { action, target } => {
                    self.dispatch_layer_context_action(action, target);
                    self.editor_state.editor_ui.layer_context_menu = None;
                    self.mark_dirty();
                }
                OpenLayerMenuPress::Swallow => {}
                OpenLayerMenuPress::Outside => {
                    // Dismissing the menu on a miss is a blank press — blur
                    // every text input along with it.
                    self.blur_text_inputs_on_blank_press();
                    self.editor_state.editor_ui.layer_context_menu = None;
                    self.mark_dirty();
                }
            }
            return Some(true);
        }
        None
    }

    /// Import dropdown + locale picker. `None` — neither claimed it.
    pub(in crate::widget_host) fn press_import_locale_tiers(
        &mut self,
        ctx: &PressCtx,
    ) -> Option<bool> {
        let (x, y) = (ctx.x, ctx.y);
        let viewport_width = ctx.viewport_width;
        let viewport_height = ctx.viewport_height;
        if self.editor_state.editor_ui.collab.panel.open {
            let top_bar_rect = Rect::xywh(
                0.0,
                0.0,
                viewport_width,
                op_editor_ui::widgets::TOP_BAR_HEIGHT,
            );
            let top_bar =
                op_editor_ui::widgets::TopBar::for_editor_ui(&self.editor_state.editor_ui)
                    .with_traffic_controls(false);
            if let Some(panel) =
                op_editor_ui::widgets::CollabPanel::for_editor_ui(&self.editor_state.editor_ui)
            {
                let panel_rect = panel.rect_at(
                    top_bar.collaboration_chip_rect_estimated(top_bar_rect),
                    Rect::xywh(0.0, 0.0, viewport_width, viewport_height),
                );
                if let Some(hit) = panel.hit_test(panel_rect, Point2D::new(x, y)) {
                    match hit {
                        op_editor_ui::widgets::CollabPanelHit::CopyShareEndpoint(endpoint) => {
                            self.host_copy_text(&endpoint);
                        }
                        hit => {
                            let _ = op_editor_ui::widgets::collab_ui::apply_panel_hit(
                                &mut self.editor_state.editor_ui,
                                hit,
                            );
                        }
                    }
                } else {
                    self.editor_state.editor_ui.collab.panel.open = false;
                    self.editor_state
                        .editor_ui
                        .collab
                        .panel
                        .join_address_focused = false;
                    self.blur_text_inputs_on_blank_press();
                }
                self.mark_dirty();
                return Some(true);
            }
        }
        // 0a0. Import dropdown — same overlay tier as the locale picker.
        if self.editor_state.editor_ui.import_menu_open {
            use op_editor_ui::widgets::{ImportMenu, ImportMenuChoice};
            let (anchor, menu_viewport) = self.import_menu_anchor(viewport_width, viewport_height);
            let menu = ImportMenu::for_editor_ui(&self.editor_state.editor_ui);
            let point = Point2D::new(x, y);
            if matches!(
                menu.hit(anchor, menu_viewport, point),
                op_editor_ui::widgets::import_menu::SelectHit::Inside
            ) {
                return Some(true);
            }
            let choice = menu.choice_at(anchor, menu_viewport, point);
            self.close_import_menu();
            match choice {
                Some(ImportMenuChoice::Figma) => {
                    self.apply_open_import(op_editor_core::figma_import_state::ImportSource::Figma);
                }
                Some(ImportMenuChoice::Html) => {
                    self.apply_open_import(op_editor_core::figma_import_state::ImportSource::Html);
                }
                None => {
                    self.blur_text_inputs_on_blank_press();
                }
            }
            self.mark_dirty();
            return Some(true);
        }

        // 0a. Locale picker overlay — top-most when open. Row hit
        //     sets locale + closes; ANY other hit (including the
        //     Globe button itself) closes the picker AND swallows
        //     the click so the same press doesn't re-toggle open.
        if self.editor_state.editor_ui.locale_picker.open {
            let panel_rect = self.locale_picker_rect(viewport_width);
            match press_flow::press_locale_picker(
                &mut self.editor_state,
                panel_rect,
                Point2D::new(x, y),
            ) {
                LocalePickerPress::Swallow => return Some(true),
                LocalePickerPress::Selected => {
                    self.mark_dirty();
                    return Some(true);
                }
                LocalePickerPress::Outside => {
                    // Silent outside-close is a blank press — blur inputs too.
                    self.blur_text_inputs_on_blank_press();
                    core_press::close_locale_picker(&mut self.editor_state.editor_ui);
                    self.mark_dirty();
                    return Some(true);
                }
            }
        }
        None
    }

    /// Shape picker, file menu, export / figma modals, sign-in modal and
    /// account dropdown. `None` — none of them claimed the press.
    pub(in crate::widget_host) fn press_menu_modal_tiers(
        &mut self,
        ctx: &PressCtx,
    ) -> Option<bool> {
        let (x, y) = (ctx.x, ctx.y);
        let viewport_width = ctx.viewport_width;
        let viewport_height = ctx.viewport_height;
        // 0aa. Shape picker overlay (native press order: before the
        //      file-menu / export / figma modal blocks).
        if self.dispatch_shape_picker_press(x, y, viewport_width, viewport_height) {
            return Some(true);
        }
        if self.editor_state.editor_ui.file_menu_open {
            self.close_image_popovers_for_higher_overlay();
            self.dispatch_file_menu_press(x, y, viewport_width);
            return Some(true);
        }
        if self.editor_state.editor_ui.export_dialog_open {
            self.close_image_popovers_for_higher_overlay();
            self.dispatch_export_dialog_press(x, y, viewport_width, viewport_height);
            return Some(true);
        }
        if self.editor_state.editor_ui.figma_import_open {
            self.close_image_popovers_for_higher_overlay();
            self.dispatch_figma_import_press(x, y, viewport_width, viewport_height);
            return Some(true);
        }
        // Sign-in modal / account dropdown — same overlay tier as native
        // §0a/§0a' (before the TopBar so a re-click on the avatar closes
        // instead of re-toggling).
        if self.editor_state.editor_ui.account_ui_available
            && self.editor_state.editor_ui.login_modal_open
        {
            self.close_image_popovers_for_higher_overlay();
            self.dispatch_login_modal_press(x, y, viewport_width, viewport_height);
            return Some(true);
        }
        if self.editor_state.editor_ui.account_ui_available
            && self.editor_state.editor_ui.account_menu_open
        {
            self.close_image_popovers_for_higher_overlay();
            self.dispatch_account_menu_press(x, y, viewport_width, viewport_height);
            return Some(true);
        }
        None
    }
}
