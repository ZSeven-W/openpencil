//! Frame clock for animated SkSL shader fills (issue #255).
//!
//! A shader fill that declares `uniform float time;` (or `half time`) gets
//! the elapsed seconds bound to it on every canvas frame, the same way the
//! scene builder binds `size`; an authored `time` value is overridden.
//!
//! The clock is paint-scoped rather than threaded through `PaintCx`: a host
//! opens a [`FrameScope`] around its canvas paint, and the shader painter
//! reads the time from it. Anything that paints without a scope — PNG / PDF
//! / SVG export, screenshots, thumbnails — renders deterministically at
//! `time = 0`. The scope also records whether an animated shader was drawn,
//! so the shared animation deadline keeps requesting frames while one is on
//! screen and the native pan cache can stand aside instead of freezing it.

use std::cell::Cell;

/// Frame interval requested while an animated shader is on screen.
pub const ANIMATED_SHADER_FRAME_MS: u64 = 16;

thread_local! {
    /// Seconds bound to `time` for the frame in progress; `None` outside a
    /// host frame (exports).
    static FRAME_TIME_S: Cell<Option<f32>> = const { Cell::new(None) };
    /// Host clock at the first frame, so `time` starts near zero and keeps
    /// f32 precision however large the host's millisecond clock is.
    static EPOCH_MS: Cell<Option<u64>> = const { Cell::new(None) };
    /// An animated shader was painted in the frame in progress.
    static PAINTED: Cell<bool> = const { Cell::new(false) };
    /// The last completed frame painted an animated shader.
    static ON_SCREEN: Cell<bool> = const { Cell::new(false) };
}

/// One host canvas frame. Dropping it publishes whether an animated shader
/// was painted and restores the previous (normally absent) clock.
pub struct FrameScope {
    previous: Option<f32>,
}

/// Open a host frame at host clock `now_ms`.
pub fn begin_frame(now_ms: u64) -> FrameScope {
    let epoch = EPOCH_MS.with(|epoch| *epoch.get().get_or_insert(now_ms));
    EPOCH_MS.with(|cell| cell.set(Some(epoch)));
    let seconds = now_ms.saturating_sub(epoch) as f64 / 1000.0;
    PAINTED.with(|painted| painted.set(false));
    FrameScope {
        previous: FRAME_TIME_S.with(|time| time.replace(Some(seconds as f32))),
    }
}

impl Drop for FrameScope {
    fn drop(&mut self) {
        ON_SCREEN.with(|on| on.set(PAINTED.with(Cell::get)));
        FRAME_TIME_S.with(|time| time.set(self.previous));
    }
}

/// The value to bind to `time` for `sksl`, or `None` when the program has
/// no `time` uniform. Marks the frame as animated when a host clock runs.
pub fn bind_time(sksl: &str) -> Option<f32> {
    if !declares_time_uniform(sksl) {
        return None;
    }
    let time = FRAME_TIME_S.with(Cell::get);
    if time.is_some() {
        PAINTED.with(|painted| painted.set(true));
    }
    Some(time.unwrap_or(0.0))
}

/// Whether the last completed host frame painted an animated shader.
pub fn animated_shader_on_screen() -> bool {
    ON_SCREEN.with(Cell::get)
}

/// `uniform float time;` / `uniform half time;` anywhere in the source.
pub fn declares_time_uniform(sksl: &str) -> bool {
    let tokens: Vec<&str> = sksl
        .split(|c: char| c.is_whitespace() || c == ';')
        .filter(|token| !token.is_empty())
        .collect();
    tokens
        .windows(3)
        .any(|w| w[0] == "uniform" && matches!(w[1], "float" | "half") && w[2] == "time")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANIMATED: &str = "uniform float2 size;\nuniform half time;\nhalf4 main(float2 p) { return half4(sin(time)); }";

    #[test]
    fn only_a_declared_time_uniform_is_bound() {
        assert!(declares_time_uniform(ANIMATED));
        assert!(declares_time_uniform("uniform float time;"));
        assert!(!declares_time_uniform(
            "uniform float timeScale; float time = 1.0;"
        ));
        assert_eq!(bind_time("half4 main(float2 p) { return half4(1); }"), None);
    }

    #[test]
    fn exports_render_at_time_zero_and_frames_advance_the_clock() {
        // No host frame: deterministic export.
        assert_eq!(bind_time(ANIMATED), Some(0.0));
        {
            let _frame = begin_frame(10_000);
            assert_eq!(bind_time(ANIMATED), Some(0.0));
        }
        assert!(animated_shader_on_screen());
        {
            let _frame = begin_frame(12_500);
            assert_eq!(bind_time(ANIMATED), Some(2.5));
        }
        // A frame that paints no animated shader clears the flag, and the
        // clock is gone again outside the frame.
        {
            let _frame = begin_frame(13_000);
        }
        assert!(!animated_shader_on_screen());
        assert_eq!(bind_time(ANIMATED), Some(0.0));
    }
}
