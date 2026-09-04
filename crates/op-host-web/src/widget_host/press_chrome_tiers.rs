//! Web `apply_press` tiers 4-6 — the rail-overlay band, the floating
//! VariablesPanel band, and the TopBar.
//!
//! Tier 4 covers the image-fill popover, the StatusBar controls, and the
//! slice of the chat model picker that can lift above the TopBar. Tier 5
//! is gated on `over_chat_model_picker` so the picker card keeps its
//! overlap with the panels beneath it.

use super::press_ctx::PressCtx;
use super::WidgetHost;
use op_editor_ui::widgets::press_flow::{self, TopBarPress};
use op_editor_ui::widgets::{PropertyPanel, TopBarHit, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

impl WidgetHost {
    /// `None` — no rail overlay claimed the press.
    pub(in crate::widget_host) fn press_rail_overlay_tiers(
        &mut self,
        ctx: &PressCtx,
    ) -> Option<bool> {
        let (x, y) = (ctx.x, ctx.y);
        let viewport_width = ctx.viewport_width;
        let viewport_height = ctx.viewport_height;
        // 0a1. Image-fill popover. It is painted in the late property-overlay
        // pass, above VariablesPanel, chat, StatusBar, marquee, and ordinary
        // rail content. The modal/menu overlays above already had first
        // refusal, while an outside press still commits and closes the popup.
        if self.editor_state.editor_ui.image_fill_popover_open {
            if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
                let property_rect = Rect {
                    origin: Point2D::new(
                        viewport_width - self.editor_state.editor_ui.property_panel_width,
                        TOP_BAR_HEIGHT,
                    ),
                    size: Point2D::new(
                        self.editor_state.editor_ui.property_panel_width,
                        (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                    ),
                };
                let point = Point2D::new(x, y);
                if panel.image_fill_popover_contains(property_rect, point) {
                    if let Some(focus) = panel.image_fill_popover_input_at(property_rect, point) {
                        return Some(self.focus_property_input_from_press(
                            focus,
                            property_rect,
                            point,
                        ));
                    }
                    if let Some(action) = panel.hit_test_action(property_rect, point) {
                        self.apply_property_action(action);
                    }
                    return Some(true);
                }
            }
            self.commit_image_tile_scale_focus_if_any();
            self.editor_state.editor_ui.image_fill_popover_open = false;
            self.mark_dirty();
            return Some(true);
        }

        // StatusBar controls — Search frames content, `[-]` / `[+]`
        // step the zoom. The bar paints above the canvas / toolbar but below
        // the late PropertyPanel overlay pass.
        if let Some(r) = self.status_bar_rect(viewport_width, viewport_height) {
            use op_editor_core::StatusBarButton;
            let bar = op_editor_ui::widgets::StatusBar::for_editor(&self.editor_state);
            if let Some(btn) = bar.control_at(r, Point2D::new(x, y)) {
                self.editor_state.editor_ui.pressed_button =
                    Some(op_editor_core::ButtonPressTarget::StatusBar(btn));
                match btn {
                    StatusBarButton::Search => self.zoom_to_fit(viewport_width, viewport_height),
                    StatusBarButton::ZoomOut => {
                        self.status_bar_zoom(false, viewport_width, viewport_height)
                    }
                    StatusBarButton::ZoomIn => {
                        self.status_bar_zoom(true, viewport_width, viewport_height)
                    }
                }
                self.mark_dirty();
                return Some(true);
            }
        }

        // Chat paints after the TopBar. In a short maximized viewport the
        // upward model dropdown can cover the bar, so route that visible slice
        // before the lower TopBar surface. Higher dropdowns/modals already ran.
        if y < TOP_BAR_HEIGHT
            && self.apply_chat_model_picker_overlay_press(x, y, viewport_width, viewport_height)
        {
            return Some(true);
        }
        None
    }

    /// Theme-preset dropdown + floating VariablesPanel.
    /// `None` — neither claimed the press.
    pub(in crate::widget_host) fn press_variables_tiers(&mut self, ctx: &PressCtx) -> Option<bool> {
        let (x, y) = (ctx.x, ctx.y);
        let viewport_width = ctx.viewport_width;
        let viewport_height = ctx.viewport_height;
        let over_chat_model_picker = ctx.over_chat_model_picker;
        // 0aa. Theme-preset dropdown (#20) — runs BEFORE the panel dispatch so
        //      the functional menu rows win over the panel's stub
        //      TogglePresetMenu mapping (native parity).
        if !over_chat_model_picker
            && self.dispatch_variables_preset_press(x, y, viewport_width, viewport_height)
        {
            return Some(true);
        }
        // 0ab. Floating VariablesPanel — full interactive grid
        //      mirroring the native host (#21). A press inside the
        //      panel rect dispatches; outside presses fall through to
        //      the normal layers (the panel floats, it isn't modal).
        if !over_chat_model_picker
            && self.dispatch_variables_panel_press(x, y, viewport_width, viewport_height)
        {
            return Some(true);
        }
        None
    }

    /// TopBar chrome + the blank-press fall-through for its gaps.
    /// `None` — the point is outside the bar entirely.
    pub(in crate::widget_host) fn press_top_bar_tier(&mut self, ctx: &PressCtx) -> Option<bool> {
        let (x, y) = (ctx.x, ctx.y);
        let viewport_width = ctx.viewport_width;
        let viewport_height = ctx.viewport_height;
        let rename_committed = ctx.rename_committed;
        let text_edit_committed = ctx.text_edit_committed;
        // 0b. TopBar — sidebar toggle + chrome buttons. Mirrors the
        //     native host so web + native behave identically.
        let top_bar_rect = self.top_bar_rect(viewport_width);
        let top_bar = self.top_bar();
        if let Some(hit) = top_bar.hit_test(top_bar_rect, Point2D::new(x, y)) {
            self.close_image_popovers_for_higher_overlay();
            // A TopBar destination replaces the collaboration Join caret.
            // Clear even stale focus so reopening Join cannot resurrect it.
            self.editor_state.editor_ui.blur_collab_join_input();
            self.commit_property_family_focus_if_any();
            let pressed = op_editor_ui::widgets::editor_state_ext::topbar_button_hover(hit);
            self.editor_state.editor_ui.pressed_button =
                Some(op_editor_core::ButtonPressTarget::TopBar(pressed));
            // Arms whose behaviour is identical on both hosts live in
            // the shared flow; only the platform ones fall through.
            match press_flow::apply_shared_top_bar_hit(&mut self.editor_state, hit, self.now_ms) {
                TopBarPress::Handled => {
                    self.mark_dirty();
                    return Some(true);
                }
                TopBarPress::FileMenuToggled => {
                    self.clear_layer_panel_hover();
                    self.mark_dirty();
                    return Some(true);
                }
                TopBarPress::Platform => {}
            }
            match hit {
                // Handled by the shared flow above.
                TopBarHit::ToggleSidebar
                | TopBarHit::ToggleTheme
                | TopBarHit::ToggleLocale
                | TopBarHit::OpenAgentSettings
                | TopBarHit::Collaboration
                | TopBarHit::ToggleFileMenu
                | TopBarHit::OpenExportMenu
                | TopBarHit::OpenAssetCenter
                | TopBarHit::OpenImportMenu => {}
                TopBarHit::ToggleGitPanel => {
                    self.editor_state.editor_ui.git_panel.open ^= true;
                }
                TopBarHit::ToggleFullscreen => {
                    // The web host runs in WASM, so it toggles the browser
                    // Fullscreen API directly — no runner round-trip and no
                    // unconsumed intent flag (unlike native, where the host
                    // can't reach the winit window).
                    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                        if doc.fullscreen_element().is_some() {
                            doc.exit_fullscreen();
                        } else if let Some(el) = doc.document_element() {
                            let _ = el.request_fullscreen();
                        }
                    }
                }
                TopBarHit::TogglePreview => {
                    if self.preview.is_some() {
                        // Exit preview mode. Track M-1: defer the teardown —
                        // only close the runtime if there is no device frame.
                        self.exit_preview(viewport_width, viewport_height);
                    } else {
                        #[cfg(feature = "canvaskit")]
                        {
                            let op_ck = self.op_ck.clone();
                            let _ = self.enter_preview_from_browser(
                                viewport_width,
                                viewport_height,
                                op_ck.as_ref(),
                            );
                        }
                        #[cfg(not(feature = "canvaskit"))]
                        {
                            self.editor_state.editor_ui.preview.mode = false;
                            self.editor_state.editor_ui.preview.warnings =
                                vec!["preview: not available in this build".to_string()];
                        }
                    }
                }
                TopBarHit::Account => {
                    if self.editor_state.editor_ui.account_ui_available {
                        if self.editor_state.editor_ui.account.is_signed_in() {
                            self.editor_state.editor_ui.account_menu_open = true;
                            self.editor_state.editor_ui.account_menu_hover = None;
                        } else {
                            self.editor_state.editor_ui.login_modal_open = true;
                            self.editor_state.editor_ui.login_modal_hover = None;
                        }
                    }
                }
            }
            self.mark_dirty();
            return Some(true);
        }
        if (top_bar_rect).contains(Point2D::new(x, y)) {
            // Top-bar gaps eat clicks but don't act — still a blank
            // press, so every text input blurs.
            let image_closed = self.close_image_popovers_for_higher_overlay();
            let blurred = self.blur_text_inputs_on_blank_press();
            return Some(image_closed || blurred || rename_committed || text_edit_committed);
        }
        None
    }
}
