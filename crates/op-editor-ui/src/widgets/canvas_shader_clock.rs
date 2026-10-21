//! Frame clock for time-driven shader fills.
//!
//! A shader fill whose SkSL declares `uniform float time;` (or `half time`)
//! receives the canvas clock in seconds, bound next to its authored uniforms
//! the same way the loader binds `size`. The clock is per thread: the canvas
//! viewport sets it once per painted frame, so an export or screenshot painted
//! on another thread never calls [`begin_frame`] and renders deterministically
//! at `time = 0`.
//!
//! Painting such a shader marks the frame as animated. Both hosts repaint only
//! on demand, so the shared deadline chain folds in [`next_deadline_ms`] and the
//! browser host (which has no deadline scheduler) checks
//! [`animated_shader_painted`] directly; without that the shader would freeze
//! on the first frame it was painted in.

use std::cell::Cell;

/// Uniform name the canvas binds to the elapsed seconds.
pub const TIME_UNIFORM: &str = "time";

/// Repaint cadence while a time-driven shader is on screen (~60 fps).
pub const FRAME_INTERVAL_MS: u64 = 16;

/// The clock wraps every hour so the `f32` keeps sub-millisecond precision.
const WRAP_MS: u64 = 60 * 60 * 1000;

thread_local! {
    static FRAME_SECONDS: Cell<f32> = const { Cell::new(0.0) };
    static ANIMATED_PAINTED: Cell<bool> = const { Cell::new(false) };
}

/// Start a canvas frame: set the shader clock from the host clock and clear
/// the animated flag the previous frame left behind.
pub fn begin_frame(now_ms: u64) {
    FRAME_SECONDS.with(|seconds| seconds.set((now_ms % WRAP_MS) as f32 / 1000.0));
    ANIMATED_PAINTED.with(|animated| animated.set(false));
}

/// Whether `sksl` declares the time uniform as a scalar
/// (`uniform float time;` / `uniform half time;`, any whitespace).
pub fn declares_time_uniform(sksl: &str) -> bool {
    let mut previous: [&str; 2] = ["", ""];
    for token in sksl
        .split(|c: char| c.is_whitespace() || c == ';')
        .filter(|token| !token.is_empty())
    {
        if previous[0] == "uniform"
            && matches!(previous[1], "float" | "half")
            && token == TIME_UNIFORM
        {
            return true;
        }
        previous = [previous[1], token];
    }
    false
}

/// The value to bind to [`TIME_UNIFORM`] for `sksl`, or `None` when the
/// shader does not declare it. A declaring shader marks the current frame as
/// animated so the host schedules the next one.
pub fn time_uniform_value(sksl: &str) -> Option<f32> {
    if !declares_time_uniform(sksl) {
        return None;
    }
    ANIMATED_PAINTED.with(|animated| animated.set(true));
    Some(FRAME_SECONDS.with(Cell::get))
}

/// Whether a time-driven shader was painted since the last [`begin_frame`].
pub fn animated_shader_painted() -> bool {
    ANIMATED_PAINTED.with(Cell::get)
}

/// The next repaint a time-driven shader on screen needs, if any.
pub fn next_deadline_ms(now_ms: u64) -> Option<u64> {
    animated_shader_painted().then(|| now_ms.saturating_add(FRAME_INTERVAL_MS))
}

#[cfg(test)]
#[path = "canvas_shader_clock_tests.rs"]
mod tests;
