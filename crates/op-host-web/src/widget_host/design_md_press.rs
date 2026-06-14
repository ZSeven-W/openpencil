//! Design-MD-panel press dispatch — mirror of the native host's
//! `widget_host/design_md_press.rs`.
//!
//! The floating Design-MD panel paints top-most; `apply_press` calls
//! [`WidgetHost::dispatch_design_md_press`] first so a click on its
//! rect is the panel's before any lower layer can claim it.

use op_editor_ui::widgets::{DesignMdHit, DesignMdPanel};
use op_editor_ui::Point2D;

use super::{PanelDragState, WidgetHost};

impl WidgetHost {
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
                self.editor_state.editor_ui.design_md_panel_open = false;
                self.editor_state.editor_ui.design_md_hover = None;
            }
            DesignMdHit::DragHeader => {
                self.design_md_drag = Some(PanelDragState {
                    grab_dx: x - panel_rect.origin.x,
                    grab_dy: y - panel_rect.origin.y,
                });
            }
            DesignMdHit::ToggleSection(index) => {
                self.editor_state.editor_ui.design_md_expanded ^= 1u8 << index;
            }
            DesignMdHit::Import => {
                // File dialogs are a host-level service web doesn't have
                // yet — raise the same request flag the native host does.
                self.editor_state.editor_ui.design_md_request =
                    Some(op_editor_core::DesignMdRequest::Import);
            }
            DesignMdHit::Export => {
                self.editor_state.editor_ui.design_md_request =
                    Some(op_editor_core::DesignMdRequest::Export);
            }
            DesignMdHit::Remove => {
                // Clearing the brief mutates the document — snapshot
                // first so a stray remove is undoable.
                let snap = self.editor_state.snapshot_for_history();
                self.editor_state.doc.design_md = None;
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
