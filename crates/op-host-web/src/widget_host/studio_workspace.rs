//! Studio generation-workspace input routing on the web widget host.
//!
//! Web twin of `op-host-native/src/widget_host/workspace.rs`. The workspace
//! is NOT a takeover like Home: its chrome (header, toolbar, deck strip,
//! dock handle, banners, quality report) claims presses ahead of the top bar
//! and rails, while the pinned chat and the docked canvas keep their
//! ordinary tiers. Geometry and hit-testing are the shared
//! `WorkspaceSurface`; the per-frame phase pump lives in
//! `studio_workspace_run.rs`.

use super::WidgetHost;
use op_editor_core::preview_slideshow::active_page_boards;
use op_editor_core::{EntrySurface, Tool, WorkspaceHit, WorkspaceView};
use op_editor_ui::widgets::host_overlay_geometry::{
    status_bar_zoom, zoom_to_fit, zoom_to_fit_node,
};
use op_editor_ui::widgets::WorkspaceSurface;
use op_editor_ui::{Point2D, Rect};

/// A live dock-width drag: the press x and the left panel's width at press
/// (the dock IS the left panel, so the drag writes `layer_panel_width`).
#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct WorkspaceDockDrag {
    pub(in crate::widget_host) start_x: f32,
    pub(in crate::widget_host) start_w: f32,
}

/// Narrowest the conversation dock may be dragged (the Studio spec keeps it
/// at 280–440; under 280 the composer and model chip start to clip).
const WORKSPACE_DOCK_MIN_WIDTH: f32 = 280.0;

impl WidgetHost {
    /// The workspace is a pointer surface; the browser never runs touch
    /// chrome, but the gate mirrors native so the two cannot drift.
    pub(crate) fn workspace_visible(&self) -> bool {
        self.editor_state.editor_ui.workspace.visible && !self.editor_state.editor_ui.touch_chrome()
    }

