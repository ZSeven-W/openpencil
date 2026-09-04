//! Canvas ↔ device-frame merge transition for entering and exiting
//! Preview mode ("从画布上该屏 frame 的当前可视矩形动画过渡到设备帧矩形，
//! 反之也是" per the design ask). Sibling of `transition.rs`
//! (screen-to-screen, INSIDE an active preview session) — this one
//! bridges the OTHER seam: canvas (design) mode and device-frame
//! preview mode.
//!
//! ## Lifecycle
//!
//! The struct is pure geometry/timing (`op_editor_ui`-only, no
//! `PreviewSession` access), so it is owned directly by
//! `WidgetHostNative` (`widget_host.rs`'s `preview_mode_transition`
//! field) rather than living inside a `PreviewSession` — on Enter the
//! session is built and installed FIRST (so `self.preview` is already
//! `Some` for the whole animation, same object `next_animation_deadline_ms`
//! already ticks for); on Exit the session must stay alive a little
//! longer than the toggle itself, so `WidgetHostNative::exit_preview`
//! defers dropping `self.preview` until this transition finishes (see
//! that method's doc).
//!
//! ## Scope
//!
//! `op_editor_ui::RenderBackend` has no save-layer / offscreen-alpha
//! primitive, so a literal cross-fade of the FULL canvas chrome (grid,
//! sibling frames, selection handles) would need extending that trait
//! across every backend — out of scope here. What IS implemented: the
//! framed screen's own content moves + scales (position AND size
//! interpolate) from its canvas-space rect into the device-frame
//! silhouette (or the reverse on exit), and the device bezel/border
//! fade in/out via a colour-lerp against the canvas backdrop
//! (mathematically equivalent to true alpha blending over a known
//! solid background — see `widget_host::preview_frame`'s call site).
//! The screen's own content never needs an explicit fade at all: the
//! SAME painter draws it in both the canvas and the preview
//! presentation (`paint_scene_page` is documented pixel-identical to
//! the canvas painter), so only its rect needs to move.
//!
//! `canvas_rect_for_frame` reuses `preview_frame::compute_frame_geometry`
//! rather than composing its own centering/fit transform: feeding a
//! shrunk/offset "canvas" rect that interpolates from the captured
//! source rect to the settled device-frame rect reproduces the EXACT
//! settled geometry at t=1 (the fake canvas degenerates to the real
//! frame rect exactly), with zero duplicated math.

use op_editor_ui::{Point2D, Rect};

pub(crate) const MODE_TRANSITION_DURATION_MS: u64 = 220;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Screen transition direction (Enter / Exit preview mode).
pub enum ModeTransitionKind {
    Enter,
    Exit,
}

/// An in-flight canvas ↔ device-frame merge animation — used by host integration layer.
pub struct ModeTransition {
    kind: ModeTransitionKind,
    started_at_ms: u64,
    /// The screen's on-SCREEN rect (after the canvas's own pan/zoom)
    /// at the moment the transition started — Enter: where it sat on
    /// the canvas just before switching; Exit: where it will land
    /// back on the canvas. `None` when the screen wasn't visible in
    /// the viewport (panned/zoomed away) — degrades to a plain
    /// cross-fade with no geometric movement.
    source_rect: Option<Rect>,
    /// The settled device-frame rect: Enter's destination, Exit's
    /// departure point.
    settled_frame_rect: Rect,
}

impl ModeTransition {
    pub fn start(
        kind: ModeTransitionKind,
        source_rect: Option<Rect>,
        settled_frame_rect: Rect,
        now_ms: u64,
    ) -> Self {
        Self {
            kind,
            started_at_ms: now_ms,
            source_rect,
            settled_frame_rect,
        }
    }

    pub fn kind(&self) -> ModeTransitionKind {
        self.kind
    }

    pub fn is_active(&self, now_ms: u64) -> bool {
        now_ms
            < self
                .started_at_ms
                .saturating_add(MODE_TRANSITION_DURATION_MS)
    }

