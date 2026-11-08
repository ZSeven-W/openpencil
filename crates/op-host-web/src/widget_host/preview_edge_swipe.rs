//! Edge-swipe-to-pop glue for the web shell's device-frame preview.
//!
//! Moved out of `preview_frame.rs` (pure code motion) to keep that file
//! under the repo's 800-line cap, matching the native host's
//! `preview_edge_swipe.rs` sibling.

impl super::WidgetHost {
    /// Screen-space distance from the framed content's left edge a press
    /// must start within to arm an edge-swipe candidate.
    const EDGE_ZONE_PX: f32 = 24.0;
    /// Cumulative rightward drag distance that fires the pop.
    const POP_THRESHOLD_PX: f32 = 60.0;

    /// Arm (or clear) an edge-swipe candidate for a fresh press at
    /// `screen_x`. Called from preview press right after the
    /// underlying pointer Down is forwarded.
    pub(in crate::widget_host) fn arm_edge_swipe_candidate(&mut self, screen_x: f32) {
        // Clear FIRST, exactly as native does: a fresh press outside the
        // edge zone must disarm any stale candidate, or a leftover armed
        // value from an undisarmed gesture fires a pop on an ordinary drag.
        self.preview_edge_swipe_start_x = None;
        // Edge swipe only works in device-frame preview with a pop-capable stack
        if !self.device_mode_active() {
            return;
        }
        let Some(frame) = self.preview_device_frame.as_ref() else {
            return;
        };
        let edge = frame.content_span_x.0;
        if screen_x < edge || screen_x - edge >= Self::EDGE_ZONE_PX {
            return;
        }
        if !self.preview.as_ref().is_some_and(|p| p.can_pop()) {
            return;
        }
        self.preview_edge_swipe_start_x = Some(screen_x);
    }

    /// Check an armed candidate against the held drag's current `screen_x`.
    /// Returns `true` exactly once per gesture when the threshold is crossed
    /// and fires the pop.
    pub(in crate::widget_host) fn maybe_fire_edge_swipe(&mut self, screen_x: f32) -> bool {
        if self.preview_edge_swipe_start_x.is_none() {
            return false;
        }
        if screen_x - self.preview_edge_swipe_start_x.unwrap() < Self::POP_THRESHOLD_PX {
            return false;
        }
        self.preview_edge_swipe_start_x = None;
        if let Some(preview) = self.preview.as_ref() {
            preview.pop_screen();
            self.on_preview_screen_switched(self.last_viewport_w, self.last_viewport_h);
        }
        true
    }

    /// Clear any armed candidate — called on release so a gesture that
    /// never crossed the threshold doesn't leak into the NEXT press.
    pub(in crate::widget_host) fn disarm_edge_swipe(&mut self) {
        self.preview_edge_swipe_start_x = None;
    }

    /// Cancel the in-flight preview pointer gesture after an edge-swipe
    /// fires. Per-pointer (R4): every held id's arena timers settle and
    /// its capture anchor releases.
    pub(in crate::widget_host) fn cancel_preview_gesture_for_edge_swipe(&mut self) {
        let pids = std::mem::take(&mut self.preview_pressed_pids);
        for pid in pids {
            if let Some(p) = self.preview.as_mut() {
                p.cancel_pointer(pid, self.now_ms);
            }
        }
    }
}
