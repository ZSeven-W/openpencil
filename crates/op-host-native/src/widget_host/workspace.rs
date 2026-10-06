//! Generation-workspace input routing + per-frame pumping on the
//! native widget host.
//!
//! The workspace is NOT a takeover like Home: its chrome (header,
//! toolbar, deck strip, dock handle) claims presses ahead of the top
//! bar / rails, while the pinned chat panel and the docked canvas keep
//! their ordinary press / cursor tiers. This module owns the chrome
//! tier, its actions, the dock-resize drag, the per-mode camera fits,
//! and the per-frame generation pump the desktop runner drives.

use super::WidgetHostNative;
use op_editor_core::preview_slideshow::active_page_boards;
use op_editor_core::{EntrySurface, Tool, WorkspaceHit, WorkspacePhase, WorkspaceView};
use op_editor_ui::widgets::host_overlay_geometry::{
    status_bar_zoom, zoom_to_fit, zoom_to_fit_node,
};
use op_editor_ui::widgets::{WorkspaceLayout, WorkspaceSurface};
use op_editor_ui::{Point2D, Rect};

/// A live dock-width drag: the press x and the left panel's width at
/// press — the dock IS the left panel, so the drag writes the shared
/// `layer_panel_width`.
#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct WorkspaceDockDrag {
    pub(in crate::widget_host) start_x: f32,
    pub(in crate::widget_host) start_w: f32,
}

impl WidgetHostNative {
    pub fn workspace_visible(&self) -> bool {
        self.editor_state.editor_ui.workspace.visible
    }

