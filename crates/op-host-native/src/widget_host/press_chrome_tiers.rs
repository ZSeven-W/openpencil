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
        // Presenting hides every surface this tier owns — the StatusBar, the
        // rail-resize gutters, the property-panel popovers (`paint.rs`). A
        // hidden widget must not keep claiming presses, or the deck would
        // have dead patches where the chrome used to be; falling through
        // hands the press to the preview tier, which advances the slide.
        if self.preview_slideshow_active() {
            return None;
        }
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
        // overlap, and it must stay ABOVE every left-rail surface: half of
        // its band lies over the rail's own content, and a rail that
        // claimed those pixels first could only ever be dragged wider.
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
        if let Some(hit) = self.topbar_hit_test(&top_bar, top_bar_rect, Point2D::new(x, y)) {
            self.close_image_popovers_for_higher_overlay();
            // A TopBar destination replaces the collaboration Join caret.
            // Clear even stale focus so reopening Join cannot resurrect it.
            self.editor_state.editor_ui.blur_collab_join_input();
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
            // Presenting a deck owns the whole canvas: the toolbar first,
            // then the board, which advances the deck on release. Neither
            // switcher paints while presenting, and the runtime is not fed —
            // a slide is being shown, not filled in.
            if self.preview_slideshow_active() {
                if !self.slideshow_toolbar_press(x, y, viewport_width, viewport_height) {
                    self.slideshow_board_press(x, y);
                }
                return Some(true);
            }
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

impl WidgetHostNative {
    /// Mobile layout: the compact app bar — layers / undo / redo /
    /// overflow. Every action routes through the same shared transitions
    /// the desktop chrome uses.
    pub(in crate::widget_host) fn press_mobile_app_bar_tier(
        &mut self,
        ctx: &PressCtx,
    ) -> Option<bool> {
        let point = Point2D::new(ctx.x, ctx.y);
        let bar = op_editor_ui::widgets::MobileAppBar::for_editor(&self.editor_state);
        let bar_rect = op_editor_ui::widgets::host_canvas_geometry::touch_app_bar_rect(
            &self.editor_state,
            ctx.viewport_width,
        );
        let hit = bar.hit_test(bar_rect, point)?;
        let now = self.now_ms;
        match hit {
            op_editor_ui::widgets::MobileAppBarHit::Layers => {
                if self.editor_state.editor_ui.expanded_touch_layout() {
                    self.cancel_native_touch_gestures();
                    let ui = &mut self.editor_state.editor_ui;
                    ui.mobile_sheet = None;
                    ui.sidebar_open = !ui.sidebar_open;
                } else {
                    self.toggle_mobile_sheet(op_editor_core::size_class::MobileSheetKind::Layers);
                }
            }
            op_editor_ui::widgets::MobileAppBarHit::Undo => {
                self.apply_undo();
            }
            op_editor_ui::widgets::MobileAppBarHit::Redo => {
                self.apply_redo();
            }
            op_editor_ui::widgets::MobileAppBarHit::Overflow => {
                self.toggle_mobile_sheet(op_editor_core::size_class::MobileSheetKind::More);
            }
        }
        let _ = now;
        self.mark_dirty();
        Some(true)
    }

    /// Mobile layout: the bottom tool dock — tool selection + shape
    /// slot + More sheet.
    pub(in crate::widget_host) fn press_mobile_dock_tier(
        &mut self,
        ctx: &PressCtx,
    ) -> Option<bool> {
        let point = Point2D::new(ctx.x, ctx.y);
        let dock = op_editor_ui::widgets::MobileDock::for_editor(&self.editor_state);
        let dock_rect = op_editor_ui::widgets::host_canvas_geometry::touch_dock_rect(
            &self.editor_state,
            ctx.viewport_width,
            ctx.viewport_height,
        );
        let hit = dock.hit_test(dock_rect, point)?;
        match hit {
            op_editor_ui::widgets::MobileDockHit::Tool(tool) => {
                self.editor_state.tool = tool;
            }
            op_editor_ui::widgets::MobileDockHit::ShapeSlot => {
                op_editor_core::host_press_transitions::toggle_shape_picker(
                    &mut self.editor_state.editor_ui,
                );
            }
            op_editor_ui::widgets::MobileDockHit::More => {
                self.toggle_mobile_sheet(op_editor_core::size_class::MobileSheetKind::More);
            }
        }
        self.mark_dirty();
        Some(true)
    }

    /// Selected-node contextual capsule: open the inspector or delete the
    /// complete selection. The explicit trash action uses document-delete
    /// semantics directly, rather than keyboard Delete's focus-first path.
    pub(in crate::widget_host) fn press_selection_actions_tier(
        &mut self,
        ctx: &PressCtx,
    ) -> Option<bool> {
        if !self.selection_actions_visible() {
            return None;
        }
        let rect = op_editor_ui::widgets::mobile_chrome::selection_actions_rect_for(
            &self.editor_state,
            ctx.viewport_width,
            ctx.viewport_height,
        );
        let hit = op_editor_ui::widgets::mobile_chrome::selection_action_hit(
            rect,
            Point2D::new(ctx.x, ctx.y),
        )?;
        match hit {
            op_editor_ui::widgets::mobile_chrome::SelectionActionHit::Properties => {
                self.editor_state.editor_ui.property_tab = op_editor_core::PropertyTab::Design;
                if !self.editor_state.editor_ui.expanded_touch_layout() {
                    self.editor_state.editor_ui.mobile_sheet =
                        Some(op_editor_core::size_class::MobileSheetKind::Properties);
                }
            }
            op_editor_ui::widgets::mobile_chrome::SelectionActionHit::Delete => {
                if self.collab_allows_document_mutation(
                    op_editor_core::CollabDocumentMutation::NodeDelete,
                ) {
                    let _ =
                        op_editor_core::host_keyboard_transitions::delete_selection_with_history(
                            &mut self.editor_state,
                        );
                }
            }
        }
        self.mark_dirty();
        Some(true)
    }

    /// Open/close the single mobile sheet; opening one closes the others.
    pub(in crate::widget_host) fn toggle_mobile_sheet(
        &mut self,
        kind: op_editor_core::size_class::MobileSheetKind,
    ) {
        self.cancel_native_touch_gestures();
        let ui = &mut self.editor_state.editor_ui;
        ui.mobile_sheet = if ui.mobile_sheet == Some(kind) {
            None
        } else {
            Some(kind)
        };
        if kind == op_editor_core::size_class::MobileSheetKind::Ai {
            // Toggling AI also flips the desktop chat visibility so the
            // two stay consistent.
            let chat = &mut self.editor_state.chat;
            if ui.mobile_sheet == Some(kind) {
                chat.collapsed = false;
                chat.focused = true;
            } else {
                chat.collapsed = true;
            }
        }
    }
}

impl WidgetHostNative {
    /// Open touch surface modal ownership. A tap outside closes only the
    /// surface; it never falls through to an app-bar, dock, page or canvas
    /// action hidden below the scrim.
    pub(in crate::widget_host) fn press_mobile_modal_surface_tier(
        &mut self,
        ctx: &PressCtx,
    ) -> Option<bool> {
        if !self.editor_state.editor_ui.touch_chrome() {
            return None;
        }
        if self.editor_state.editor_ui.variables_panel_open {
            let panel = self.variables_panel_rect(ctx.viewport_width, ctx.viewport_height)?;
            if !panel.contains(Point2D::new(ctx.x, ctx.y)) {
                self.editor_state.editor_ui.variables_panel_open = false;
                self.mark_dirty();
                return Some(true);
            }
            return None;
        }
        let kind = self.editor_state.editor_ui.mobile_sheet?;
        let panel = self.mobile_sheet_rect(ctx.viewport_width, ctx.viewport_height, kind);
        let point = Point2D::new(ctx.x, ctx.y);

        if kind == op_editor_core::size_class::MobileSheetKind::More && panel.contains(point) {
            return self.press_mobile_more_sheet_tier(ctx);
        }

        let owns_shared_close = matches!(
            kind,
            op_editor_core::size_class::MobileSheetKind::Layers
                | op_editor_core::size_class::MobileSheetKind::Properties
                | op_editor_core::size_class::MobileSheetKind::More
        );
        if owns_shared_close
            && op_editor_ui::widgets::mobile_chrome::sheet_close_rect(panel).contains(point)
        {
            self.cancel_native_touch_gestures();
            self.editor_state.editor_ui.mobile_sheet = None;
            self.mark_dirty();
            return Some(true);
        }
        if !panel.contains(point) {
            self.cancel_native_touch_gestures();
            self.editor_state.editor_ui.mobile_sheet = None;
            if kind == op_editor_core::size_class::MobileSheetKind::Ai {
                self.editor_state.chat.collapsed = true;
                self.editor_state.chat.focused = false;
            }
            self.mark_dirty();
            return Some(true);
        }
        None
    }

    /// Mobile More sheet: row presses + close. `None` when no More sheet
    /// is open or the press is outside it.
    pub(in crate::widget_host) fn press_mobile_more_sheet_tier(
        &mut self,
        ctx: &PressCtx,
    ) -> Option<bool> {
        if !self.editor_state.editor_ui.touch_chrome()
            || self.editor_state.editor_ui.mobile_sheet
                != Some(op_editor_core::size_class::MobileSheetKind::More)
        {
            return None;
        }
        let point = Point2D::new(ctx.x, ctx.y);
        let sheet = self.mobile_sheet_rect(
            ctx.viewport_width,
            ctx.viewport_height,
            op_editor_core::size_class::MobileSheetKind::More,
        );
        if !sheet.contains(point) {
            return None;
        }
        let close = op_editor_ui::widgets::mobile_chrome::sheet_close_rect(sheet);
        if close.contains(point) {
            self.cancel_native_touch_gestures();
            self.editor_state.editor_ui.mobile_sheet = None;
            self.mark_dirty();
            return Some(true);
        }
        if let Some(entry) =
            op_editor_ui::widgets::mobile_chrome::more_hit_test(&self.editor_state, sheet, point)
        {
            if entry == op_editor_ui::widgets::MobileMoreEntry::Ai {
                self.toggle_mobile_sheet(op_editor_core::size_class::MobileSheetKind::Ai);
                self.mark_dirty();
                return Some(true);
            }
            self.cancel_native_touch_gestures();
            self.editor_state.editor_ui.mobile_sheet = None;
            match entry {
                op_editor_ui::widgets::MobileMoreEntry::Ai => unreachable!("handled above"),
                op_editor_ui::widgets::MobileMoreEntry::OpenFile => {
                    if self.collab_allows_user_action(
                        op_editor_core::CollabGateAction::ReplaceDocument,
                    ) {
                        self.editor_state.editor_ui.pending_file_action =
                            Some(op_editor_core::editor_ui_state::FileAction::Open);
                    }
                }
                op_editor_ui::widgets::MobileMoreEntry::Templates
                | op_editor_ui::widgets::MobileMoreEntry::Assets => {
                    let tab = match entry {
                        op_editor_ui::widgets::MobileMoreEntry::Templates => {
                            op_editor_core::AssetCenterTab::Templates
                        }
                        op_editor_ui::widgets::MobileMoreEntry::Assets => {
                            op_editor_core::AssetCenterTab::Styles
                        }
                        _ => unreachable!("matched mobile asset-center entry"),
                    };
                    let _ = op_editor_ui::widgets::press_flow::apply_shared_top_bar_hit(
                        &mut self.editor_state,
                        op_editor_ui::widgets::TopBarHit::OpenAssetCenter,
                        self.now_ms,
                    );
                    self.editor_state
                        .editor_ui
                        .scene_template_center
                        .select_tab(tab);
                }
                op_editor_ui::widgets::MobileMoreEntry::Settings => {
                    let _ = op_editor_ui::widgets::press_flow::apply_shared_top_bar_hit(
                        &mut self.editor_state,
                        op_editor_ui::widgets::TopBarHit::OpenAgentSettings,
                        self.now_ms,
                    );
                }
                op_editor_ui::widgets::MobileMoreEntry::Variables => {
                    self.editor_state.editor_ui.toggle_variables_panel();
                }
                op_editor_ui::widgets::MobileMoreEntry::Preview => {
                    let (_, _, cw, ch) =
                        self.canvas_region(ctx.viewport_width, ctx.viewport_height);
                    self.toggle_preview((cw, ch));
                }
                op_editor_ui::widgets::MobileMoreEntry::Export => {
                    self.editor_state.editor_ui.open_export_dialog();
                }
            }
            self.mark_dirty();
            return Some(true);
        }
        // Inside the sheet but off any row — consume the press.
        Some(true)
    }
}
