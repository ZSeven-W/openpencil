//! Slides-panel arm — the web host's platform tail.
//!
//! The same decisions as native's
//! `op-host-native/src/widget_host/slides_panel.rs`, because both call
//! the same `op_editor_ui::widgets::slides_panel_flow`, with ONE
//! difference: the rows carry no rendered thumbnail.
//!
//! **Why the browser shows placeholders.** Rendering a board needs a
//! second offscreen surface driven by the native scene painter; the web
//! bundle has no such renderer, and the CanvasKit canvas the page paints
//! into is not one it can allocate siblings from. The capability is
//! therefore split exactly like `deck_html_export_supported`: the tab,
//! the cards, the number chips, the highlight, the click-to-navigate and
//! the reorder all work here, and the thumbnail plate paints as its
//! placeholder — a faint slide glyph, so a row reads as a slide without
//! a preview rather than as a broken plate. Routing the render
//! to the `--serve-web` daemon (which already rasters boards for
//! `export_nodes`) is the follow-up that closes the gap; it is a
//! transport question, not a design one.
//!
//! The capability itself is `EditorUiState::slide_thumbnails_supported`,
//! which this host never sets — the same shape as
//! `deck_html_export_supported`.

use super::WidgetHost;
use op_editor_ui::widgets::host_canvas_geometry as canvas_geometry;
use op_editor_ui::widgets::slides_panel_flow as flow;
use op_editor_ui::widgets::{BoardChip, SlidesPanelLayout, SlidesPanelTabs};
use op_editor_ui::{Point2D, Rect};

/// Everything an event or a paint needs about the panel.
pub(in crate::widget_host) struct SlidesFrame {
    pub(in crate::widget_host) chips: Vec<BoardChip>,
    pub(in crate::widget_host) active: Option<usize>,
    pub(in crate::widget_host) layout: SlidesPanelLayout,
}

impl WidgetHost {
    fn slides_panel_rect(&self, viewport_h: f32) -> Rect {
        canvas_geometry::layer_panel_rect(&self.editor_state, viewport_h)
    }

    /// The tab row, when this document shows one and the rail is open.
    pub(in crate::widget_host) fn slides_tab_row(
        &self,
        viewport_h: f32,
    ) -> Option<SlidesPanelTabs> {
        if !self.editor_state.editor_ui.sidebar_open {
            return None;
        }
        flow::tab_row(&self.editor_state, self.slides_panel_rect(viewport_h))
    }

    /// The rail rect the Layers tree gets — the whole rail, less the tab
    /// row when one shows.
    pub(in crate::widget_host) fn layers_content_rect(&self, viewport_h: f32) -> Rect {
        let panel = self.slides_panel_rect(viewport_h);
        if !self.editor_state.editor_ui.sidebar_open {
            return panel;
        }
        flow::layers_content_rect(&self.editor_state, panel)
    }

    /// Resolve the panel, or `None` when the slides tab is not on show.
    pub(in crate::widget_host) fn slides_panel_frame(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
    ) -> Option<SlidesFrame> {
        if !self.editor_state.editor_ui.sidebar_open {
            return None;
        }
        let panel = self.slides_panel_rect(viewport_h);
        let chips = flow::slides(&self.editor_state);
        self.refresh_layout_scene();
        let layout = flow::layout(&self.editor_state, &chips, &self.layout_scene, panel)?;
        let canvas = canvas_geometry::canvas_rect(&self.editor_state, viewport_w, viewport_h);
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

    pub(in crate::widget_host) fn paint_slides_panel(
        &self,
        backend: &mut dyn op_editor_ui::RenderBackend,
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
        let mut cx = PaintCx { backend };
        widget.paint(&mut cx, &slides.layout, &self.theme);
        // No blit comes between the two here — the browser has no board
        // renderer — but the pair stays adjacent so the call shape is
        // the one native uses, with its blit in the gap.
        widget.paint_overlay(&mut cx, &slides.layout, &self.theme);
    }

    /// Paint just the tab row, for the frames where the layer tree owns
    /// the rest of the rail.
    pub(in crate::widget_host) fn paint_slides_tab_row(
        &self,
        backend: &mut dyn op_editor_ui::RenderBackend,
        tabs: &SlidesPanelTabs,
    ) {
        use op_editor_ui::widgets::PaintCx;
        let (layers_label, slides_label) = flow::tab_labels(&self.editor_state);
        let mut cx = PaintCx { backend };
        tabs.paint(
            &mut cx,
            &self.theme,
            self.editor_state.editor_ui.slides_panel.hover,
            layers_label,
            slides_label,
        );
    }

    /// Route a press. Returns whether the panel claimed it.
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
        let Some(tabs) = self.slides_tab_row(viewport_h) else {
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

    /// Track the cursor: `(owns, changed)`, like native's twin.
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
        let Some(tabs) = self.slides_tab_row(viewport_h) else {
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

    /// Cursor-move tier for the panel. `Some(dirty)` when it claimed the
    /// move.
    ///
    /// Above the layer-row hover below it because when the slides tab is
    /// showing there are no layer rows under the cursor, and its tab row
    /// sits over the tree in the other tab. A live row drag keeps
    /// ownership wherever the pointer went, so a reorder does not cancel
    /// the moment the cursor leaves the rail.
    ///
    /// `blocked_by_overlay` covers the chat model picker as well as the
    /// floating panels. Native resolves the picker in an EARLIER tier
    /// than the rail; web's ladder runs the rail first, so without this
    /// the rail would claim a move the picker should have seen — and a
    /// picker left open with no layoutable bounds heals on the very
    /// dispatch the rail was swallowing.
    pub(in crate::widget_host) fn slides_panel_cursor_tier(
        &mut self,
        point: Point2D,
        blocked_by_overlay: bool,
    ) -> Option<bool> {
        if blocked_by_overlay {
            return None;
        }
        let (owns, changed) =
            self.slides_panel_hover(point.x, point.y, self.last_viewport_w, self.last_viewport_h);
        owns.then_some(changed)
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
                        // The shared deck reorder, so the rail can never
                        // write the deck order differently from the
                        // presentation that reads it. No collaboration
                        // gate here, unlike native's twin: the web host
                        // carries no collaboration session at all.
                        op_editor_ui::widgets::deck_boards::apply_reorder(
                            &mut self.editor_state,
                            &chip.id.clone(),
                            to,
                        );
                    }
                }
                outcome
            }
            None => flow::tab_release(&mut self.editor_state),
        };
        match outcome {
            flow::SlidesRelease::Idle => false,
            flow::SlidesRelease::SelectTab(tab) => {
                if flow::select_tab(&mut self.editor_state, tab) {
                    self.force_rotate_layer_panel_owner();
                }
                self.mark_dirty();
                true
            }
            flow::SlidesRelease::Present => {
                #[cfg(feature = "canvaskit")]
                {
                    let op_ck = self.op_ck.clone();
                    let _ = self.enter_preview_from_browser(viewport_w, viewport_h, op_ck.as_ref());
                }
                #[cfg(not(feature = "canvaskit"))]
                {
                    self.editor_state.editor_ui.preview.mode = false;
                    self.editor_state.editor_ui.preview.warnings =
                        vec!["preview: not available in this build".to_string()];
                }
                self.mark_dirty();
                true
            }
            // Same as native: the export rows queued their file action in
            // the shared flow, and this host's own `pending_file_action`
            // drain streams the document to the daemon's PDF route. The
            // widget arm has only the repaint left.
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

    /// Frame one board in the canvas region. Camera only.
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
