//! Design-MD-panel press dispatch — extracted from `press.rs` to
//! keep that file under the repo's 800-line cap.
//!
//! The floating Design-MD panel sits alongside the Git panel in
//! paint order; `apply_press` calls
//! [`WidgetHostNative::dispatch_design_md_press`] after the Git panel
//! and before the canvas overlays.

use op_editor_ui::widgets::{DesignMdHit, DesignMdPanel};
use op_editor_ui::Point2D;

use super::{DesignMdDragState, WidgetHostNative};

impl WidgetHostNative {
    /// Dispatch a press inside the floating Design-MD panel.
    ///
    /// Returns `true` when the click was consumed — a close, a
    /// section toggle, a queued import / export request, a drag
    /// start, or a swallowed click inside the panel body.
    pub(in crate::widget_host) fn dispatch_design_md_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let Some(panel_rect) = self.design_md_panel_rect(viewport_width, viewport_height) else {
            return false;
        };
        let point = Point2D::new(x, y);
        let Some((hit, pressed_button)) =
            DesignMdPanel::for_editor(&self.editor_state).and_then(|p| {
                Some((
                    p.hit_test(panel_rect, point)?,
                    p.hover_at(panel_rect, point),
                ))
            })
        else {
            return false;
        };
        if let Some(button) = pressed_button {
            self.editor_state.editor_ui.pressed_button =
                Some(op_editor_core::ButtonPressTarget::DesignMd(button));
        }
        match hit {
            DesignMdHit::Close => {
                self.editor_state.editor_ui.design_md_panel.open = false;
                self.editor_state.editor_ui.design_md_panel.hover = None;
            }
            DesignMdHit::DragHeader => {
                self.design_md_drag = Some(DesignMdDragState {
                    grab_dx: x - panel_rect.origin.x,
                    grab_dy: y - panel_rect.origin.y,
                });
            }
            DesignMdHit::ToggleSection(index) => {
                self.editor_state.editor_ui.design_md_panel.expanded ^= 1u8 << index;
            }
            DesignMdHit::Import => {
                if self.collab_allows_document_mutation_from(
                    op_editor_core::CollabDocumentMutation::Unsupported(
                        op_editor_core::CollabUnsupportedFeature::RootMetadata,
                    ),
                    op_editor_core::CollabEditSource::Import,
                ) {
                    self.editor_state.editor_ui.design_md_panel.request =
                        Some(op_editor_core::DesignMdRequest::Import);
                }
            }
            DesignMdHit::AutoGenerate => {
                if self.collab_allows_document_mutation_from(
                    op_editor_core::CollabDocumentMutation::Unsupported(
                        op_editor_core::CollabUnsupportedFeature::RootMetadata,
                    ),
                    op_editor_core::CollabEditSource::Ai,
                ) {
                    self.editor_state.editor_ui.design_md_panel.request =
                        Some(op_editor_core::DesignMdRequest::AutoGenerate);
                }
            }
            DesignMdHit::Export => {
                self.editor_state.editor_ui.design_md_panel.request =
                    Some(op_editor_core::DesignMdRequest::Export);
            }
            DesignMdHit::Remove => {
                if !self.collab_allows_document_mutation(
                    op_editor_core::CollabDocumentMutation::Unsupported(
                        op_editor_core::CollabUnsupportedFeature::RootMetadata,
                    ),
                ) {
                    self.mark_dirty();
                    return true;
                }
                // Clearing the brief mutates the document — snapshot
                // first so a stray remove is undoable.
                let snap = self.editor_state.snapshot_for_history();
                self.editor_state.doc.design_md = None;
                self.editor_state.editor_ui.design_md_panel.scroll.offset = 0.0;
                self.editor_state.history_push_past(snap);
            }
            DesignMdHit::Inside => {
                // Blank press on panel chrome — blur chrome inputs.
                self.blur_text_inputs_on_blank_press();
            }
        }
        self.mark_dirty();
        true
    }
}