    /// Next wake time for the host's animation loop — same shape as
    /// `transition::ScreenTransition::next_deadline_ms`. Not currently
    /// consumed by the host (which already ticks ~30fps whenever
    /// `self.preview.is_some()`, and this transition only ever exists
    /// while that's true), kept for parity / a future tighter wake
    /// schedule.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn next_deadline_ms(&self, now_ms: u64) -> Option<u64> {
        if !self.is_active(now_ms) {
            return None;
        }
        Some((now_ms.saturating_add(16)).min(self.started_at_ms + MODE_TRANSITION_DURATION_MS))
    }

    /// Ease-out-cubic progress from 0 (just started) to 1 (settled) —
    /// same formula `transition::ScreenTransition` uses.
    fn eased_t(&self, now_ms: u64) -> f32 {
        let elapsed = now_ms.saturating_sub(self.started_at_ms);
        let t = (elapsed as f32 / MODE_TRANSITION_DURATION_MS as f32).clamp(0.0, 1.0);
        1.0 - (1.0 - t).powi(3)
    }

    /// Progress toward the DEVICE-FRAME (settled preview) end: Enter
    /// runs 0→1 as `eased_t` naturally does; Exit is the same curve
    /// played backward (1→0), so both directions share one lerp
    /// without duplicating the math.
    fn settle_t(&self, now_ms: u64) -> f32 {
        match self.kind {
            ModeTransitionKind::Enter => self.eased_t(now_ms),
            ModeTransitionKind::Exit => 1.0 - self.eased_t(now_ms),
        }
    }

    /// The rect to feed `preview_frame::compute_frame_geometry` in
    /// place of the real canvas region for this frame: interpolated
    /// between the captured source rect (or, absent one, the settled
    /// rect itself — a pure cross-fade with no geometric movement) and
    /// the settled device-frame rect.
    pub fn canvas_rect_for_frame(&self, now_ms: u64) -> Rect {
        let t = self.settle_t(now_ms);
        let source = self.source_rect.unwrap_or(self.settled_frame_rect);
        lerp_rect(source, self.settled_frame_rect, t)
    }

    /// Blend factor for the bezel/border colour-lerp toward the
    /// device-frame's own tone: 0 at the canvas-only end, 1 at the
    /// fully-settled device-frame end (both directions share this via
    /// `settle_t`).
    pub fn chrome_blend(&self, now_ms: u64) -> f32 {
        self.settle_t(now_ms)
    }

    /// Whether a source rect was captured (vs. the pure-cross-fade
    /// degrade path) — test-only introspection.
    #[cfg(test)]
    pub(crate) fn has_source_rect(&self) -> bool {
        self.source_rect.is_some()
    }
}

fn lerp_rect(a: Rect, b: Rect, t: f32) -> Rect {
    Rect {
        origin: Point2D::new(
            a.origin.x + (b.origin.x - a.origin.x) * t,
            a.origin.y + (b.origin.y - a.origin.y) * t,
        ),
        size: Point2D::new(
            a.size.x + (b.size.x - a.size.x) * t,
            a.size.y + (b.size.y - a.size.y) * t,
        ),
    }
}

