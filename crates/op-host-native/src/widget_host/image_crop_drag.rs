//! Figma-style image-fill crop editing for the native canvas host.
//!
//! The gesture itself lives in `op_editor_ui::widgets::image_crop_flow`
//! (shared with the web host); this module is the native platform tail.

use super::WidgetHostNative;
use op_editor_core::NodeId;
use op_editor_ui::widgets::image_crop_flow::{self as crop_flow, ImageCropMove};

pub(in crate::widget_host) use crop_flow::ImageCropDragState;

impl WidgetHostNative {
    pub(in crate::widget_host) fn enter_selected_image_crop_edit(&mut self) -> bool {
        let Some(changed) = crop_flow::enter_selected_image_crop_edit(&mut self.editor_state)
        else {
            return false;
        };
        self.node_drag = None;
        if changed {
            self.mark_dirty();
        }
        true
    }

    pub(in crate::widget_host) fn exit_image_crop_edit(&mut self) -> bool {
        let had_drag = self.finish_image_crop_drag();
        let had_edit = crop_flow::clear_image_crop_editing(&mut self.editor_state);
        if had_edit {
            self.mark_dirty();
        }
        had_drag || had_edit
    }

    pub(in crate::widget_host) fn start_active_image_crop_drag(
        &mut self,
        target: &NodeId,
        hit_path: &[NodeId],
        x: f32,
        y: f32,
    ) -> bool {
        if !crop_flow::can_start_image_crop_drag(&self.editor_state, target) {
            return false;
        }
        if !self.collab_allows_document_mutation(
            op_editor_core::CollabDocumentMutation::Unsupported(
                op_editor_core::CollabUnsupportedFeature::ExternalAssets,
            ),
        ) {
            return true;
        }
        self.refresh_layout_scene();
        let Some(drag) = crop_flow::start_image_crop_drag(
            &self.editor_state,
            &self.layout_scene,
            target,
            hit_path,
            x,
            y,
        ) else {
            return false;
        };
        self.image_crop_drag = Some(drag);
        self.node_drag = None;
        true
    }

    pub(in crate::widget_host) fn apply_image_crop_drag_cursor_move(
        &mut self,
        x: f32,
        y: f32,
    ) -> Option<bool> {
        if self.image_crop_drag.is_some()
            && !self.collab_allows_document_mutation(
                op_editor_core::CollabDocumentMutation::Unsupported(
                    op_editor_core::CollabUnsupportedFeature::ExternalAssets,
                ),
            )
        {
            self.image_crop_drag = None;
            return Some(true);
        }
        let mut drag = self.image_crop_drag.take()?;
        match crop_flow::image_crop_drag_cursor_move(&mut self.editor_state, &mut drag, x, y) {
            // The gesture stays dropped — `take()` above already cleared it.
            ImageCropMove::Detached => {
                self.mark_dirty();
                Some(true)
            }
            ImageCropMove::Idle => {
                self.image_crop_drag = Some(drag);
                Some(false)
            }
            ImageCropMove::Moved { changed } => {
                self.image_crop_drag = Some(drag);
                if changed {
                    self.scene_cache.invalidate();
                    self.mark_dirty();
                }
                Some(changed)
            }
        }
    }

    pub(in crate::widget_host) fn finish_image_crop_drag(&mut self) -> bool {
        let Some(drag) = self.image_crop_drag.take() else {
            return false;
        };
        if crop_flow::finish_image_crop_drag(&mut self.editor_state, drag) {
            self.scene_cache.invalidate();
            self.mark_dirty();
        }
        true
    }
}
