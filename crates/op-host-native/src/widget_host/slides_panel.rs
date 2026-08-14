//! Slides-panel arm — the native host's platform tail.
//!
//! Which slides exist, where the rows sit, and what a press / drag /
//! release means all live in `op_editor_ui::widgets::slides_panel*`;
//! this file supplies the rail rect, blits the rendered thumbnails,
//! frames a board when a row is clicked, issues the reorder command
//! when one is dropped, and starts the presentation from the footer.
//! Its web twin (`op-host-web/src/widget_host/slides_panel.rs`) is the
//! same shape over the same shared flow, minus the thumbnails — the
//! browser has no local board renderer.

use super::WidgetHostNative;
use op_editor_core::size_class::MobileSheetKind;
use op_editor_core::LeftPanelTab;
use op_editor_ui::widgets::host_canvas_geometry as canvas_geometry;
use op_editor_ui::widgets::slides_panel_flow as flow;
use op_editor_ui::widgets::{BoardChip, SlidesPanelLayout, SlidesPanelTabs};
use op_editor_ui::{Point2D, Rect, RenderBackend};

use crate::backend::NativeFrameBackend;

/// Everything an event or a paint needs about the panel: the slides,
/// the one the camera is on, and where the rows are.
pub(in crate::widget_host) struct SlidesFrame {
    pub(in crate::widget_host) chips: Vec<BoardChip>,
    pub(in crate::widget_host) active: Option<usize>,
    pub(in crate::widget_host) layout: SlidesPanelLayout,
}

impl WidgetHostNative {
    /// Whether the left rail is actually visible on the current surface.
    ///
    /// Compact and Medium touch layouts show it in the Layers sheet while
    /// keeping `sidebar_open = false`; desktop and Expanded keep using the
    /// persistent sidebar flag.
    pub(in crate::widget_host) fn left_rail_visible(&self) -> bool {
        let ui = &self.editor_state.editor_ui;
        ui.sidebar_open || (ui.touch_chrome() && ui.mobile_sheet == Some(MobileSheetKind::Layers))
    }

    /// Canonical visible rect for rail content.
    ///
    /// A touch sheet owns a shared title / close band. Slides tabs, slide
    /// rows and Layers rows all start below that band; desktop keeps the
    /// original full rail. Every paint, hit and scroll entry point derives
    /// from this answer.
    fn slides_panel_rect(&self, viewport_w: f32, viewport_h: f32) -> Rect {
        if self.editor_state.editor_ui.touch_chrome()
            && self.editor_state.editor_ui.mobile_sheet == Some(MobileSheetKind::Layers)
        {
            let sheet = self.mobile_sheet_rect(viewport_w, viewport_h, MobileSheetKind::Layers);
            return op_editor_ui::widgets::mobile_chrome::sheet_content_rect(sheet);
        }
        canvas_geometry::layer_panel_rect(&self.editor_state, viewport_h)
    }