/// Linear colour-lerp toward `target`, factor `t` in `0.0..=1.0`. Used
/// to fade the device bezel/border in (Enter) or out (Exit) against
/// the canvas backdrop it paints over — equivalent to true alpha
/// blending since the backdrop underneath is a known, already-painted
/// solid colour (`widget_host::preview_frame::paint_device_frame`
/// always fills the canvas region with `theme.canvas_surface` first).
/// Used by host integration layer.
pub fn lerp_color(
    from: op_editor_ui::Color,
    to: op_editor_ui::Color,
    t: f32,
) -> op_editor_ui::Color {
    let t = t.clamp(0.0, 1.0);
    op_editor_ui::Color {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(w, h),
        }
    }

    #[test]
    fn is_active_spans_exactly_the_duration() {
        let t = ModeTransition::start(ModeTransitionKind::Enter, None, rect(0.0, 0.0, 1.0, 1.0), 0);
        assert!(t.is_active(219));
        assert!(!t.is_active(220));
    }

    #[test]
    fn next_deadline_ms_ticks_until_done() {
        let t = ModeTransition::start(
            ModeTransitionKind::Enter,
            None,
            rect(0.0, 0.0, 1.0, 1.0),
            1_000,
        );
        assert_eq!(t.next_deadline_ms(1_000), Some(1_016));
        // Never overshoots the animation's own end.
        assert_eq!(t.next_deadline_ms(1_210), Some(1_220));
        assert_eq!(
            t.next_deadline_ms(1_220),
            None,
            "finished — no more wakeups"
        );
    }

    #[test]
    fn enter_settles_at_the_device_frame_rect() {
        let source = rect(10.0, 20.0, 100.0, 200.0);
        let settled = rect(500.0, 40.0, 390.0, 844.0);
        let t = ModeTransition::start(ModeTransitionKind::Enter, Some(source), settled, 1_000);
        let at_start = t.canvas_rect_for_frame(1_000);
        assert!((at_start.origin.x - source.origin.x).abs() < 0.5);
        assert!((at_start.size.x - source.size.x).abs() < 0.5);
        let at_end = t.canvas_rect_for_frame(1_300);
        assert!((at_end.origin.x - settled.origin.x).abs() < 0.5);
        assert!((at_end.origin.y - settled.origin.y).abs() < 0.5);
        assert!((at_end.size.x - settled.size.x).abs() < 0.5);
        assert!((at_end.size.y - settled.size.y).abs() < 0.5);
    }

    #[test]
    fn exit_reverses_enter_geometrically() {
        let source = rect(10.0, 20.0, 100.0, 200.0);
        let settled = rect(500.0, 40.0, 390.0, 844.0);
        let t = ModeTransition::start(ModeTransitionKind::Exit, Some(source), settled, 1_000);
        let at_start = t.canvas_rect_for_frame(1_000);
        assert!(
            (at_start.origin.x - settled.origin.x).abs() < 0.5,
            "exit starts settled"
        );
        let at_end = t.canvas_rect_for_frame(1_300);
        assert!(
            (at_end.origin.x - source.origin.x).abs() < 0.5,
            "exit ends back at the source"
        );
    }

    #[test]
    fn missing_source_rect_degrades_to_a_fixed_cross_fade() {
        let settled = rect(500.0, 40.0, 390.0, 844.0);
        let t = ModeTransition::start(ModeTransitionKind::Enter, None, settled, 1_000);
        assert!(!t.has_source_rect());
        let mid = t.canvas_rect_for_frame(1_100);
        // No geometric movement at all — every sample equals the
        // settled rect, only `chrome_blend` moves.
        assert!((mid.origin.x - settled.origin.x).abs() < 0.5);
        assert!((mid.size.x - settled.size.x).abs() < 0.5);
    }

    #[test]
    fn chrome_blend_runs_0_to_1_on_enter_and_1_to_0_on_exit() {
        let settled = rect(0.0, 0.0, 1.0, 1.0);
        let enter = ModeTransition::start(ModeTransitionKind::Enter, None, settled, 1_000);
        assert_eq!(enter.chrome_blend(1_000), 0.0);
        assert!((enter.chrome_blend(1_220) - 1.0).abs() < 1e-6);

        let exit = ModeTransition::start(ModeTransitionKind::Exit, None, settled, 1_000);
        assert!((exit.chrome_blend(1_000) - 1.0).abs() < 1e-6);
        assert!(exit.chrome_blend(1_220).abs() < 1e-6);
    }

    #[test]
    fn lerp_color_interpolates_every_channel() {
        use op_editor_ui::Color;
        let from = Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        };
        let to = Color {
            r: 1.0,
            g: 0.5,
            b: 0.25,
            a: 1.0,
        };
        let mid = lerp_color(from, to, 0.5);
        assert!((mid.r - 0.5).abs() < 1e-6);
        assert!((mid.g - 0.25).abs() < 1e-6);
        assert!((mid.b - 0.125).abs() < 1e-6);
        assert!((mid.a - 0.5).abs() < 1e-6);
    }
}
