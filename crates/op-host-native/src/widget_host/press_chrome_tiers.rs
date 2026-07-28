//! `apply_press` tiers 3-5 — the rail-overlay band, the TopBar, and
//! Preview (Play) mode.
//!
//! Tier 3 covers surfaces that float over the rails: the image-fill
//! popover, the StatusBar controls, the panel-resize gutter, and the slice
//! of the chat model picker that can lift above the TopBar. Several of
//! these are gated on `in_git_panel` / `in_chat_model_picker` because the
//! Git panel and the picker paint above them.
//!
//! Tier 5 must stay AFTER the TopBar tier: the Play/Stop button lives in
//! the bar, and every other press while previewing routes into the runtime.

use super::press_ctx::PressCtx;
use super::{PanelResize, PanelResizeKind, WidgetHostNative};
use op_editor_ui::widgets::press_flow::{self, TopBarPress};
use op_editor_ui::widgets::{TopBar, TopBarHit, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
    /// `None` — no rail overlay claimed the press.
    pub(in crate::widget_host) fn press_rail_overlay_tiers(
        &mut self,
        ctx: &PressCtx,
    ) -> Option<bool> {
        let (x, y) = (ctx.x, ctx.y);
        let viewport_width = ctx.viewport_width;
        let viewport_height = ctx.viewport_height;
        let in_git_panel = ctx.in_git_panel;
        let in_chat_model_picker = ctx.in_chat_model_picker;
        // 0a1. Image-fill popover. Property overlays are painted after the
        // VariablesPanel, chat, StatusBar, marquee, and rail chrome, so the
        // visible popup must own their overlap before those lower surfaces.
        // Git and the modal/menu overlays above already had first refusal.
        if !in_git_panel
            && self.dismiss_image_fill_popover_on_press(x, y, viewport_width, viewport_height)
        {
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

        // 0z. Panel-resize gutter — ±4 px from rail edges. The gutter is
        // lower than the floating image-fill card even where their bounds
        // overlap.
        if y >= TOP_BAR_HEIGHT && !in_git_panel && !in_chat_model_picker {
            if let Some(kind) = self.panel_resize_hover(x, y, viewport_width) {
                let start_width = match kind {
                    PanelResizeKind::LayerRight => self.editor_state.editor_ui.layer_panel_width,
                    PanelResizeKind::PropertyLeft => {
                        self.editor_state.editor_ui.property_panel_width
                    }
                };
                self.panel_resize = Some(PanelResize {
                    kind,
                    start_x: x,
                    start_width,
                });
                return Some(true);
            }
        }

        // Chat paints after the TopBar. A short/top-anchored chat can lift the
        // upward model dropdown across the bar, so its visible overlap must be
        // routed before the lower TopBar surface. Other dropdowns/modals above
        // chat already had first refusal in the blocks above.
        if y < TOP_BAR_HEIGHT
            && !in_git_panel
            && self.apply_chat_model_picker_overlay_press(x, y, viewport_width, viewport_height)
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
        // 0b. TopBar — sidebar toggle button + theme + locale picker.
        self.refresh_layout_scene();
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_width, TOP_BAR_HEIGHT),
        };
        let mut top_bar = TopBar::for_editor_ui(&self.editor_state.editor_ui);
        top_bar.chip_text_w = Some(self.topbar_chip_text_w(&top_bar));
        if let Some(hit) = top_bar.hit_test(top_bar_rect, Point2D::new(x, y)) {
            self.close_image_popovers_for_higher_overlay();
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
                | TopBarHit::OpenImportMenu => return Some(true),
                TopBarHit::ToggleFullscreen => {
                    // The widget host doesn't own the winit window — raise
                    // an intent the desktop runner consumes next frame.
                    self.editor_state.editor_ui.pending_fullscreen_toggle = true;
                    self.mark_dirty();
                    return Some(true);
                }
                TopBarHit::TogglePreview => {
                    // Enter / exit canvas Preview (Play) mode. Layout is
                    // solved per-root from the document; the canvas region
                    // is passed only for API compatibility (paint transform
                    // uses the viewport, layout does not).
                    let (_cx0, _cy0, cw, ch) = self.canvas_region(viewport_width, viewport_height);
                    self.toggle_preview((cw, ch));
                    return Some(true);
                }
                TopBarHit::ToggleGitPanel => {
                    // Mirror `main.rs` A::ToggleGitPanel bookkeeping; the
                    // binary's per-frame `if git_panel.open { refresh }`
                    // performs the actual repo scan.
                    let panel = &mut self.editor_state.editor_ui.git_panel;
                    let opening = !panel.open;
                    panel.open = opening;
                    if opening {
                        panel.loading = true;
                    } else {
                        panel.defocus_commit_input(self.now_ms);
                        panel.remote_focused = false;
                        panel.https_focused = false;
                        panel.diff = None;
                        panel.merge_resolve = None;
                        // Close the clone wizard synchronously so a rapid
                        // close→reopen can't resurface a stale form before
                        // the host's `poll_git_clone_job` runs (it then
                        // abandons any in-flight job — `cloning` form gone).
                        panel.clone_form = None;
                    }
                    self.mark_dirty();
                    return Some(true);
                }
                TopBarHit::Account => {
                    if !self.editor_state.editor_ui.account_ui_available {
                        return Some(false);
                    }
                    if self.editor_state.editor_ui.account.is_signed_in() {
                        self.editor_state.editor_ui.account_menu_open = true;
                        self.editor_state.editor_ui.account_menu_hover = None;
                    } else {
                        self.editor_state.editor_ui.login_modal_open = true;
                        self.editor_state.editor_ui.login_modal_hover = None;
                    }
                    self.mark_dirty();
                    return Some(true);
                }
            }
        }
        if (top_bar_rect).contains(Point2D::new(x, y)) {
            // Other top-bar gaps eat clicks but don't act — still a
            // blank press, so every text input blurs.
            let image_closed = self.close_image_popovers_for_higher_overlay();
            let blurred = self.blur_text_inputs_on_blank_press();
            return Some(image_closed || blurred || rename_committed || text_edit_committed);
        }
        None
    }

    /// Preview (Play) mode swallows every canvas press. `None` — not
    /// previewing.
    pub(in crate::widget_host) fn press_preview_tier(&mut self, ctx: &PressCtx) -> Option<bool> {
        let (x, y) = (ctx.x, ctx.y);
        let viewport_width = ctx.viewport_width;
        let viewport_height = ctx.viewport_height;
        // 0b'. Preview (Play) mode — the canvas belongs to the live
        // runtime, not the editor. The TopBar Play/Stop button was
        // already handled above; any other press routes into the
        // runtime (taps on switches / buttons, caret placement) and is
        // swallowed so no editor selection / node-creation fires.
        if self.preview.is_some() {
            if self.screen_switcher_press(x, y, viewport_width, viewport_height) {
                return Some(true);
            }
            if self.preview_switcher_press(x, y, viewport_width, viewport_height) {
                return Some(true);
            }
            self.preview_dispatch_press(x, y, viewport_width, viewport_height);
            return Some(true);
        }
        None
    }
}
