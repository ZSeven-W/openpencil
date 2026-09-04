//! Device-frame preview teardown glue for the web shell.
//!
//! Moved out of `preview_frame.rs` to keep the device-frame geometry file
//! under the repo's 800-line cap while preserving the same lifecycle split as
//! the native host.

use super::WidgetHost;

impl WidgetHost {
    /// Exit Preview mode. Track M-1: when a device frame is on screen,
    /// this does NOT drop the runtime immediately — it starts the reverse
    /// merge animation (device-frame rect → the screen's canvas-space rect)
    /// and leaves `self.preview` alive so there is still a live scene to
    /// paint while it plays. `settle_mode_transition` (called once per
    /// frame from the paint entry point) does the ACTUAL teardown once
    /// the animation finishes. Calling `do_exit_preview` again while the
    /// exit animation is already playing is a no-op (idempotent).
    pub(in crate::widget_host) fn do_exit_preview(
        &mut self,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        #[cfg(feature = "canvaskit")]
        {
            let Some(settled) = self.preview_device_frame.as_ref().map(|f| f.frame) else {
                // No device frame on screen (Canvas-mode preview, or
                // preview was never entered) — nothing to merge back into,
                // so exit immediately exactly as before.
                self.finish_exit_teardown();
                self.mark_dirty();
                return;
            };
            if self.preview.is_none()
                || matches!(
                    self.preview_mode_transition.as_ref().map(|t| t.kind()),
                    Some(op_preview_core::ModeTransitionKind::Exit)
                )
            {
                return; // already out, or already exiting
            }
            let dest_rect = self.preview.as_ref().and_then(|session| {
                session.framed_root().map(|(_, rect)| {
                    self.framed_root_to_screen_rect(rect, viewport_width, viewport_height)
                })
            });
            self.preview_mode_transition = Some(op_preview_core::ModeTransition::start(
                op_preview_core::ModeTransitionKind::Exit,
                dest_rect,
                settled,
                self.now_ms,
            ));
            self.mark_dirty();
        }
    }

    /// The actual Preview-mode teardown deferred by `do_exit_preview` —
    /// shared by `settle_mode_transition` (the animation finished on
    /// its own) and re-enter (the user re-opened before it did).
    pub(in crate::widget_host) fn finish_exit_teardown(&mut self) {
        self.preview = None;
        self.clear_device_preview_state();
        self.preview_pressed_pids.clear();
        self.preview_last_doc_by_pid.clear();
        self.preview_surface_capture = None;
        self.slideshow_cursor = None;
        self.slideshow_press_screen = None;
        self.editor_state.editor_ui.exit_preview();
        #[cfg(feature = "canvaskit")]
        {
            self.preview_mode_transition = None;
        }
    }

    /// Once a Track M-1 merge animation finishes, finalize whichever
    /// direction it was playing: Enter just clears the (by-then
    /// no-op) transition marker; Exit performs the teardown that was
    /// deferred at `do_exit_preview` time. Called once per frame from the
    /// paint entry point, right at the start. A no-op while a transition
    /// is still active, or when none is playing.
    pub(in crate::widget_host) fn settle_mode_transition(&mut self) {
        #[cfg(feature = "canvaskit")]
        {
            let Some(transition) = self.preview_mode_transition.as_ref() else {
                return;
            };
            if transition.is_active(self.now_ms) {
                return;
            }
            if transition.kind() == op_preview_core::ModeTransitionKind::Exit {
                // ACTUAL teardown deferred by do_exit_preview
                self.finish_exit_teardown();
            } else {
                self.preview_mode_transition = None;
            }
        }
    }
}
