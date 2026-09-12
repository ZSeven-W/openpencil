//! Native-host wiring for the post-generation 成品视图.
//!
//! Like Home, the result view is a full-surface takeover: while visible
//! it owns every press / hover / wheel event before any lower tier can
//! see them. This module routes its input, runs its six actions, paints
//! the widget, and blits the REAL board renders (through the same
//! `SlideThumbCache` the slides rail uses, keyed by board id + raster
//! size, so both surfaces share one bounded renderer) over the widget's
//! placeholder slots.

use super::WidgetHostNative;
use crate::backend::NativeFrameBackend;
use op_editor_core::{EntrySurface, NodeId, ResultHit};
use op_editor_ui::widgets::{board_enter, ResultViewSurface, Widget, BOARD_RADIUS};
use op_editor_ui::{Point2D, Rect, RenderBackend};

impl WidgetHostNative {
    pub fn result_view_visible(&self) -> bool {
        self.editor_state.editor_ui.result_view.visible
    }

    /// The takeover press. `Some(true)` for ANY point while the view is
    /// visible — a click on empty stage paper is still the result view's
    /// click, never the canvas's.
    pub(in crate::widget_host) fn press_result_view(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Option<bool> {
        if !self.result_view_visible() {
            return None;
        }
        let point = Point2D::new(x, y);
        let surface = ResultViewSurface::for_editor_at(&self.editor_state, self.now_ms)?;
        let hit = surface.hit_test(viewport_width, viewport_height, point);
        if let Some(hit) = hit {
            self.editor_state.editor_ui.result_view.pressed = Some(hit);
            self.run_result_action(hit, viewport_width, viewport_height);
        }
        self.mark_dirty();
        Some(true)
    }

    /// Hover bookkeeping, mirroring `cursor_move_home`.
    pub(in crate::widget_host) fn cursor_move_result_view(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Option<bool> {
        if !self.result_view_visible() {
            return None;
        }
        let surface = ResultViewSurface::for_editor_at(&self.editor_state, self.now_ms)?;
        let next = surface.hit_test(viewport_width, viewport_height, Point2D::new(x, y));
        if self.editor_state.editor_ui.result_view.hover == next {
            return Some(true);
        }
        self.editor_state.editor_ui.result_view.hover = next;
        self.mark_dirty();
        Some(true)
    }

    /// Nothing scrolls under the takeover — the wheel is swallowed while
    /// the view is up, exactly like a modal.
    pub(in crate::widget_host) fn try_scroll_result_view(&mut self) -> bool {
        self.result_view_visible()
    }

    fn run_result_action(&mut self, hit: ResultHit, viewport_w: f32, viewport_h: f32) {
        let view = &mut self.editor_state.editor_ui.result_view;
        match hit {
            ResultHit::BackHome => {
                view.hide();
                self.editor_state.editor_ui.home.visible = true;
                self.editor_state.editor_ui.entry_surface = EntrySurface::Home;
            }
            // Professional, FullEdit and the Escape key all mean the same
            // thing: drop to the canvas and stay there.
            ResultHit::Professional | ResultHit::FullEdit => {
                view.hide();
            }
            ResultHit::Screen(index) => {
                view.selected = index.min(view.root_ids.len().saturating_sub(1));
            }
            ResultHit::EditThisScreen => {
                let selected = view.selected;
                let Some(root_id) = view.root_ids.get(selected).cloned() else {
                    view.hide();
                    return;
                };
                view.hide();
                self.editor_state
                    .set_single_selection(NodeId::new(root_id.clone()));
                self.refresh_layout_scene();
                op_editor_ui::widgets::host_overlay_geometry::zoom_to_fit_node(
                    &mut self.editor_state,
                    &self.layout_scene,
                    &root_id,
                    viewport_w,
                    viewport_h,
                );
                // Land the user in the composer with the edit prefix
                // typed but NOT sent — they name the change themselves.
                self.editor_state.chat.focus_input_at_end(self.now_ms);
                self.editor_state.chat.set_input_text("改这一页：");
                self.editor_state.chat.focused = true;
            }
            ResultHit::PlayPrototype => {
                view.hide();
                self.toggle_preview_with_cached_viewport();
            }
            ResultHit::ComponentsVariables => {
                view.hide();
                self.editor_state.editor_ui.toggle_variables_panel();
            }
            ResultHit::Restyle => {
                view.hide();
                // Reopen once the restyle turn finishes, then launch it
                // through the same three-call pattern as Home's send.
                view.arm_for_generation(
                    view.family.unwrap_or(op_editor_core::HomeFamily::AppUi),
                    view.brief.clone(),
                );
                self.editor_state.chat.focus_input_at_end(self.now_ms);
                self.editor_state
                    .chat
                    .set_input_text("换一种视觉风格，保持结构、内容和交互不变");
                self.editor_state.chat.begin_send();
                self.editor_state.chat.focused = false;
            }
            ResultHit::Export => {
                view.hide();
                self.editor_state.editor_ui.open_export_dialog();
            }
        }
    }

    /// Paint the takeover: widget first, real board rasters over the
    /// placeholder slots, then top the shared thumbnail cache up within
    /// the frame budget and arm the next wake while work remains.
    pub(in crate::widget_host) fn paint_result_view(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        // Refresh BEFORE the surface borrows the editor state for the
        // rest of the frame — the blit below resolves scene nodes through
        // the fresh scene.
        self.refresh_layout_scene();
        let Some(surface) = ResultViewSurface::for_editor_at(&self.editor_state, self.now_ms)
        else {
            return;
        };
        {
            let mut cx = op_editor_ui::widgets::PaintCx {
                backend: &mut *frame,
            };
            surface.paint(&mut cx, Rect::xywh(0.0, 0.0, viewport_w, viewport_h));
        }
        let layout = surface.layout(viewport_w, viewport_h);
        // Blit whatever rasters are already cached (stale is fine — a
        // board one edit behind reads far better than a hole), with the
        // same entrance fade / rise / hover lift the widget applies.
        let shown_at_ms = surface.state.shown_at_ms;
        for (index, board) in surface.boards.iter().enumerate() {
            let Some(image) = self.slide_thumbs.image(&board.id) else {
                continue;
            };
            let Some(slot) = surface.screen_draw_rect(&layout, index) else {
                continue;
            };
            let (_, alpha) = board_enter(index, shown_at_ms, self.now_ms);
            let image = image.clone();
            frame.save();
            frame.clip_round_rect(slot, BOARD_RADIUS);
            frame.draw_offscreen_layer_to_with_alpha(&image, slot, slot, alpha);
            frame.restore();
        }

        // Top the cache up: the result view lists every board, each at
        // its own fitted raster size, so entries stay keyed by
        // (board id, size) and coexist with the slides rail's rasters.
        let revision = self.editor_state.document_revision();
        let ids: Vec<String> = surface.boards.iter().map(|b| b.id.clone()).collect();
        self.slide_thumbs.retain_boards(&ids);
        if !self.slide_thumbs.tick(revision, self.now_ms) {
            // Document still settling — keep the stale rasters up; the
            // entrance motion keeps frames coming meanwhile.
            return;
        }
        let Some(page) = self.layout_scene.active_page() else {
            return;
        };
        let mut wanted: Vec<(String, &op_editor_ui::layout_scene::SceneNode, Point2D)> = Vec::new();
        for (index, board) in surface.boards.iter().enumerate() {
            if let Some(node) = page.find(&board.id) {
                if let Some(slot) = layout.screens.get(index) {
                    wanted.push((board.id.clone(), node, slot.size));
                }
            }
        }
        let rendered = self.slide_thumbs.render_pending(frame, &wanted, revision);
        if rendered || self.slide_thumbs.has_pending(frame, &wanted, revision) {
            self.slide_thumbs.wake_in(self.now_ms, 16);
        } else {
            self.slide_thumbs.settle();
        }
    }
}

#[cfg(test)]
#[path = "result_view_tests.rs"]
mod tests;
