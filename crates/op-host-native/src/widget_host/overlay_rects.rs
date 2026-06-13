//! Floating-overlay rect getters + the overlay predicates for
//! `WidgetHostNative`.
//!
//! Split out of `geometry.rs` to keep that file under the repo's
//! 800-line cap. Every panel / popover / dropdown that paints on top
//! of the canvas computes its on-screen rect here; `over_topmost_panel`
//! and `over_floating_overlay` OR those rects so input dispatch + the
//! cursor hint never bleed a canvas action (Move / Crosshair) through a
//! floating overlay onto a node underneath.

use super::helpers::{GIT_PANEL_CARET_GAP, TOOLBAR_INSET_X, TOOLBAR_INSET_Y};
use super::WidgetHostNative;
use op_editor_ui::widgets::{
    AlignToolbar, GitPanel, GitPanelHit, LayoutCx, LocalePicker, ShapePicker, Toolbar, TopBar,
    Widget, GIT_PANEL_INSET, ICON_PICKER_PANEL_H, ICON_PICKER_PANEL_W, LOCALE_PICKER_WIDTH,
    SHAPE_PICKER_WIDTH, TOOLBAR_WIDTH, TOP_BAR_HEIGHT,
};
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
    /// Floating Git-panel rect — `None` when the panel is closed.
    /// Single source of truth for the panel placement: `paint.rs`
    /// calls this so the caret + hit-test + cursor logic can't drift.
    ///
    /// The panel hangs centred under the TopBar Git button (a popover,
    /// TS parity), clamped to the canvas region. When the button is
    /// hidden (wasm) or the canvas is too narrow it falls back to the
    /// top-left float.
    pub(in crate::widget_host) fn git_panel_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<Rect> {
        let panel = GitPanel::for_editor(&self.editor_state)?;
        let pw = panel.panel_width();
        let ph = panel.height();
        let (canvas_left, _cy, canvas_w, _ch) = self.canvas_region(viewport_w, viewport_h);

        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_w, TOP_BAR_HEIGHT),
        };
        let top_bar = TopBar::for_editor_ui(&self.editor_state.editor_ui);
        let min_x = canvas_left + GIT_PANEL_INSET;
        let max_x = (canvas_left + canvas_w - pw - GIT_PANEL_INSET).max(min_x);
        let origin_x = match top_bar.git_button_center_x(top_bar_rect) {
            Some(cx) => (cx - pw / 2.0).clamp(min_x, max_x),
            None => min_x,
        };
        Some(Rect {
            origin: Point2D::new(origin_x, TOP_BAR_HEIGHT + GIT_PANEL_CARET_GAP),
            size: Point2D::new(pw, ph),
        })
    }

    /// The Git panel's full interactive footprint — the body plus the
    /// caret bridge above it (up to the top bar). [`git_panel_rect`] is
    /// the painted body; the caret is painted in that gap, so the
    /// hit-test, cursor, and outside-click logic must use THIS rect, or
    /// a click / hover on the caret falls through to the canvas under it.
    pub(in crate::widget_host) fn git_panel_outer_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<Rect> {
        self.git_panel_rect(viewport_w, viewport_h).map(|r| {
            let extra = r.origin.y - TOP_BAR_HEIGHT; // == GIT_PANEL_CARET_GAP
            Rect {
                origin: Point2D::new(r.origin.x, TOP_BAR_HEIGHT),
                size: Point2D::new(r.size.x, r.size.y + extra),
            }
        })
    }

    /// Update `git_panel.empty_hovered_card` from the cursor — drives
    /// the per-card hover effect + the disabled-Init hint (shown only
    /// while card 0 is hovered). Returns `true` when the hovered card
    /// changed (a repaint is due).
    pub(in crate::widget_host) fn update_git_panel_empty_hover(&mut self, x: f32, y: f32) -> bool {
        let hovered = self
            .git_panel_rect(self.last_viewport_w, self.last_viewport_h)
            .and_then(|body| {
                GitPanel::for_editor(&self.editor_state)
                    .and_then(|p| p.empty_card_at(body, Point2D::new(x, y)))
            });
        if hovered != self.editor_state.editor_ui.git_panel.empty_hovered_card {
            self.editor_state.editor_ui.git_panel.empty_hovered_card = hovered;
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    /// Update `git_panel.branch_button_hovered` from the cursor — drives
    /// the ready-view `⎇ <branch> ▾` button's `hover:bg-accent` wash.
    /// Reuses the panel's own hit-test (`BranchPicker` is returned only
    /// over that button in the ready view). Returns `true` when the hover
    /// state flipped (a repaint is due).
    pub(in crate::widget_host) fn update_git_panel_ready_hover(&mut self, x: f32, y: f32) -> bool {
        let point = Point2D::new(x, y);
        let panel_body = self.git_panel_rect(self.last_viewport_w, self.last_viewport_h);
        let hit = panel_body.and_then(|body| {
            GitPanel::for_editor(&self.editor_state).and_then(|p| p.hit_test(body, point))
        });
        let tracked_picker_hover = panel_body.and_then(|body| {
            GitPanel::for_editor(&self.editor_state).and_then(|p| {
                match p.tracked_picker_select_hit(body, point) {
                    op_editor_ui::widgets::git_panel::SelectHit::Row(idx) => Some(idx),
                    op_editor_ui::widgets::git_panel::SelectHit::Inside
                    | op_editor_ui::widgets::git_panel::SelectHit::Outside => None,
                }
            })
        });
        // The `⎇ <branch> ▾` trigger keeps its own bool wash; the plain
        // action buttons (pull / push / overflow / commit / milestone /
        // refresh) light up via `button_hover`.
        let branch_hovered = matches!(hit, Some(GitPanelHit::BranchPicker));
        let button_hover = hit.and_then(op_editor_ui::widgets::editor_state_ext::git_button_hover);
        let mut changed = false;
        if branch_hovered != self.editor_state.editor_ui.git_panel.branch_button_hovered {
            self.editor_state.editor_ui.git_panel.branch_button_hovered = branch_hovered;
            changed = true;
        }
        if button_hover != self.editor_state.editor_ui.git_panel.button_hover {
            self.editor_state.editor_ui.git_panel.button_hover = button_hover;
            changed = true;
        }
        if tracked_picker_hover != self.editor_state.editor_ui.git_panel.tracked_picker.hover {
            self.editor_state.editor_ui.git_panel.tracked_picker.hover = tracked_picker_hover;
            changed = true;
        }
        if changed {
            self.mark_dirty();
        }
        changed
    }

    /// Whether `(x, y)` is over the *disabled* empty-state Init card —
    /// the no-saved-file onboarding card that can't create history.
    /// The cursor shows `NotAllowed` over it (TS `cursor-not-allowed`).
    pub(in crate::widget_host) fn over_disabled_init_card(
        &self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        if self.editor_state.editor_ui.git_panel.has_saved_file {
            return false;
        }
        self.git_panel_rect(viewport_w, viewport_h)
            .and_then(|body| {
                GitPanel::for_editor(&self.editor_state).and_then(|p| p.empty_init_card_rect(body))
            })
            .is_some_and(|card| (card).contains(Point2D::new(x, y)))
    }

    /// Floating Component-Browser panel rect — `None` when closed.
    /// Same centred-on-open + clamped placement as the Design-MD panel.
    pub(in crate::widget_host) fn component_browser_panel_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<Rect> {
        use op_editor_ui::widgets::{COMPONENT_BROWSER_PANEL_H, COMPONENT_BROWSER_PANEL_W};
        let ui = &self.editor_state.editor_ui;
        if !ui.component_browser_open {
            return None;
        }
        let (px, py) = ui.component_browser_pos.unwrap_or_else(|| {
            (
                ((viewport_w - COMPONENT_BROWSER_PANEL_W) / 2.0).max(0.0),
                ((viewport_h - COMPONENT_BROWSER_PANEL_H) / 2.0).max(0.0),
            )
        });
        let x = px.clamp(0.0, (viewport_w - 80.0).max(0.0));
        let y = py.clamp(0.0, (viewport_h - 40.0).max(0.0));
        Some(Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(COMPONENT_BROWSER_PANEL_W, COMPONENT_BROWSER_PANEL_H),
        })
    }

    /// Floating Icon-picker panel rect — `None` when closed.
    /// The TS picker is a dialog; native centers a compact searchable
    /// panel because the built-in Rust catalog is local and finite.
    pub(in crate::widget_host) fn icon_picker_panel_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<Rect> {
        if !self.editor_state.editor_ui.icon_picker_open {
            return None;
        }
        let ui = &self.editor_state.editor_ui;
        let (px, py) = ui.icon_picker_panel_pos.unwrap_or_else(|| {
            (
                ((viewport_w - ICON_PICKER_PANEL_W) / 2.0).max(0.0),
                ((viewport_h - ICON_PICKER_PANEL_H) / 2.0).max(0.0),
            )
        });
        let x = px.clamp(0.0, (viewport_w - 80.0).max(0.0));
        let y = py.clamp(0.0, (viewport_h - 40.0).max(0.0));
        Some(Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(ICON_PICKER_PANEL_W, ICON_PICKER_PANEL_H),
        })
    }

    /// Whether `point` is inside ANY top-most floating panel
    /// (Design-MD or Component-Browser). Used by the input gates so
    /// wheel / pan / right-press / hover side-effects do not leak to
    /// the canvas / lower layers beneath the panel.
    pub(in crate::widget_host) fn over_topmost_panel(
        &self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        let p = Point2D::new(x, y);
        self.design_md_panel_rect(viewport_w, viewport_h)
            .is_some_and(|r| (r).contains(p))
            || self
                .variables_panel_rect(viewport_w, viewport_h)
                .is_some_and(|r| (r).contains(p))
            || self
                .icon_picker_panel_rect(viewport_w, viewport_h)
                .is_some_and(|r| (r).contains(p))
            || self
                .component_browser_panel_rect(viewport_w, viewport_h)
                .is_some_and(|r| (r).contains(p))
    }

    /// The open File-menu dropdown rect, or `None` when closed. The
    /// rect depends only on the menu's row count (recent-file list),
    /// not on time, so a `0` clock is fine here.
    pub(in crate::widget_host) fn file_menu_rect(&self, viewport_w: f32) -> Option<Rect> {
        use op_editor_ui::widgets::file_menu::FileMenu;
        if !self.editor_state.editor_ui.file_menu_open {
            return None;
        }
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_w, TOP_BAR_HEIGHT),
        };
        let anchor =
            TopBar::file_menu_rect(top_bar_rect, self.editor_state.editor_ui.window_fullscreen);
        let menu = FileMenu::from_editor_ui(&self.editor_state.editor_ui, 0);
        Some(menu.rect_at(anchor))
    }

    /// Whether `(x, y)` is over ANY floating overlay that paints on top
    /// of the canvas region — the centred modal-ish panels, the Git
    /// panel popover, the always-on Toolbar / StatusBar / chat, and the
    /// open dropdowns (shape / locale / file menu). The cursor must
    /// stay neutral over these instead of bleeding a canvas action
    /// cursor (Move / Crosshair) for whatever node sits underneath.
    pub(in crate::widget_host) fn over_floating_overlay(
        &self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        if self.over_topmost_panel(x, y, viewport_w, viewport_h) {
            return true;
        }
        let p = Point2D::new(x, y);
        let ui = &self.editor_state.editor_ui;
        // The Git panel + always-on floating widgets + the alignment
        // toolbar (shown over the canvas for a multi-selection).
        if self
            .git_panel_outer_rect(viewport_w, viewport_h)
            .is_some_and(|r| (r).contains(p))
            || self
                .status_bar_rect(viewport_w, viewport_h)
                .is_some_and(|r| (r).contains(p))
            || self
                .ai_chat_rect(viewport_w, viewport_h)
                .is_some_and(|r| (r).contains(p))
            || (self.toolbar_rect(viewport_w, viewport_h)).contains(p)
            || self
                .align_toolbar_rect(viewport_w, viewport_h)
                .is_some_and(|r| (r).contains(p))
        {
            return true;
        }
        // Open dropdowns + the right-click context menu.
        (ui.shape_picker.open && (self.shape_picker_rect(viewport_w, viewport_h)).contains(p))
            || (ui.locale_picker.open && (self.locale_picker_rect(viewport_w)).contains(p))
            || self
                .file_menu_rect(viewport_w)
                .is_some_and(|r| (r).contains(p))
            || self
                .layer_context_menu_rect()
                .is_some_and(|r| (r).contains(p))
    }

    /// Floating Design-MD panel rect — `None` when the panel is
    /// closed. The top-left comes from `editor_ui.design_md_panel_pos`
    /// (centred by the host on open), clamped to keep the header bar
    /// reachable after a viewport resize.
    pub(in crate::widget_host) fn design_md_panel_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<Rect> {
        use op_editor_ui::widgets::{DESIGN_MD_PANEL_H, DESIGN_MD_PANEL_W};
        let ui = &self.editor_state.editor_ui;
        if !ui.design_md_panel_open {
            return None;
        }
        let (px, py) = ui.design_md_panel_pos.unwrap_or_else(|| {
            (
                ((viewport_w - DESIGN_MD_PANEL_W) / 2.0).max(0.0),
                ((viewport_h - DESIGN_MD_PANEL_H) / 2.0).max(0.0),
            )
        });
        // Keep at least the header bar on-screen.
        let x = px.clamp(0.0, (viewport_w - 80.0).max(0.0));
        let y = py.clamp(0.0, (viewport_h - 40.0).max(0.0));
        Some(Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(DESIGN_MD_PANEL_W, DESIGN_MD_PANEL_H),
        })
    }

    pub(in crate::widget_host) fn shape_picker_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Rect {
        let (cx0, _cy, cw, _ch) = self.canvas_region(viewport_w, viewport_h);
        let toolbar = Toolbar::for_editor(&self.editor_state);
        let toolbar_h = toolbar
            .layout(&LayoutCx {
                available_width: TOOLBAR_WIDTH,
                dpi: 1.0,
            })
            .rect
            .size
            .y;
        let toolbar_rect = Rect {
            origin: Point2D::new(cx0 + TOOLBAR_INSET_X, TOP_BAR_HEIGHT + TOOLBAR_INSET_Y),
            size: Point2D::new(TOOLBAR_WIDTH, toolbar_h),
        };
        let slot = toolbar
            .shape_slot_rect(toolbar_rect)
            .unwrap_or(toolbar_rect);
        let panel_h = ShapePicker::panel_height();
        let max_x = cx0 + cw - SHAPE_PICKER_WIDTH - 4.0;
        let toolbar_right = toolbar_rect.origin.x + toolbar_rect.size.x;
        let x = (toolbar_right + 8.0).min(max_x);
        let y = slot.origin.y;
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(SHAPE_PICKER_WIDTH, panel_h),
        }
    }

    pub(in crate::widget_host) fn locale_picker_rect(&self, viewport_w: f32) -> Rect {
        let top_bar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_w, TOP_BAR_HEIGHT),
        };
        let globe = TopBar::globe_rect(top_bar_rect);
        let panel_h = LocalePicker::panel_height();
        let x = (globe.origin.x + globe.size.x / 2.0 - LOCALE_PICKER_WIDTH / 2.0)
            .max(8.0)
            .min(viewport_w - LOCALE_PICKER_WIDTH - 8.0);
        let y = globe.origin.y + globe.size.y + 6.0;
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(LOCALE_PICKER_WIDTH, panel_h),
        }
    }

    /// The floating AlignToolbar's rect, or `None` when fewer than two
    /// nodes are selected. The toolbar floats over the canvas, so the
    /// cursor must stay neutral over it.
    pub(in crate::widget_host) fn align_toolbar_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<Rect> {
        let (canvas_left, _t, canvas_w, canvas_h) = self.canvas_region(viewport_w, viewport_h);
        let canvas_region = Rect {
            origin: Point2D::new(canvas_left, TOP_BAR_HEIGHT),
            size: Point2D::new(canvas_w, canvas_h),
        };
        AlignToolbar::for_canvas_region(canvas_region, &self.editor_state).map(|t| t.rect())
    }

    /// The open layer/page right-click context-menu rect, or `None`
    /// when no menu is open. Anchored at the clicked row, the menu can
    /// extend right into the canvas — so it suppresses the cursor too.
    pub(in crate::widget_host) fn layer_context_menu_rect(&self) -> Option<Rect> {
        use op_editor_ui::widgets::layer_context_menu::LayerContextMenu;
        let state = self.editor_state.editor_ui.layer_context_menu.clone()?;
        Some(LayerContextMenu::for_state(&self.editor_state, state).rect())
    }
}