    /// The tab row, when this document shows one and a persistent rail or
    /// touch Layers sheet is open. `None` covers both "nothing to list"
    /// and "rail surface closed".
    pub(in crate::widget_host) fn slides_tab_row(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<SlidesPanelTabs> {
        if !self.left_rail_visible() {
            return None;
        }
        flow::tab_row(
            &self.editor_state,
            self.slides_panel_rect(viewport_w, viewport_h),
        )
    }

    /// The rail rect the Layers tree gets — the whole rail, less the tab
    /// row when one shows. Paint, hit-test, drag and scroll all derive
    /// from this, so the tab row can never shift the layer rows out from
    /// under their own click targets.
    pub(in crate::widget_host) fn layers_content_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Rect {
        let panel = self.slides_panel_rect(viewport_w, viewport_h);
        if !self.left_rail_visible() {
            return panel;
        }
        flow::layers_content_rect(&self.editor_state, panel)
    }

    /// Resolve the panel for the current document, or `None` when the
    /// slides tab is not on show. Every slides-panel entry point starts
    /// here, so paint and hit-test can never lay the rows out
    /// differently.
    pub(in crate::widget_host) fn slides_panel_frame(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<SlidesFrame> {
        if !self.left_rail_visible() {
            return None;
        }
        let panel = self.slides_panel_rect(viewport_w, viewport_h);
        let chips = flow::slides(&self.editor_state);
        self.refresh_layout_scene();
        let layout = flow::layout(&self.editor_state, &chips, &self.layout_scene, panel)?;
        let canvas = canvas_geometry::canvas_rect(&self.editor_state, viewport_w, viewport_h);
        // "The board nearest the viewport centre" — the same answer the
        // presentation starts from, so the rail can never highlight a
        // different slide than the one Play would open on.
        let active = op_editor_ui::widgets::deck_boards::active_chip_index(
            &chips,
            &self.layout_scene,
            &self.editor_state,
            canvas,
        );
        Some(SlidesFrame {
            chips,
            active,
            layout,
        })
    }

    /// Paint the panel and blit whatever thumbnails are cached, then
    /// render up to the per-frame budget of missing ones.
    pub(in crate::widget_host) fn paint_slides_panel(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        slides: &SlidesFrame,
    ) {
        use op_editor_ui::widgets::PaintCx;
        let (layers_label, slides_label) = flow::tab_labels(&self.editor_state);
        let actions = flow::action_labels(
            &self.editor_state,
            flow::selected_slide_count(&self.editor_state, &slides.chips),
        );
        let widget = flow::widget(
            slides.active,
            &self.editor_state,
            layers_label,
            slides_label,
            actions.labels(),
        );
        {
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            widget.paint(&mut cx, &slides.layout, &self.theme);
        }
        self.blit_slide_thumbnails(frame, slides);
        // After the blit, never before: the number chips and the carried
        // row's ghost sit ON the picture, so painting them with the rest
        // of the widget would bury them under the rasters below.
        {
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            widget.paint_overlay(&mut cx, &slides.layout, &self.theme);
        }
    }

    /// Paint just the tab row, for the frames where the layer tree owns
    /// the rest of the rail.
    pub(in crate::widget_host) fn paint_slides_tab_row(
        &self,
        frame: &mut NativeFrameBackend<'_>,
        tabs: &SlidesPanelTabs,
    ) {
        use op_editor_ui::widgets::PaintCx;
        let (layers_label, slides_label) = flow::tab_labels(&self.editor_state);
        let mut cx = PaintCx { backend: frame };
        tabs.paint(
            &mut cx,
            &self.theme,
            self.editor_state.editor_ui.slides_panel.hover,
            layers_label,
            slides_label,
        );
    }

    /// Draw the cached rasters over the placeholders, then top the cache
    /// up within this frame's budget.
    fn blit_slide_thumbnails(&mut self, frame: &mut NativeFrameBackend<'_>, slides: &SlidesFrame) {
        let visible = slides.layout.visible_rows();
        // Blit first: a row that already has pixels shows them this
        // frame, whether or not it is also due a re-render.
        for (index, _) in &visible {
            let Some(chip) = slides.chips.get(*index) else {
                continue;
            };
            let Some(image) = self.slide_thumbs.image(&chip.id) else {
                continue;
            };
            let Some(clip) = slides.layout.visible_thumb_rect(*index) else {
                continue;
            };
            // Clip to the band, draw at the full box: a half-scrolled row
            // shows the top of its board rather than a squashed copy, and
            // the last row's picture stops at the footer exactly where
            // the widget's own placeholder does. The extra round clip is
            // the plate's own corners — the blit covers the placeholder
            // exactly, so without it a square picture would paint over
            // the rounded plate the widget just drew.
            let image = image.clone();
            let thumb = slides.layout.thumb_rect(*index);
            frame.save();
            frame.clip_round_rect(thumb, op_editor_ui::widgets::SLIDE_THUMB_RADIUS);
            frame.draw_offscreen_layer_to(&image, clip, thumb);
            frame.restore();
        }

        let revision = self.editor_state.document_revision();
        self.slide_thumbs.retain_boards(
            &slides
                .chips
                .iter()
                .map(|c| c.id.clone())
                .collect::<Vec<_>>(),
        );
        if !self.slide_thumbs.tick(revision, self.now_ms) {
            // The document is still moving. Keep the stale rasters on
            // screen; `tick` has already armed the wake for when it
            // settles.
            return;
        }
        let Some(page) = self.layout_scene.active_page() else {
            return;
        };
        // Visible rows first, then the rest — scrolling to a slide must
        // not wait behind rasters nobody is looking at. Each entry
        // carries its OWN raster size: rows are a fixed height and every
        // board is fitted into that box on its own aspect, so a raster
        // baked at one shared size would stretch every board that is not
        // the shape the deck's first one happened to be.
        let mut wanted: Vec<(String, &op_editor_ui::layout_scene::SceneNode, Point2D)> = Vec::new();
        for (index, _) in &visible {
            if let Some(chip) = slides.chips.get(*index) {
                if let Some(node) = page.find(&chip.id) {
                    wanted.push((chip.id.clone(), node, slides.layout.thumb_rect(*index).size));
                }
            }
        }
        let visible_ids: Vec<usize> = visible.iter().map(|(i, _)| *i).collect();
        for (index, chip) in slides.chips.iter().enumerate() {
            if visible_ids.contains(&index) {
                continue;
            }
            if let Some(node) = page.find(&chip.id) {
                wanted.push((chip.id.clone(), node, slides.layout.thumb_rect(index).size));
            }
        }
        let rendered = self.slide_thumbs.render_pending(frame, &wanted, revision);
        if rendered || self.slide_thumbs.has_pending(frame, &wanted, revision) {
            // Either fresh pixels landed (blit them next frame) or rows
            // are still outstanding (render the next batch then). One
            // frame's grace either way.
            self.slide_thumbs.wake_in(self.now_ms, 16);
        } else {
            self.slide_thumbs.settle();
        }
    }

    /// Route a press. Returns whether the panel claimed it — a press
    /// anywhere on the rail is the panel's, so it never also reaches the
    /// canvas behind it.
    pub(in crate::widget_host) fn slides_panel_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        let point = Point2D::new(x, y);
        if let Some(slides) = self.slides_panel_frame(viewport_w, viewport_h) {
            return match flow::press(&mut self.editor_state, &slides.layout, point) {
                flow::SlidesPress::Missed => false,
                flow::SlidesPress::Claimed(_) => {
                    self.mark_dirty();
                    true
                }
            };
        }
        // The Layers tab owns the rail: only the tab row itself is ours.
        let Some(tabs) = self.slides_tab_row(viewport_w, viewport_h) else {
            return false;
        };
        let Some(target) = tabs.hit(point) else {
            return false;
        };
        self.editor_state.editor_ui.slides_panel.pressed = Some(target);
        self.editor_state.editor_ui.slides_panel.hover = Some(target);
        self.mark_dirty();
        true
    }

    /// Track the cursor over the panel. Returns whether the panel owns
    /// the point (so lower hover tiers stop) and whether anything
    /// visible changed.
    pub(in crate::widget_host) fn slides_panel_hover(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> (bool, bool) {
        let point = Point2D::new(x, y);
        if let Some(slides) = self.slides_panel_frame(viewport_w, viewport_h) {
            let changed = flow::cursor_move(&mut self.editor_state, &slides.layout, point);
            if changed {
                self.mark_dirty();
            }
            let owns = slides.layout.contains_point(point)
                || self.editor_state.editor_ui.slides_panel.drag.is_some();
            return (owns, changed);
        }
        let Some(tabs) = self.slides_tab_row(viewport_w, viewport_h) else {
            // No tab row on this document — drop any hover it left.
            let changed = self.editor_state.editor_ui.slides_panel.clear_pointer();
            if changed {
                self.mark_dirty();
            }
            return (false, changed);
        };
        let changed = flow::tab_cursor_move(&mut self.editor_state, &tabs, point);
        if changed {
            self.mark_dirty();
        }
        (tabs.hit(point).is_some(), changed)
    }

    /// Close a slides-panel gesture. Returns whether one was in flight.
    pub(in crate::widget_host) fn slides_panel_release(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        if self.editor_state.editor_ui.slides_panel.pressed.is_none() {
            return false;
        }
        let outcome = match self.slides_panel_frame(viewport_w, viewport_h) {
            Some(slides) => {
                let outcome = flow::release(&mut self.editor_state, &slides.layout);
                if let flow::SlidesRelease::Activate(index) = outcome {
                    if let Some(chip) = slides.chips.get(index) {
                        self.frame_slide_board(&chip.id.clone(), viewport_w, viewport_h);
                    }
                }
                if let flow::SlidesRelease::Reorder { from, to } = outcome {
                    if let Some(chip) = slides.chips.get(from) {
                        self.apply_slides_reorder(&chip.id.clone(), to);
                    }
                }
                outcome
            }
            None => flow::tab_release(&mut self.editor_state),
        };
        match outcome {
            flow::SlidesRelease::Idle => false,
            flow::SlidesRelease::SelectTab(tab) => {
                self.select_left_panel_tab(tab);
                self.mark_dirty();
                true
            }
            flow::SlidesRelease::Present => {
                // The same path the TopBar's Play button takes, so the
                // action bar and the top bar cannot start two different
                // kinds of presentation.
                let (_x, _y, cw, ch) = self.canvas_region(viewport_w, viewport_h);
                self.enter_preview((cw, ch));
                self.mark_dirty();
                true
            }
            // The export rows queued their file action in the shared
            // flow; the desktop host's own pump drains
            // `pending_file_action` on the next frame, which is where the
            // save picker and `export_deck_pdf` live. Nothing platform-
            // specific is left to do but repaint.
            flow::SlidesRelease::ToggleExportMenu
            | flow::SlidesRelease::ExportAllSlides
            | flow::SlidesRelease::ExportSelectedSlides => {
                self.mark_dirty();
                true
            }
            _ => {
                self.mark_dirty();
                true
            }
        }
    }

    /// How many board thumbnails this host has rendered since it
    /// started. Diagnostic only — the end-to-end capture asserts the
    /// rail actually filled in rather than merely painting placeholders.
    pub fn slide_thumbnail_render_count(&self) -> u64 {
        self.slide_thumbs.renders
    }

    /// Show a left-rail tab.
    fn select_left_panel_tab(&mut self, tab: LeftPanelTab) {
        if flow::select_tab(&mut self.editor_state, tab) {
            // The rail's two tabs are different heights of layer rows;
            // a scroll offset from the other tab would strand the tree.
            self.force_rotate_layer_panel_owner();
        }
    }

    /// Frame one board in the canvas region. Camera only — navigating a
    /// deck must not land on the undo stack.
    fn frame_slide_board(&mut self, board_id: &str, viewport_w: f32, viewport_h: f32) {
        self.refresh_layout_scene();
        op_editor_ui::widgets::host_overlay_geometry::zoom_to_fit_node(
            &mut self.editor_state,
            &self.layout_scene,
            board_id,
            viewport_w,
            viewport_h,
        );
    }

    /// Move a board to a new position in the page's child order, through
    /// the shared deck reorder so the rail can never write the deck order
    /// differently from the presentation that reads it.
    fn apply_slides_reorder(&mut self, board_id: &str, to: usize) {
        if !self.collab_allows_document_mutation(op_editor_core::CollabDocumentMutation::NodeMove) {
            return;
        }
        op_editor_ui::widgets::deck_boards::apply_reorder(&mut self.editor_state, board_id, to);
    }

    /// Wheel / trackpad scroll over the slide list.
    pub(in crate::widget_host) fn slides_panel_scroll(
        &mut self,
        point: Point2D,
        delta_y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<bool> {
        let slides = self.slides_panel_frame(viewport_w, viewport_h)?;
        flow::scroll(&mut self.editor_state, Some(&slides.layout), point, delta_y)
    }
}

#[cfg(test)]
#[path = "slides_panel_tests.rs"]
mod slides_panel_tests;