    /// The chrome tier — after Home and the modal tiers, ahead of the top
    /// bar / rails. `None` lets the chat and canvas tiers run.
    pub(in crate::widget_host) fn press_workspace(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Option<bool> {
        if !self.workspace_visible() || self.preview_slideshow_active() {
            return None;
        }
        let surface = WorkspaceSurface::for_editor_at(&self.editor_state, self.now_ms)?;
        let layout = surface.layout(viewport_width, viewport_height);
        let hit = surface.hit_test_layout(&layout, Point2D::new(x, y));
        drop(surface);
        // An open report panel closes on any press outside it (the press
        // itself still reaches whatever is underneath).
        if self.editor_state.editor_ui.workspace.quality_open
            && !matches!(
                hit,
                Some(
                    WorkspaceHit::QualityChip
                        | WorkspaceHit::QualityPanel
                        | WorkspaceHit::QualityItem { .. }
                )
            )
        {
            self.editor_state.editor_ui.workspace.quality_open = false;
            self.mark_dirty();
        }
        let hit = hit?;
        self.editor_state.editor_ui.workspace.pressed = Some(hit);
        self.run_workspace_action(hit, x, viewport_width, viewport_height);
        self.mark_dirty();
        Some(true)
    }

    fn run_workspace_action(
        &mut self,
        hit: WorkspaceHit,
        press_x: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        let board_count = active_page_boards(&self.editor_state).len();
        match hit {
            WorkspaceHit::Back | WorkspaceHit::ReturnEdit => {
                // Home is a takeover painted above the workspace; the
                // workspace stays active so Home's footer offers 回到工作区
                // and the generation keeps running.
                self.editor_state.editor_ui.home.visible = true;
                self.editor_state.editor_ui.entry_surface = EntrySurface::Home;
            }
            WorkspaceHit::Export => self.editor_state.editor_ui.open_export_dialog(),
            WorkspaceHit::UseVariant(index) => {
                self.use_workspace_variant(index, viewport_w, viewport_h);
            }
            // The share page needs a save picker plus the offscreen
            // rasteriser, so the header button is gated on
            // `deck_html_export_supported`, which web leaves `false`.
            WorkspaceHit::Share => {}
            // Web never opens a document into the shared view, but the
            // prefill itself is host-free, so the press still works.
            WorkspaceHit::MakeSame => {
                self.editor_state.editor_ui.begin_make_same(self.now_ms);
            }
            // Web never enters drawer mode (the chat dock stays docked).
            WorkspaceHit::DrawerScrim => {}
            WorkspaceHit::Professional => {
                let workspace = &mut self.editor_state.editor_ui.workspace;
                let restore = workspace.previous_tool;
                workspace.enter_professional();
                self.editor_state.tool = restore.unwrap_or(Tool::Select);
            }
            WorkspaceHit::ToggleDock => {
                // The dock's collapse toggle collapses the LEFT PANEL.
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
                let workspace = &mut self.editor_state.editor_ui.workspace;
                let selected = workspace.selected;
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
                if self
                    .editor_state
                    .editor_ui
                    .workspace
                    .step_selected(delta, board_count)
                {
                    self.frame_workspace_board(viewport_w, viewport_h);
                }
            }
            WorkspaceHit::ZoomOut | WorkspaceHit::ZoomIn => {
                let zoom_in = matches!(hit, WorkspaceHit::ZoomIn);
                status_bar_zoom(&mut self.editor_state, zoom_in, viewport_w, viewport_h);
            }
            WorkspaceHit::ZoomFit => self.apply_workspace_fit(viewport_w, viewport_h),
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
            WorkspaceHit::Play => self.toggle_workspace_play(viewport_w, viewport_h),
            WorkspaceHit::Retry => {
                // A template draft's run is a refine of those boards:
                // retrying refines them again in place.
                if self
                    .editor_state
                    .editor_ui
                    .workspace
                    .draft_template
                    .is_some()
                {
                    self.queue_draft_refine();
                } else {
                    self.retry_workspace_brief();
                }
            }
            WorkspaceHit::DraftAction => self.run_workspace_draft_action(),
            WorkspaceHit::QualityChip => {
                let workspace = &mut self.editor_state.editor_ui.workspace;
                workspace.quality_open = !workspace.quality_open;
            }
            WorkspaceHit::QualityPanel => {}
            WorkspaceHit::QualityItem { topic, item } => {
                self.focus_quality_item(topic, item, viewport_w, viewport_h);
            }
        }
    }

    /// Play — the same preview entry the TopBar toggle uses. The surface
    /// only reports the hit while `play_enabled()` (a finished run with
    /// boards), so no gating is repeated here.
    fn toggle_workspace_play(&mut self, viewport_w: f32, viewport_h: f32) {
        if self.preview.is_some() {
            self.exit_preview(viewport_w, viewport_h);
            return;
        }
        let op_ck = self.op_ck.clone();
        let _ = self.enter_preview_from_browser(viewport_w, viewport_h, op_ck.as_ref());
    }

    /// A remaining-issue row was clicked: select the node it names and frame
    /// it. Rows without a node and nodes that no longer exist do nothing.
    fn focus_quality_item(&mut self, topic: usize, item: usize, viewport_w: f32, viewport_h: f32) {
        let Some(node_id) = self
            .editor_state
            .editor_ui
            .workspace
            .quality
            .as_ref()
            .and_then(|report| report.remaining_item(topic, item))
            .and_then(|entry| entry.node_id.clone())
        else {
            return;
        };
        let selected = self
            .editor_state
            .apply(op_editor_core::EditorCommand::SetSelection {
                node_id: op_editor_core::NodeId::new(node_id.clone()),
            });
        if !selected {
            return;
        }
        self.refresh_layout_scene();
        zoom_to_fit_node(
            &mut self.editor_state,
            &self.layout_scene,
            &node_id,
            viewport_w,
            viewport_h,
        );
    }

    /// Re-send the stored brief through the orchestrator route without
    /// leaving the workspace. A stopped run may have drawn part of the
    /// design; retrying starts on a fresh page instead of stacking a second
    /// attempt over the first (see `studio_home_send.rs` for why this swap
    /// does not ask).
    pub(in crate::widget_host) fn retry_workspace_brief(&mut self) {
        let workspace = &self.editor_state.editor_ui.workspace;
        let family = workspace.family;
        let mut options = workspace.options.clone();
        options.text = workspace.brief.trim().to_string();
        let Some(prompt) = family.generation_prompt(&options) else {
            return;
        };
        if self.editor_state.editor_ui.workspace.phase == op_editor_core::WorkspacePhase::Stopped
            && !op_editor_core::blank_starter::active_page_is_blank_starter(&self.editor_state)
        {
            // The swap resets the workspace; restore the run it belongs to.
            let saved = self.editor_state.editor_ui.workspace.clone();
            self.start_fresh_document_for_home();
            self.editor_state.editor_ui.workspace = saved;
        }
        self.editor_state.editor_ui.workspace.resume_generating(0);
        self.editor_state.chat.focus_input_at_end(self.now_ms);
        self.editor_state.chat.set_input_text(prompt);
        // A side-by-side run retries as one: the same number of directions.
        op_editor_core::pin_workspace_retry_route(&mut self.editor_state);
        let sent = self.begin_chat_send();
        self.editor_state.chat.focused = false;
        if sent {
            self.mark_dirty();
        }
    }

    /// Keep direction `index` of a side-by-side run as the working design
    /// (`op_editor_core::use_workspace_variant`: the others move to their
    /// own page) and refit the camera on the one that stayed. The edit
    /// reaches the daemon like any other local edit, through live sync.
    pub(in crate::widget_host) fn use_workspace_variant(
        &mut self,
        index: usize,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        let Some(changed) = op_editor_core::use_workspace_variant(&mut self.editor_state, index)
        else {
            return false;
        };
        self.apply_workspace_fit(viewport_w, viewport_h);
        self.mark_dirty();
        changed
    }

    /// Frame the deck's selected board (thumb clicks, pager, arrows).
    fn frame_workspace_board(&mut self, viewport_w: f32, viewport_h: f32) {
        self.refresh_layout_scene();
        let board_id = active_page_boards(&self.editor_state)
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
    pub(in crate::widget_host) fn apply_workspace_fit(&mut self, viewport_w: f32, viewport_h: f32) {
        self.refresh_layout_scene();
        let boards = active_page_boards(&self.editor_state);
        match self.editor_state.editor_ui.workspace.view {
            WorkspaceView::AllBoards | WorkspaceView::Overview => {
                zoom_to_fit(
                    &mut self.editor_state,
                    &self.layout_scene,
                    viewport_w,
                    viewport_h,
                );
            }
            WorkspaceView::Single { index } => {
                if let Some(id) = boards.get(index).or_else(|| boards.first()).cloned() {
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

    /// LongPage: fit the board WIDTH into the canvas width minus 48 px and
    /// centre it vertically; the wheel pans.
    fn fit_workspace_long_page(
        &mut self,
        board_id: Option<String>,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        let Some(id) = board_id else {
            return;
        };
        let Some(bounds) = self
            .layout_scene
            .active_page()
            .and_then(|page| page.find(&id))
            .map(|node| node.bounds)
        else {
            return;
        };
        let bounds: Rect = bounds;
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

    /// ← / → step the deck while the workspace is up in a paging view. Runs
    /// after the text-caret arms and before the nudge, like the desktop
    /// runner; `false` when nothing moved so the nudge still gets the key.
    pub fn apply_workspace_step_board(&mut self, forward: bool) -> bool {
        let paging = self.workspace_visible()
            && matches!(
                self.editor_state.editor_ui.workspace.view,
                WorkspaceView::Single { .. } | WorkspaceView::Overview
            );
        if !paging {
            return false;
        }
        let count = active_page_boards(&self.editor_state).len();
        let delta = if forward { 1 } else { -1 };
        let moved = self
            .editor_state
            .editor_ui
            .workspace
            .step_selected(delta, count);
        if moved {
            self.frame_workspace_board(self.last_viewport_w, self.last_viewport_h);
            self.mark_dirty();
        }
        moved
    }

    /// Hover bookkeeping + the live dock-width drag. `None` over no
    /// workspace chrome so the chat / canvas hover tiers still run.
    pub(in crate::widget_host) fn cursor_move_workspace(&mut self, x: f32, y: f32) -> Option<bool> {
        if !self.workspace_visible() {
            return None;
        }
        if let Some(drag) = self.workspace_dock_drag {
            let width = (drag.start_w + (x - drag.start_x)).clamp(
                WORKSPACE_DOCK_MIN_WIDTH,
                op_editor_core::LAYER_PANEL_MAX_WIDTH,
            );
            self.editor_state.editor_ui.set_layer_panel_width(width);
            self.mark_dirty();
            return Some(true);
        }
        let surface = WorkspaceSurface::for_editor_at(&self.editor_state, self.now_ms)?;
        let layout = surface.layout(self.last_viewport_w, self.last_viewport_h);
        let next = surface.hit_test_layout(&layout, Point2D::new(x, y));
        drop(surface);
        if self.editor_state.editor_ui.workspace.hover == next {
            return next.map(|_| false);
        }
        self.editor_state.editor_ui.workspace.hover = next;
        self.mark_dirty();
        next.map(|_| true)
    }

    /// End the dock drag / pressed state (pointer release).
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
        }
        false
    }

    /// Wheel over a LongPage workspace pans the page vertically instead of
    /// zooming (a modified wheel still zooms). `delta_y` is the browser
    /// router's scroll delta (positive towards the top, the same value the
    /// panels scroll by). `false` when not applicable.
    pub fn apply_workspace_long_page_wheel(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        zoom_modifier: bool,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        if zoom_modifier
            || !self.workspace_visible()
            || self.home_visible()
            || self.editor_state.editor_ui.workspace.view != WorkspaceView::LongPage
            || !self.over_canvas(x, y, viewport_w, viewport_h)
        {
            return false;
        }
        // Camera only — the layout cache stays intact (like every canvas
        // pan); the wheel listener repaints off the `true`.
        self.editor_state.viewport.pan(0.0, delta_y);
        true
    }
}
