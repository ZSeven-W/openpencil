//! The phone works reader on the native widget host: its press tier, the
//! stage's one-finger scroll / swipe, the board framing, and the paint arm.
//!
//! The reader is a takeover over the SAME document the professional
//! canvas edits: its stage IS the canvas (`canvas_region` answers the
//! stage while the reader is up), framed on one board at a time. Actions
//! reuse the workspace's real paths — Stop drains through the chat pump's
//! stop, Retry re-sends the stored brief, 继续对话 / 改这一页 open the
//! existing mobile chat sheet — so no second generation path exists.
//!
//! 改这一页 binds the next send to the board on show: the board becomes
//! the selection (the desktop direct-modify route's scope) and a
//! `PageEditTarget` is staged on the workspace, which the launcher turns
//! into a scoped prompt plus a hard fence (`op_editor_core::
//! workspace_page_edit`).

use super::WidgetHostNative;
use crate::backend::NativeFrameBackend;
use op_editor_core::preview_slideshow::active_page_boards;
use op_editor_core::size_class::MobileSheetKind;
use op_editor_core::{
    infer_reading_family, reads_as_long_page, EntrySurface, NodeId, PenNodeExt, ReaderHit, Tool,
    Viewport,
};
use op_editor_ui::widgets::{PaintCx, Widget, WorksReader};
use op_editor_ui::{Point2D, Rect};

/// A live one-finger drag on the reader's stage.
#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct ReaderStageDrag {
    start_x: f32,
    start_y: f32,
    last_x: f32,
    last_y: f32,
}

/// Horizontal travel that turns a stage drag into a page turn.
const SWIPE_MIN: f32 = 64.0;
/// Breathing room between the framed board and the stage edges.
const READER_FIT_PADDING: f32 = 16.0;
/// Most a small board (a poster, a card) is magnified to fill the stage.
const READER_MAX_ZOOM: f32 = 2.0;
/// How long a works-list tap on a recent file keeps its "open in the
/// reader" intent while the shell loads it.
const READER_ON_OPEN_WINDOW_MS: u64 = 30_000;

impl WidgetHostNative {
    /// Whether the phone works reader owns the screen (and Home is not
    /// painted over it).
    pub fn works_reader_visible(&self) -> bool {
        self.editor_state.editor_ui.works_reader_visible() && !self.home_visible()
    }

