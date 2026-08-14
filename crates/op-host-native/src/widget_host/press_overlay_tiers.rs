//! `apply_press` tiers 1-2 — the top-most modal / overlay surfaces and the
//! menu + modal dropdown tier.
//!
//! Tier 1 runs before ANY other hit-test: a missing-font prompt, floating
//! panel, colour picker, modal Git popover, or an open context menu owns
//! the press outright. Tier 2 is the dropdown/modal band (shape picker,
//! file menu, export / figma / login / account, import, locale).
//!
//! Order within each helper is paint Z-order and is load-bearing.

use super::press_ctx::PressCtx;
use super::WidgetHostNative;
use op_editor_core::host_press_transitions as core_press;
use op_editor_ui::widgets::press_flow::{self, LocalePickerPress, OpenLayerMenuPress};
use op_editor_ui::widgets::{CollabPanel, ImportMenu, ImportMenuChoice};
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
    /// `None` — no top-most overlay claimed the press.
    pub(in crate::widget_host) fn press_topmost_overlay_tiers(
        &mut self,
        ctx: &PressCtx,
    ) -> Option<bool> {
        if self.preview_slideshow_active() && self.editor_state.editor_ui.touch_chrome() {
            return None;
        }
        let (x, y) = (ctx.x, ctx.y);
        let viewport_width = ctx.viewport_width;
        let viewport_height = ctx.viewport_height;
        // Missing-font prompt is the absolute top-most modal. Outside presses
        // are swallowed; only its explicit dismiss button closes it.
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
        // Transient notice banner — painted just under the missing-font modal
        // and above the diagnostics notice, so it hit-tests between the two
        // (hit-test is reverse paint order). Non-modal: only presses on the
        // banner itself are consumed, and the shared flow re-checks that the
        // toast has not expired since the cached rect was painted.
        if op_editor_ui::widgets::editor_toast_flow::press(
            &mut self.editor_state,
            self.toast_rect,
            Point2D::new(x, y),
            self.now_ms,
        ) {
            self.mark_dirty();
            return Some(true);
        }
        // Post-import HTML diagnostics — a non-modal notice painted just
        // under the missing-font modal, so it hit-tests right after it. It
        // consumes ONLY presses inside its own card; everything else falls
        // through to the tiers below (see `press_overlay_tiers` doc).
        if self.dispatch_html_import_diagnostics_press(x, y, viewport_width, viewport_height) {
            self.close_image_popovers_for_higher_overlay();
            return Some(true);
        }
        // Floating Design-MD panel — painted top-most (`paint.rs`
        // §12), so it hit-tests first: a click on its rect is the
        // panel's before any lower layer can claim it (dispatch in
        // `design_md_press.rs`).
        if self.dispatch_design_md_press(x, y, viewport_width, viewport_height) {
            self.close_image_popovers_for_higher_overlay();
            return Some(true);
        }
        if self.dispatch_icon_picker_press(x, y, viewport_width, viewport_height) {
            self.close_image_popovers_for_higher_overlay();
            return Some(true);
        }
        if self.dispatch_scene_template_press(x, y, viewport_width, viewport_height) {
            return Some(true);
        }
        if self.dispatch_prompt_center_press(x, y, viewport_width, viewport_height) {
            self.close_image_popovers_for_higher_overlay();
            return Some(true);
        }
        // Floating Component-Browser panel — painted just under the
        // Design-MD panel; hit-tests right after it.
        if self.dispatch_component_browser_press(x, y, viewport_width, viewport_height) {
            self.close_image_popovers_for_higher_overlay();
            return Some(true);
        }
        if self.editor_state.editor_ui.agent_settings_open
            && self.dispatch_agent_settings_press(x, y, viewport_width, viewport_height)
        {
            self.close_image_popovers_for_higher_overlay();
            return Some(true);
        }
        // 0-color. Color picker overlay — top-most when open
        //          (dispatch in `color_picker_press.rs`).
        if self.dispatch_color_picker_press(x, y, viewport_width, viewport_height) {
            self.close_image_popovers_for_higher_overlay();
            return Some(true);
        }

        // 0-git-modal. While a Git ready-state header popover (branch
        // picker / overflow / its subview) is open it is MODAL: route
        // every press to the Git panel first so a click anywhere
        // outside the popover dismisses it. The popover extends past the
        // panel rect, so the rail / top-bar / canvas blocks below cannot
        // be relied on to forward an outside click — `hit_test` returns
        // `DismissPopover` for any outside-popover point.
        //
        // IMPORTANT: fall through on `false`. If the popover flags are
        // stale (set while the panel was ready, but it has since left
        // the ready state — gone dirty / merging / loading, so
        // `hit_test`'s ready-gated popover capture no longer fires),
        // `dispatch_git_panel_press` returns `false` for an outside
        // click. Returning that directly would dead-end EVERY press; so
        // only consume when it actually handled the click.
        {
            let gp = &self.editor_state.editor_ui.git_panel;
            if gp.open
                && (gp.branch_picker_open || gp.overflow_open)
                && self.dispatch_git_panel_press(x, y, viewport_width, viewport_height)
            {
                self.close_image_popovers_for_higher_overlay();
                return Some(true);
            }
        }

        // Path-anchor context menu — a row hit dispatches + consumes;
        // an outside press closes it and FALLS THROUGH (TS parity:
        // the canvas mousedown still routes). `pen_press.rs`.
        if self.dispatch_path_anchor_menu_press(x, y) {
            return Some(true);
        }

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

    /// `None` — no dropdown / modal claimed the press.
    pub(in crate::widget_host) fn press_menu_modal_tiers(
        &mut self,
        ctx: &PressCtx,
    ) -> Option<bool> {
        if self.preview_slideshow_active() && self.editor_state.editor_ui.touch_chrome() {
            return None;
        }
        let (x, y) = (ctx.x, ctx.y);
        let viewport_width = ctx.viewport_width;
        let viewport_height = ctx.viewport_height;
        // Shared collaboration popover. Like the account/locale menus it is
        // modal within the dropdown tier: a press outside closes and is
        // swallowed, while a row/button only queues a runtime-owned action.
        if self.editor_state.editor_ui.collab.panel.open
            && !((self.editor_state.editor_ui.account_ui_available
                || self.editor_state.editor_ui.touch_chrome())
                && self.editor_state.editor_ui.login_modal_open)
        {
            if let Some(panel) = CollabPanel::for_editor_ui(&self.editor_state.editor_ui) {
                let panel_rect =
                    op_editor_ui::widgets::touch_overlay_geometry::collaboration_panel_rect(
                        &self.editor_state,
                        &panel,
                        viewport_width,
                        viewport_height,
                    );
                let point = Point2D::new(x, y);
                if let Some(hit) = panel.hit_test(panel_rect, point) {
                    match hit {
                        op_editor_ui::widgets::CollabPanelHit::CopyInvite(invite)
                        | op_editor_ui::widgets::CollabPanelHit::CopyShareEndpoint(invite) => {
                            self.editor_state.chat.queue_copy_text(invite);
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
                    self.editor_state.editor_ui.collab.panel.hover = None;
                    self.blur_text_inputs_on_blank_press();
                }
                self.mark_dirty();
                return Some(true);
            }
        }
        // 0ab. Shape picker overlay.
        if self.dispatch_shape_picker_press(x, y, viewport_width, viewport_height) {
            return Some(true);
        }

        if self.editor_state.editor_ui.file_menu_open {
            self.close_image_popovers_for_higher_overlay();
            self.dispatch_file_menu_press(x, y, viewport_width);
            return Some(true);
        }
        // Export quick menu — same tier as the file menu it shortcuts. It
        // must sit above the TopBar tier so a second press on the download
        // button closes it instead of re-toggling it open.
        if self.editor_state.editor_ui.export_quick_menu_open {
            self.close_image_popovers_for_higher_overlay();
            self.dispatch_export_quick_menu_press(x, y, viewport_width);
            return Some(true);
        }
        // The slides rail's export dropdown, on the same rung and for the
        // same reason. The rail's own press tier sits far BELOW the TopBar,
        // so without this rung a click on the top bar would leave the menu
        // open behind it; here it is dismissed like any other dropdown.
        // Routed through the rail's ordinary entry point rather than a
        // second dispatcher — `slides_panel_flow::press` already answers
        // rows, chrome and dismiss, and claims every press while open.
        if self.editor_state.editor_ui.slides_panel.export_menu_open
            && self.slides_panel_press(x, y, viewport_width, viewport_height)
        {
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
        if (self.editor_state.editor_ui.account_ui_available
            || self.editor_state.editor_ui.touch_chrome())
            && self.editor_state.editor_ui.login_modal_open
        {
            self.close_image_popovers_for_higher_overlay();
            self.dispatch_login_modal_press(x, y, viewport_width, viewport_height);
            return Some(true);
        }

        // 0a'. Account dropdown — anchored under the TopBar avatar
        // button; must hit-test before the TopBar's own block so a
        // re-click on the avatar closes rather than re-toggling.
        if self.editor_state.editor_ui.account_ui_available
            && self.editor_state.editor_ui.account_menu_open
        {
            self.close_image_popovers_for_higher_overlay();
            self.dispatch_account_menu_press(x, y, viewport_width, viewport_height);
            return Some(true);
        }

        // 0a0. Import dropdown — same overlay tier as the locale picker.
        if self.editor_state.editor_ui.import_menu_open {
            self.refresh_layout_scene();
            let (anchor, viewport) = self.import_menu_anchor(viewport_width, viewport_height);
            let menu = ImportMenu::for_editor_ui(&self.editor_state.editor_ui);
            let choice = menu.choice_at(anchor, viewport, Point2D::new(x, y));
            let hit = menu.hit(anchor, viewport, Point2D::new(x, y));
            if matches!(hit, op_editor_ui::widgets::import_menu::SelectHit::Inside) {
                return Some(true);
            }
            self.close_import_menu();
            match choice {
                Some(ImportMenuChoice::Figma) => {
                    self.apply_open_figma_import();
                }
                Some(ImportMenuChoice::Html) => {
                    self.apply_open_html_import();
                }
                None => {
                    // Silent outside-close is a blank press — blur inputs too.
                    self.blur_text_inputs_on_blank_press();
                }
            }
            self.mark_dirty();
            return Some(true);
        }

        // 0a. Locale picker overlay — top-most when open.
        if self.editor_state.editor_ui.locale_picker.open {
            self.refresh_layout_scene();
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
}
