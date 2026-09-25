//! The tablet half of the native works reader: the strip's real board
//! rasters and where the chat opens over a tablet reader.
//!
//! The strip reuses the shared `SlideThumbCache` pump the desktop deck
//! strip and the slides rail use (retain → tick → render within the
//! frame budget), so a board edited by a page-edit turn refreshes its
//! tile the same way it refreshes everywhere else.

use super::super::WidgetHostNative;
use crate::backend::NativeFrameBackend;
use op_editor_ui::widgets::works_reader::{tablet_chat_rect, thumb_plate, ReaderForm};
use op_editor_ui::widgets::ReaderLayout;
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
    /// Where the chat opens over a TABLET reader — the landscape side
    /// panel or the portrait floating sheet — or `None` when the reader
    /// is not up on a tablet (the phone keeps its modal bottom sheet).
    pub(in crate::widget_host) fn reader_tablet_chat_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
        visible_bottom: f32,
    ) -> Option<Rect> {
        if !self.works_reader_visible() {
            return None;
        }
        let form = ReaderForm::for_ui(&self.editor_state.editor_ui, viewport_w, viewport_h);
        form.is_tablet()
            .then(|| tablet_chat_rect(form, viewport_w, viewport_h, visible_bottom))
    }

    /// Blit the strip's board rasters over the placeholder plates and
    /// top the thumbnail cache up for the tiles on show.
    pub(in crate::widget_host) fn paint_reader_thumbs(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        layout: &ReaderLayout,
        boards: &[String],
    ) {
        if layout.thumbs.is_empty() {
            return;
        }
        for &(index, tile) in &layout.thumbs {
            let Some(image) = boards
                .get(index)
                .and_then(|board| self.slide_thumbs.image(board))
            else {
                continue;
            };
            let plate = thumb_plate(tile);
            let image = image.clone();
            frame.draw_offscreen_layer_to(&image, plate, plate);
        }
        let revision = self.editor_state.document_revision();
        self.slide_thumbs.retain_boards(boards);
        if !self.slide_thumbs.tick(revision, self.now_ms) {
            return;
        }
        let Some(page) = self.layout_scene.active_page() else {
            return;
        };
        let wanted: Vec<(String, &op_editor_ui::layout_scene::SceneNode, Point2D)> = layout
            .thumbs
            .iter()
            .filter_map(|&(index, tile)| {
                let board = boards.get(index)?;
                let node = page.find(board)?;
                let plate = thumb_plate(tile);
                Some((board.clone(), node, plate.size))
            })
            .collect();
        let rendered = self.slide_thumbs.render_pending(frame, &wanted, revision);
        if rendered || self.slide_thumbs.has_pending(frame, &wanted, revision) {
            self.slide_thumbs.wake_in(self.now_ms, 16);
        } else {
            self.slide_thumbs.settle();
        }
    }
}
