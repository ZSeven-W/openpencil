//! The top of the web host's cursor-move ladder: in-flight pointer-capture
//! drags and the modal surfaces that own the cursor outright.
//!
//! Pure code motion out of [`cursor_input`](super::cursor_input) at the
//! repo's 800-line cap — same tier shape the native host uses
//! (`widget_host/cursor_move_*.rs`): a tier returns `Some(consumed)` to end
//! the event and `None` to fall through to the next one, and **the call order
//! in the spine is the behaviour**. These tiers ran first before the split and
//! must keep running first: a modal that does not claim the cursor lets the
//! hover washes underneath it light up through the scrim.
//!
//! Note the two shapes of "consumed" here. A drag or a diagnostics card
//! answers only when it actually handled the point (`if … { return … }`,
//! falling through otherwise); an open modal answers for every point inside
//! the window, including the ones it did nothing with — that is what makes it
//! modal.

use op_editor_ui::widgets::cursor_hover_flow as hover_flow;
use op_editor_ui::Point2D;

use super::WidgetHost;

impl WidgetHost {
    /// Drags holding pointer capture, then the modal surfaces, in priority
    /// order. `None` means none of them owned this move.
    pub(in crate::widget_host) fn cursor_move_modal_tiers(
        &mut self,
        x: f32,
        y: f32,
    ) -> Option<bool> {
        if self.apply_path_anchor_drag_move(x, y) {
            return Some(true);
        }
        // In-flight VariablesPanel edge resize — owns the cursor.
        if self.variables_resize.is_some()
            && self.apply_variables_panel_resize(x, y, self.last_viewport_w, self.last_viewport_h)
        {
            return Some(true);
        }
        // A motion timeline drag holds the pointer until release.
        let (vw, vh) = (self.last_viewport_w, self.last_viewport_h);
        let point = Point2D::new(x, y);
        if op_editor_ui::widgets::motion_timeline_flow::drag(
            &mut self.editor_state,
            point,
            vw,
            vh,
            self.now_ms,
        ) {
            self.mark_dirty();
            return Some(true);
        }
        // Missing-fonts modal — owns the cursor while open. Hover the
        // per-row choose-file buttons + the dismiss action.
        if self.editor_state.editor_ui.missing_fonts_modal_open {
            let changed = hover_flow::missing_fonts_modal_hover(
                &mut self.editor_state,
                self.last_viewport_w,
                self.last_viewport_h,
                Point2D::new(x, y),
            );
            if changed {
                self.mark_dirty();
            }
            return Some(changed);
        }
        // Post-import HTML diagnostics notice — tints its own buttons and
        // owns the cursor only while it is under the card.
        if self.editor_state.editor_ui.html_import_diagnostics_open
            && self.update_html_import_diagnostics_hover(
                x,
                y,
                self.last_viewport_w,
                self.last_viewport_h,
            )
        {
            return Some(true);
        }
        // Sign-in modal — owns the cursor while open (native parity).
        if self.editor_state.editor_ui.account_ui_available
            && self.editor_state.editor_ui.login_modal_open
        {
            let changed = hover_flow::login_modal_hover(
                &mut self.editor_state,
                self.last_viewport_w,
                self.last_viewport_h,
                Point2D::new(x, y),
            );
            if changed {
                self.mark_dirty();
            }
            return Some(changed);
        }
        // Signed-in account dropdown — owns the cursor while open.
        if self.editor_state.editor_ui.account_ui_available
            && self.editor_state.editor_ui.account_menu_open
        {
            let changed = hover_flow::account_menu_hover(
                &mut self.editor_state,
                self.last_viewport_w,
                Point2D::new(x, y),
            );
            if changed {
                self.mark_dirty();
            }
            return Some(changed);
        }
        if self.editor_state.editor_ui.agent_settings_open {
            return Some(self.update_agent_settings_hover(x, y));
        }
        None
    }
}