    /// The reader's press tier: every press inside the reader is its own
    /// (it is a takeover), except while a mobile sheet is open — the
    /// sheet's own tiers own those presses.
    pub(in crate::widget_host) fn press_works_reader(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<bool> {
        if !self.works_reader_visible() || self.editor_state.editor_ui.mobile_sheet.is_some() {
            return None;
        }
        let hit = {
            let reader = WorksReader::for_editor_at(&self.editor_state, self.now_ms)?;
            let layout = reader.layout(viewport_w, viewport_h);
            reader.hit_test_layout(&layout, Point2D::new(x, y))
        };
        let Some(hit) = hit else {
            // A dead spot (header padding, a disabled button) still
            // belongs to the reader, never to the canvas under it.
            return Some(true);
        };
        if hit != ReaderHit::Stage {
            self.editor_state.editor_ui.workspace.reader_pressed = Some(hit);
        }
        self.run_reader_action(hit, x, y, viewport_w, viewport_h);
        self.mark_dirty();
        Some(true)
    }

    fn run_reader_action(&mut self, hit: ReaderHit, x: f32, y: f32, vw: f32, vh: f32) {
        let board_count = active_page_boards(&self.editor_state).len();
        match hit {
            ReaderHit::Back => {
                // Home is a takeover over the reader; the workspace stays
                // active, so the run keeps going and Home's featured card
                // offers 回到工作区.
                let ui = &mut self.editor_state.editor_ui;
                ui.home.visible = true;
                ui.entry_surface = EntrySurface::Home;
            }
            ReaderHit::ModeNormal => {
                // The reader IS the normal view; nothing to switch.
            }
            ReaderHit::ModeProfessional => self.reader_enter_professional(vw, vh),
            ReaderHit::Prev | ReaderHit::Next => {
                let delta = if hit == ReaderHit::Prev { -1 } else { 1 };
                if self
                    .editor_state
                    .editor_ui
                    .workspace
                    .step_selected(delta, board_count)
                {
                    self.frame_reader_board(vw, vh);
                }
            }
            ReaderHit::Stop => self.reader_stop(),
            ReaderHit::Retry => self.retry_workspace_brief(),
            ReaderHit::ContinueChat => {
                self.editor_state
                    .editor_ui
                    .workspace
                    .clear_staged_page_edit();
                self.open_reader_chat_sheet();
            }
            ReaderHit::EditPage => self.reader_stage_page_edit(),
            ReaderHit::Stage => {
                self.reader_drag = Some(ReaderStageDrag {
                    start_x: x,
                    start_y: y,
                    last_x: x,
                    last_y: y,
                });
            }
        }
    }

    /// 专业: the full mobile canvas over the same document, selection and
    /// history, opened on the board the reader was showing.
    fn reader_enter_professional(&mut self, vw: f32, vh: f32) {
        let board = self.reader_current_board();
        let ui = &mut self.editor_state.editor_ui;
        let restore = ui.workspace.previous_tool;
        ui.workspace.enter_professional();
        ui.entry_surface = EntrySurface::Canvas;
        self.editor_state.tool = restore.unwrap_or(Tool::Select);
        if let Some(board) = board {
            self.refresh_layout_scene();
            op_editor_ui::widgets::host_overlay_geometry::zoom_to_fit_node(
                &mut self.editor_state,
                &self.layout_scene,
                &board,
                vw,
                vh,
            );
        }
    }

    /// Stop through the same drain the chat panel's stop uses; the
    /// workspace settles Stopped immediately so the status line answers
    /// the tap, and the pump's drain retires the worker.
    fn reader_stop(&mut self) {
        let chat = &mut self.editor_state.chat;
        chat.stop_streaming();
        chat.pending_stop_chat = true;
        let workspace = &mut self.editor_state.editor_ui.workspace;
        let epoch = workspace.run_epoch;
        workspace.mark_stopped(epoch);
    }

    /// 改这一页: select the board on show and bind the next send to it,
    /// then open the chat sheet for the instruction.
    fn reader_stage_page_edit(&mut self) {
        let Some(board) = self.reader_current_board() else {
            return;
        };
        let index = self.reader_current_index();
        self.editor_state
            .set_single_selection(NodeId::new(board.as_str()));
        self.editor_state
            .editor_ui
            .workspace
            .stage_page_edit(board, index);
        self.open_reader_chat_sheet();
    }

    fn open_reader_chat_sheet(&mut self) {
        if self.editor_state.editor_ui.mobile_sheet != Some(MobileSheetKind::Ai) {
            self.toggle_mobile_sheet(MobileSheetKind::Ai);
        }
    }

    fn reader_current_index(&self) -> usize {
        let count = active_page_boards(&self.editor_state).len();
        self.editor_state
            .editor_ui
            .workspace
            .selected
            .min(count.saturating_sub(1))
    }

    fn reader_current_board(&self) -> Option<String> {
        active_page_boards(&self.editor_state)
            .get(self.reader_current_index())
            .cloned()
    }

    /// Frame the reader's current board in the stage: one board fitted
    /// whole, or a long page fitted to width and aligned to its top.
    pub fn frame_reader_board(&mut self, viewport_w: f32, viewport_h: f32) -> bool {
        let Some(board) = self.reader_current_board() else {
            return false;
        };
        self.refresh_layout_scene();
        let Some(bounds) = self
            .layout_scene
            .active_page()
            .and_then(|page| page.find(&board))
            .map(|node| node.aggregate_bounds())
        else {
            return false;
        };
        if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
            return false;
        }
        let (_, _, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        let long = reads_as_long_page(self.editor_state.editor_ui.workspace.family);
        let before = self.editor_state.viewport;
        let viewport = &mut self.editor_state.viewport;
        if long {
            viewport.zoom = ((cw - READER_FIT_PADDING * 2.0) / bounds.size.x)
                .clamp(Viewport::MIN_ZOOM, READER_MAX_ZOOM);
            viewport.pan_x = cw / 2.0 - (bounds.origin.x + bounds.size.x / 2.0) * viewport.zoom;
            viewport.pan_y = READER_FIT_PADDING - bounds.origin.y * viewport.zoom;
        } else {
            viewport.fit_to_with_max_zoom(bounds, cw, ch, READER_FIT_PADDING, READER_MAX_ZOOM);
        }
        self.editor_state.viewport != before
    }

    /// The zoom `frame_reader_board` would pick for the current board —
    /// "is the user zoomed in?" is measured against it.
    fn reader_fit_zoom(&self, viewport_w: f32, viewport_h: f32) -> Option<f32> {
        let board = self.reader_current_board()?;
        let bounds = self
            .layout_scene
            .active_page()
            .and_then(|page| page.find(&board))
            .map(|node| node.aggregate_bounds())?;
        let (_, _, cw, ch) = self.canvas_region(viewport_w, viewport_h);
        let pad = READER_FIT_PADDING * 2.0;
        Some(
            ((cw - pad).max(1.0) / bounds.size.x.max(1.0))
                .min((ch - pad).max(1.0) / bounds.size.y.max(1.0))
                .clamp(Viewport::MIN_ZOOM, READER_MAX_ZOOM),
        )
    }

    /// A stage drag follows the finger: a long page scrolls vertically
    /// (clamped to the page), a single board pans freely.
    pub(in crate::widget_host) fn cursor_move_works_reader(
        &mut self,
        x: f32,
        y: f32,
    ) -> Option<bool> {
        let drag = self.reader_drag.as_mut()?;
        let (dx, dy) = (x - drag.last_x, y - drag.last_y);
        drag.last_x = x;
        drag.last_y = y;
        if reads_as_long_page(self.editor_state.editor_ui.workspace.family) {
            self.reader_scroll_long_page(dy);
        } else {
            self.editor_state.viewport.pan(dx, dy);
        }
        self.mark_dirty();
        Some(true)
    }

    /// Scroll a long page by `dy`, never past its top or bottom edge.
    fn reader_scroll_long_page(&mut self, dy: f32) {
        let (vw, vh) = (self.last_viewport_w, self.last_viewport_h);
        let (_, _, _, ch) = self.canvas_region(vw, vh);
        let bounds = self.reader_current_board().and_then(|board| {
            self.layout_scene
                .active_page()
                .and_then(|page| page.find(&board))
                .map(|node| node.aggregate_bounds())
        });
        let viewport = &mut self.editor_state.viewport;
        let Some(bounds) = bounds else {
            viewport.pan(0.0, dy);
            return;
        };
        let top = READER_FIT_PADDING - bounds.origin.y * viewport.zoom;
        let bottom = ch - READER_FIT_PADDING - (bounds.origin.y + bounds.size.y) * viewport.zoom;
        let min = bottom.min(top);
        viewport.pan_y = (viewport.pan_y + dy).clamp(min, top);
    }

    /// End a stage drag or a button press. A single board swiped far
    /// enough sideways turns the page; a pan that did not zoom in snaps
    /// back to the framed board.
    pub(in crate::widget_host) fn release_works_reader(&mut self, vw: f32, vh: f32) -> bool {
        let pressed = self
            .editor_state
            .editor_ui
            .workspace
            .reader_pressed
            .take()
            .is_some();
        let Some(drag) = self.reader_drag.take() else {
            if pressed {
                self.mark_dirty();
            }
            return pressed;
        };
        if reads_as_long_page(self.editor_state.editor_ui.workspace.family) {
            self.mark_dirty();
            return true;
        }
        let (dx, dy) = (drag.last_x - drag.start_x, drag.last_y - drag.start_y);
        let count = active_page_boards(&self.editor_state).len();
        let swiped = dx.abs() >= SWIPE_MIN && dx.abs() > dy.abs() * 1.5;
        let zoomed_in = self
            .reader_fit_zoom(vw, vh)
            .is_some_and(|fit| self.editor_state.viewport.zoom > fit * 1.01);
        if swiped && !zoomed_in {
            let delta = if dx < 0.0 { 1 } else { -1 };
            self.editor_state
                .editor_ui
                .workspace
                .step_selected(delta, count);
        }
        if !zoomed_in {
            self.frame_reader_board(vw, vh);
        }
        self.mark_dirty();
        true
    }

    /// Show the live document in the reader: re-enter the workspace this
    /// document already has, or open a finished-work reading of it (the
    /// 作品 list, a blank canvas the user drew on).
    pub fn open_current_work_in_reader(&mut self, viewport_w: f32, viewport_h: f32) {
        let sizes: Vec<(f64, f64)> = self
            .editor_state
            .active_children()
            .iter()
            .filter(|node| matches!(node, jian_ops_schema::node::PenNode::Frame(_)))
            .filter_map(|node| Some((node.width_px()?, node.height_px()?)))
            .collect();
        let tool = self.editor_state.tool;
        let ui = &mut self.editor_state.editor_ui;
        ui.home.hide();
        ui.entry_surface = EntrySurface::Home;
        if ui.workspace.active {
            ui.workspace.reenter(self.now_ms);
        } else {
            ui.workspace
                .open_for_reading(infer_reading_family(&sizes), self.now_ms);
            ui.workspace.previous_tool = Some(tool);
        }
        self.frame_reader_board(viewport_w, viewport_h);
        self.mark_dirty();
    }

    /// Arm "open the next loaded document in the reader" (a 作品 list tap
    /// on a recent file; the shell performs the load).
    pub(in crate::widget_host) fn arm_reader_on_next_open(&mut self) {
        self.reader_on_next_open_ms = Some(self.now_ms);
    }

    /// Called right after a whole-document swap: honour a fresh 作品 tap.
    pub(in crate::widget_host) fn take_reader_on_open(&mut self) {
        let Some(armed) = self.reader_on_next_open_ms.take() else {
            return;
        };
        if self.now_ms.saturating_sub(armed) > READER_ON_OPEN_WINDOW_MS {
            return;
        }
        if !self.editor_state.editor_ui.compact_layout() {
            return;
        }
        let (vw, vh) = (self.last_viewport_w, self.last_viewport_h);
        self.open_current_work_in_reader(vw, vh);
    }

    /// The current board's rect on screen, from the same camera the
    /// canvas just painted with.
    fn reader_board_screen_rect(&self, viewport_w: f32, viewport_h: f32) -> Option<Rect> {
        let board = self.reader_current_board()?;
        let bounds = self
            .layout_scene
            .active_page()
            .and_then(|page| page.find(&board))
            .map(|node| node.aggregate_bounds())?;
        let (x0, y0, _, _) = self.canvas_region(viewport_w, viewport_h);
        let viewport = &self.editor_state.viewport;
        Some(Rect::xywh(
            x0 + viewport.pan_x + bounds.origin.x * viewport.zoom,
            y0 + viewport.pan_y + bounds.origin.y * viewport.zoom,
            bounds.size.x * viewport.zoom,
            bounds.size.y * viewport.zoom,
        ))
    }

    /// Paint the reader chrome over the canvas the ordinary pass already
    /// painted into the stage.
    pub(in crate::widget_host) fn paint_works_reader(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        if !self.works_reader_visible() {
            return;
        }
        let board_screen = self.reader_board_screen_rect(viewport_w, viewport_h);
        let Some(reader) = WorksReader::for_editor_at(&self.editor_state, self.now_ms) else {
            return;
        };
        let reader = reader.with_board_screen(board_screen);
        let mut cx = PaintCx {
            backend: &mut *frame,
        };
        reader.paint(&mut cx, Rect::xywh(0.0, 0.0, viewport_w, viewport_h));
    }
}

#[cfg(test)]
#[path = "works_reader_tests.rs"]
mod tests;