    /// The chrome tier — runs after Home and the modal tiers, ahead of
    /// the top bar / rails. `None` lets the chat and canvas tiers run.
    pub(in crate::widget_host) fn press_workspace(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Option<bool> {
        if !self.workspace_visible() || self.editor_state.editor_ui.touch_chrome() {
            return None;
        }
        let surface = WorkspaceSurface::for_editor_at(&self.editor_state, self.now_ms)?;
        let layout = surface.layout(viewport_width, viewport_height);
        let hit = surface.hit_test_layout(&layout, Point2D::new(x, y))?;
        self.editor_state.editor_ui.workspace.pressed = Some(hit);
        self.run_workspace_action(hit, &layout, x);
        self.mark_dirty();
        Some(true)
    }

    fn run_workspace_action(&mut self, hit: WorkspaceHit, layout: &WorkspaceLayout, press_x: f32) {
        let board_count = active_page_boards(self.editor_state()).len();
        let viewport_w = layout.header.size.x;
        // canvas bottom + strip height + the header/toolbar band = vh.
        let viewport_h = op_editor_core::WORKSPACE_HEADER_H
            + op_editor_core::WORKSPACE_TOOLBAR_H
            + layout.canvas.size.y
            + layout.strip.map_or(0.0, |strip| strip.size.y);
        match hit {
            WorkspaceHit::Back | WorkspaceHit::ReturnEdit => {
                // Home is a takeover painted above the workspace; the
                // workspace stays active so Home's footer offers
                // 回到工作区 and the generation keeps running.
                self.editor_state.editor_ui.home.visible = true;
                self.editor_state.editor_ui.entry_surface = EntrySurface::Home;
            }
            WorkspaceHit::Export => {
                self.editor_state.editor_ui.open_export_dialog();
            }
            WorkspaceHit::Professional => {
                let workspace = &mut self.editor_state.editor_ui.workspace;
                let restore = workspace.previous_tool;
                workspace.enter_professional();
                self.editor_state.tool = restore.unwrap_or(Tool::Select);
            }
            WorkspaceHit::ToggleDock => {
                // The dock's collapse toggle collapses the LEFT PANEL:
                // one column, one open flag.
                self.editor_state.editor_ui.sidebar_open =
                    !self.editor_state.editor_ui.sidebar_open;
            }
            WorkspaceHit::DockResize => {
                self.workspace_dock_drag = Some(WorkspaceDockDrag {
                    start_x: press_x,
                    start_w: self.editor_state.editor_ui.layer_panel_width,
                });
            }
            WorkspaceHit::View(view) => {
                let selected = self.editor_state.editor_ui.workspace.selected;
                let workspace = &mut self.editor_state.editor_ui.workspace;
                workspace.view = match view {
                    WorkspaceView::Single { .. } => WorkspaceView::Single {
                        index: selected.min(board_count.saturating_sub(1)),
                    },
                    other => other,
                };
                self.apply_workspace_fit(viewport_w, viewport_h);
            }
            WorkspaceHit::Prev | WorkspaceHit::Next => {
                let delta = if matches!(hit, WorkspaceHit::Prev) {
                    -1
                } else {
                    1
                };
                let moved = self
                    .editor_state
                    .editor_ui
                    .workspace
                    .step_selected(delta, board_count);
                if moved {
                    self.frame_workspace_board(viewport_w, viewport_h);
                }
            }
            WorkspaceHit::ZoomOut | WorkspaceHit::ZoomIn => {
                let zoom_in = matches!(hit, WorkspaceHit::ZoomIn);
                status_bar_zoom(&mut self.editor_state, zoom_in, viewport_w, viewport_h);
                self.note_viewport_zoom_gesture();
            }
            WorkspaceHit::ZoomFit => {
                self.apply_workspace_fit(viewport_w, viewport_h);
            }
            WorkspaceHit::Thumb(index) => {
                let moved = {
                    let workspace = &mut self.editor_state.editor_ui.workspace;
                    let before = (workspace.selected, workspace.view);
                    workspace.select_board(index, board_count);
                    before != (workspace.selected, workspace.view)
                };
                if moved {
                    self.frame_workspace_board(viewport_w, viewport_h);
                }
            }
            WorkspaceHit::Overview => {
                let workspace = &mut self.editor_state.editor_ui.workspace;
                workspace.view = if workspace.view == WorkspaceView::Overview {
                    WorkspaceView::Single {
                        index: workspace.selected,
                    }
                } else {
                    WorkspaceView::Overview
                };
                self.apply_workspace_fit(viewport_w, viewport_h);
            }
            WorkspaceHit::Play => {
                self.toggle_preview_with_cached_viewport();
            }
            WorkspaceHit::Retry => {
                self.retry_workspace_brief();
            }
        }
    }

    /// Re-send the stored brief through the orchestrator route — the
    /// same three-call pattern Home's send uses, without leaving the
    /// workspace.
    fn retry_workspace_brief(&mut self) {
        let workspace = &self.editor_state.editor_ui.workspace;
        let family = workspace.family;
        let brief = workspace.brief.trim().to_string();
        let mut options = workspace.options.clone();
        options.text = brief;
        let Some(prompt) = family.generation_prompt(&options) else {
            return;
        };
        self.editor_state.editor_ui.workspace.resume_generating(0);
        self.editor_state.chat.focus_input_at_end(self.now_ms);
        self.editor_state.chat.set_input_text(prompt);
        self.editor_state.chat.launch_route = op_editor_core::LaunchRoute::Orchestrator;
        let sent = self.editor_state.chat.begin_send();
        self.editor_state.chat.focused = false;
        if sent {
            self.mark_dirty();
        }
    }

    /// Frame the deck's selected board (thumb clicks, pager, arrows).
    fn frame_workspace_board(&mut self, viewport_w: f32, viewport_h: f32) {
        self.refresh_layout_scene();
        let board_id = active_page_boards(self.editor_state())
            .get(self.editor_state.editor_ui.workspace.selected)
            .cloned();
        if let Some(id) = board_id {
            zoom_to_fit_node(
                &mut self.editor_state,
                &self.layout_scene,
                &id,
                viewport_w,
                viewport_h,
            );
        }
    }

    /// Apply the camera fit for the workspace's current view mode.
    pub fn apply_workspace_fit(&mut self, viewport_w: f32, viewport_h: f32) {
        self.refresh_layout_scene();
        let view = self.editor_state.editor_ui.workspace.view;
        let boards = active_page_boards(self.editor_state());
        match view {
            WorkspaceView::AllBoards | WorkspaceView::Overview => {
                zoom_to_fit(
                    &mut self.editor_state,
                    &self.layout_scene,
                    viewport_w,
                    viewport_h,
                );
            }
            WorkspaceView::Single { index } => {
                let id = boards.get(index).or_else(|| boards.first()).cloned();
                if let Some(id) = id {
                    zoom_to_fit_node(
                        &mut self.editor_state,
                        &self.layout_scene,
                        &id,
                        viewport_w,
                        viewport_h,
                    );
                }
            }
            WorkspaceView::LongPage => {
                self.fit_workspace_long_page(boards.first().cloned(), viewport_w, viewport_h);
            }
        }
    }

    /// LongPage: fit the board WIDTH into the canvas width minus 48 px
    /// and centre it vertically; the wheel pans.
    fn fit_workspace_long_page(
        &mut self,
        board_id: Option<String>,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        let Some(id) = board_id else {
            return;
        };
        let Some(node) = self
            .layout_scene
            .active_page()
            .and_then(|page| page.find(&id))
        else {
            return;
        };
        let bounds: Rect = node.bounds;
        if bounds.size.x <= 0.0 {
            return;
        }
        let (_, _, canvas_w, canvas_h) = self.canvas_region(viewport_w, viewport_h);
        let viewport = &mut self.editor_state.viewport;
        viewport.zoom = ((canvas_w - 48.0) / bounds.size.x).clamp(
            op_editor_core::Viewport::MIN_ZOOM,
            op_editor_core::Viewport::MAX_ZOOM,
        );
        let centre_x = bounds.origin.x + bounds.size.x / 2.0;
        let centre_y = bounds.origin.y + bounds.size.y / 2.0;
        viewport.pan_x = canvas_w / 2.0 - centre_x * viewport.zoom;
        viewport.pan_y = canvas_h / 2.0 - centre_y * viewport.zoom;
    }

    /// ← / → paging in the deck (desktop keyboard arm).
    pub fn workspace_step_board(
        &mut self,
        forward: bool,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        if !self.workspace_visible() {
            return false;
        }
        let delta = if forward { 1 } else { -1 };
        let count = active_page_boards(self.editor_state()).len();
        let moved = self
            .editor_state
            .editor_ui
            .workspace
            .step_selected(delta, count);
        if moved {
            self.frame_workspace_board(viewport_w, viewport_h);
            self.mark_dirty();
        }
        moved
    }

    /// Hover bookkeeping + the live dock-width drag. `None` when the
    /// point is over no workspace chrome so the chat / canvas hover
    /// tiers still run — the workspace is not a takeover. A hover that
    /// just cleared still repaints before falling through.
    pub(in crate::widget_host) fn cursor_move_workspace(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<bool> {
        if !self.workspace_visible() || self.editor_state.editor_ui.touch_chrome() {
            return None;
        }
        // A live dock drag follows the cursor even off the handle.
        if let Some(drag) = self.workspace_dock_drag {
            self.editor_state
                .editor_ui
                .set_layer_panel_width(drag.start_w + (x - drag.start_x));
            self.mark_dirty();
            return Some(true);
        }
        let surface = WorkspaceSurface::for_editor_at(&self.editor_state, self.now_ms)?;
        let layout = surface.layout(viewport_w, viewport_h);
        let next = surface.hit_test_layout(&layout, Point2D::new(x, y));
        if self.editor_state.editor_ui.workspace.hover == next {
            return next.map(|_| true);
        }
        self.editor_state.editor_ui.workspace.hover = next;
        self.mark_dirty();
        next.map(|_| true)
    }

    /// End the dock drag (pointer release).
    pub(in crate::widget_host) fn release_workspace_drag(&mut self) -> bool {
        if self.workspace_dock_drag.take().is_some() {
            self.mark_dirty();
            return true;
        }
        if self
            .editor_state
            .editor_ui
            .workspace
            .pressed
            .take()
            .is_some()
        {
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Whether the cursor sits on the dock's resize handle (the runner
    /// maps this to an EW-resize cursor hint).
    pub(in crate::widget_host) fn workspace_dock_resize_hover(
        &self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        if !self.editor_state.editor_ui.chat_pinned() {
            return false;
        }
        WorkspaceSurface::for_editor_at(&self.editor_state, self.now_ms)
            .and_then(|surface| {
                let layout = surface.layout(viewport_w, viewport_h);
                surface
                    .hit_test_layout(&layout, Point2D::new(x, y))
                    .map(|hit| matches!(hit, WorkspaceHit::DockResize))
            })
            .unwrap_or(false)
    }

    /// The per-frame generation pump the desktop runner drives while a
    /// workspace is active: refresh boards, clamp the selection, and —
    /// while Generating in AllBoards — refit the camera once per NEW
    /// board count so the user watches boards land. Returns whether
    /// anything changed.
    pub fn pump_workspace_generation(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
        generating: bool,
    ) -> bool {
        let workspace = &self.editor_state.editor_ui.workspace;
        if !workspace.active || !workspace.visible {
            return false;
        }
        let boards = active_page_boards(&self.editor_state);
        let count = boards.len();
        let selected = workspace.selected.min(count.saturating_sub(1));
        // Refit on every new board WHATEVER the view mode is. Gating this
        // on AllBoards meant the families that open in another view —
        // 演示文稿 on Single, 网页 / 信息图 on LongPage — never refit at
        // all during a run, so their boards landed under the stale camera
        // the blank starter left behind and spilled off the canvas
        // (measured 2026-09-13: five 1920×1080 slides drawn at the 1200×800
        // starter's zoom, clipped on the right, nothing centred).
        // Refit whenever the CONTENT MOVES, not merely when a board is
        // added: the orchestrator re-flows boards it already created into
        // rows and resizes them, so the count can sit at 5 while the
        // bounds change twice. Comparing bounds also settles by itself —
        // once the deck stops moving the camera stops, so a manual pan
        // later in the run is left alone.
        // Only a live run touches the scene here: the bounds probe must
        // not make every idle workspace frame rebuild the layout.
        if generating {
            self.refresh_layout_scene();
        }
        let bounds = generating
            .then(|| self.layout_scene.content_bounds())
            .flatten()
            .map(|content| {
                (
                    content.origin.x,
                    content.origin.y,
                    content.size.x,
                    content.size.y,
                )
            });
        let bounds_moved = match (bounds, self.editor_state.editor_ui.workspace.fitted_bounds) {
            (Some(now), Some(then)) => {
                let moved = (now.0 - then.0).abs()
                    + (now.1 - then.1).abs()
                    + (now.2 - then.2).abs()
                    + (now.3 - then.3).abs();
                moved > 0.5
            }
            (Some(_), None) => true,
            _ => false,
        };
        let refit_due = generating && count > 0 && bounds_moved;
        if !refit_due && selected == self.editor_state.editor_ui.workspace.selected {
            return false;
        }
        self.editor_state.editor_ui.workspace.selected = selected;
        if refit_due {
            self.editor_state.editor_ui.workspace.fitted_board_count = count;
            self.editor_state.editor_ui.workspace.fitted_bounds = bounds;
            // The scene was refreshed above to read the bounds. While
            // the run streams, "watch the boards land" is an all-boards
            // camera no matter which view the family settles into; the
            // settled view is applied once the run finishes.
            zoom_to_fit(
                &mut self.editor_state,
                &self.layout_scene,
                viewport_w,
                viewport_h,
            );
        }
        true
    }

    /// Resolve the workspace phase from real chat/orchestrator state.
    /// Called by the desktop runner on its idle edge; `epoch` is the
    /// run epoch the edge belongs to (the workspace's own stamp when
    /// known). Zero boards or an error transcript is a failure.
    pub fn settle_workspace_idle_edge(
        &mut self,
        epoch: u64,
        board_count: usize,
        last_assistant_failed: bool,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        let workspace = &mut self.editor_state.editor_ui.workspace;
        if !workspace.active || workspace.phase != WorkspacePhase::Generating {
            return false;
        }
        let changed = if board_count == 0 || last_assistant_failed {
            workspace.mark_failed(epoch)
        } else {
            workspace.mark_done(epoch)
        };
        if changed {
            // The generation camera tracked every board as it landed;
            // the finished run hands the canvas over to the view the
            // family actually settles into (演示文稿 frames slide 1,
            // 网页 / 信息图 fit the page width).
            if self.editor_state.editor_ui.workspace.phase == WorkspacePhase::Done {
                self.apply_workspace_fit(viewport_w, viewport_h);
            }
            self.mark_dirty();
        }
        changed
    }
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;
