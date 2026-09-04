//! The engine's global monotonic clock feed.
//!
//! Carved out of `lifecycle.rs` as pure code motion to keep that spine
//! under the repo's 800-line cap.

use crate::lifecycle::Session;

impl Session {
    /// Advance the engine's GLOBAL monotonic clocks to `t_ms` — the
    /// session clock, and (in editor mode) the widget host + the live
    /// preview runtime. Never moves backward: a frame pump at 2000
    /// followed by a pointer event carrying 950 leaves every global
    /// clock at 2000. Every clock-feeding ABI path (frame pump, render-
    /// free background tick, generic `op_pointer`, and the dedicated
    /// editor `_at` entries) goes through here; the event's own
    /// factual timestamp is carried separately into the pointer
    /// dispatch, so a backward event still measures its own delta.
    pub(crate) fn advance_global_clock(&mut self, t_ms: u64) {
        self.now_ms = self.now_ms.max(t_ms);
        #[cfg(feature = "editor")]
        if let Some(host) = self.editor.as_mut() {
            host.set_now_ms(t_ms);
        }
    }
}
