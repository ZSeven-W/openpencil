//! The menu / modal / floating-panel / notice half of the web host's
//! composition pass.
//!
//! Carved out of `paint.rs` at the repo's 800-line cap. Pure code motion:
//! the block is byte-identical to the tail `paint_editor` used to run
//! inline, and `paint_editor` still calls it at exactly the same point, so
//! the z-order it encodes is unchanged.

use super::WidgetHost;
use op_editor_ui::widgets::{
    ComponentBrowserPanel, DesignMdPanel, IconPickerPanel, PaintCx, PromptCenterPanel,
    SceneTemplatePanel, Widget, TOP_BAR_HEIGHT,
};
use op_editor_ui::{Point2D, Rect, RenderBackend};

impl WidgetHost {
    /// Menus, modals, floating panels and the top-most notices.
    ///
    /// Pure code motion out of [`Self::paint_editor`] at the repo's
    /// 800-line cap: the block is byte-identical to the one that used to
    /// run inline, and `paint_editor` calls it at exactly the same point,
    /// so the z-order it encodes is unchanged.
    pub(in crate::widget_host) fn paint_menu_modal_and_panel_layers(
        &mut self,
        backend: &mut dyn RenderBackend,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        // Re-bound here with the same expression `paint_editor` used, so the
        // moved block reads exactly the state it read inline.
        let ui = &self.editor_state.editor_ui;
        // File-menu dropdown — anchored under TopBar's folder+chevron
        // button (native §10b).
        if let Some(menu_rect) = self.file_menu_rect(viewport_width) {
            use op_editor_ui::widgets::file_menu::FileMenu;
            let menu = FileMenu::from_editor_ui(&self.editor_state.editor_ui, self.wall_now_secs);
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            menu.paint(&mut cx, menu_rect);
        }

        // Export quick menu — anchored under the TopBar download button,
        // same overlay band as the file menu it shortcuts (native §10b).
        if ui.export_quick_menu_open {
            use op_editor_ui::widgets::ExportQuickMenu;
            let menu_rect = self.export_quick_menu_rect(viewport_width);
            let menu = ExportQuickMenu::for_editor_ui(&self.editor_state.editor_ui);
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            menu.paint(&mut cx, menu_rect);
        }

        // Figma import modal — full-viewport scrim + centred card
        // (native §10c).
        if ui.figma_import_open {
            use op_editor_ui::widgets::figma_import::FigmaImportModal;
            backend.fill_rect(
                Rect {
                    origin: Point2D::new(0.0, 0.0),
                    size: Point2D::new(viewport_width, viewport_height),
                },
                op_editor_ui::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.45,
                },
            );
            let modal = FigmaImportModal::for_editor(&self.editor_state);
            let modal_rect = modal.rect(viewport_width, viewport_height);
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            modal.paint(&mut cx, modal_rect);
        }

        // Export dialog — full-viewport scrim + centred card
        // (native §10d).
        if ui.export_dialog_open {
            use op_editor_ui::widgets::ExportDialog;
            backend.fill_rect(
                Rect {
                    origin: Point2D::new(0.0, 0.0),
                    size: Point2D::new(viewport_width, viewport_height),
                },
                op_editor_ui::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.45,
                },
            );
            let dlg = ExportDialog::centered(viewport_width, viewport_height);
            dlg.paint(&mut *backend, &self.theme, &self.editor_state.editor_ui);
        }

        // Sign-in modal — full-viewport scrim + centred card (native §10e).
        if ui.account_ui_available && ui.login_modal_open {
            use op_editor_ui::widgets::login_modal::LoginModal;
            backend.fill_rect(
                Rect {
                    origin: Point2D::new(0.0, 0.0),
                    size: Point2D::new(viewport_width, viewport_height),
                },
                op_editor_ui::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.45,
                },
            );
            let modal = LoginModal::for_editor(&self.editor_state);
            let modal_rect = modal.rect(viewport_width, viewport_height);
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            modal.paint(&mut cx, modal_rect);
        }

        // Signed-in account dropdown — anchored under the TopBar avatar
        // button, no scrim (native §10f).
        if ui.account_ui_available && ui.account_menu_open {
            use op_editor_ui::widgets::account_menu::AccountMenu;
            use op_editor_ui::widgets::top_bar::TopBar;
            use op_editor_ui::widgets::TOP_BAR_HEIGHT;
            let top_bar_rect = Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(viewport_width, TOP_BAR_HEIGHT),
            };
            let top_bar = TopBar::for_editor_ui(&self.editor_state.editor_ui);
            let anchor = top_bar.account_button_rect(top_bar_rect);
            if let Some(menu) = AccountMenu::for_editor_ui(&self.editor_state.editor_ui) {
                let menu_rect = menu.rect_at(anchor);
                let mut cx = PaintCx {
                    backend: &mut *backend,
                };
                menu.paint(&mut cx, menu_rect);
            }
        }

        // Settings modal — Cmd+, overlay. Painted before the colour
        // picker / context menu / floating panels, mirroring native
        // §10a z-order.
        if ui.agent_settings_open {
            use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;
            let panel = AgentSettingsPanel::for_web_editor_at(&self.editor_state, self.now_ms);
            let panel_rect = panel.rect(viewport_width, viewport_height);
            // Dim scrim behind the modal so the underlying canvas
            // reads as "blocked." Matches the native shell's chrome.
            backend.fill_rect(
                Rect {
                    origin: Point2D::new(0.0, 0.0),
                    size: Point2D::new(viewport_width, viewport_height),
                },
                op_editor_ui::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.5,
                },
            );
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            panel.paint(&mut cx, panel_rect);
        }

        // Colour picker — floating overlay near the right rail
        // (native §10b').
        if let Some(state) = self.editor_state.ui.color_picker.clone() {
            use op_editor_ui::widgets::color_picker::ColorPicker;
            let picker = ColorPicker::for_state(&self.editor_state, state);
            let picker_rect = picker.rect(viewport_width, viewport_height);
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            picker.paint(&mut cx, picker_rect);
        }

        // Layer context menu — right-click overlay above everything
        // painted so far (native §11).
        if let Some(state) = self.editor_state.editor_ui.layer_context_menu.clone() {
            use op_editor_ui::widgets::layer_context_menu::LayerContextMenu;
            let menu = LayerContextMenu::for_state(&self.editor_state, state);
            let menu_rect = menu.rect();
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            menu.paint(&mut cx, menu_rect);
        }

        // Path-anchor context menu — Select-tool right-click on a
        // path anchor / handle (native §11a).
        if let Some(state) = self.editor_state.ui.path_anchor_menu.clone() {
            use op_editor_ui::widgets::path_anchor_context_menu::PathAnchorContextMenu;
            let menu = PathAnchorContextMenu::for_state(&self.editor_state, state);
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            menu.paint(&mut cx);
        }

        // Floating Component-Browser panel — painted just below the
        // Design-MD panel so when both are open Design-MD sits
        // absolute-top (native §11.5).
        if let (Some(panel), Some(panel_rect)) = (
            ComponentBrowserPanel::for_editor_at(&self.editor_state, self.now_ms),
            self.component_browser_panel_rect(viewport_width, viewport_height),
        ) {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            panel.paint(&mut cx, panel_rect);
        }

        // Asset Center gallery — a dimming scrim across the whole viewport,
        // then the panel over it (native `paint_floating_panels.rs`). The
        // scrim is also the surface that takes a dismiss press, so paint and
        // hit-test must agree that it covers everything.
        if let (Some(panel), Some(panel_rect)) = (
            SceneTemplatePanel::for_editor_at(&self.editor_state, self.now_ms),
            self.scene_template_panel_rect(viewport_width, viewport_height),
        ) {
            if let Some(scrim) = self.scene_template_scrim_rect(viewport_width, viewport_height) {
                backend.fill_rect(scrim, op_editor_ui::widgets::SCENE_TEMPLATE_SCRIM);
            }
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            panel.paint(&mut cx, panel_rect);
        }

        if let (Some(panel), Some(panel_rect)) = (
            PromptCenterPanel::for_editor_at(&self.editor_state, self.now_ms),
            self.prompt_center_panel_rect(viewport_width, viewport_height),
        ) {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            panel.paint(&mut cx, panel_rect);
        }

        // Floating Icon picker — opened from the shape-tool dropdown.
        // Above the component browser, below Design-MD, matching the
        // press routing order (native §11.7).
        if let (Some(panel), Some(panel_rect)) = (
            IconPickerPanel::for_editor_at(&self.editor_state, self.now_ms),
            self.icon_picker_panel_rect(viewport_width, viewport_height),
        ) {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            panel.paint(&mut cx, panel_rect);
        }

        // Floating Design-MD panel — the document's design.md brief.
        // Painted last among the panels so it is the top-most overlay;
        // hit-test mirrors this (`press.rs` dispatches it first)
        // (native §12).
        if let (Some(panel), Some(panel_rect)) = (
            DesignMdPanel::for_editor(&self.editor_state),
            self.design_md_panel_rect(viewport_width, viewport_height),
        ) {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            panel.paint(&mut cx, panel_rect);
        }

        // File-drop overlay — top-most layer while a file is dragged
        // over the window (native §13). The web runner doesn't raise
        // `file_drop_active` yet; painting the guard keeps z-order
        // parity for when DOM drag events get wired.
        if self.editor_state.editor_ui.file_drop_active {
            let (drop_left, _y, drop_w, drop_h) =
                self.canvas_region(viewport_width, viewport_height);
            let drop_rect = Rect {
                origin: Point2D::new(drop_left, TOP_BAR_HEIGHT),
                size: Point2D::new(drop_w, drop_h),
            };
            op_editor_ui::widgets::file_drop_overlay::paint_file_drop_overlay(
                &mut *backend,
                &self.theme,
                self.editor_state.editor_ui.effective_locale(),
                drop_rect,
                // The browser host has no drag-position stream yet, so it can
                // never resolve a node target to ring.
                None,
            );
        }

        // The top-most overlay band lives in its own sibling, mirroring
        // the native host's `paint_topmost_overlays.rs`; its internal
        // z-order is what `press_overlay_tiers.rs` mirrors in reverse.
        self.paint_topmost_overlays(&mut *backend, viewport_width, viewport_height);
    }
}
