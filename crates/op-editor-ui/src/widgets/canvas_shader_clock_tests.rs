//! Tests for the time-driven shader clock. Each `#[test]` runs on its own
//! thread, so the thread-local clock starts fresh in every test.

use super::*;

#[test]
fn declares_time_uniform_accepts_float_and_half_with_any_spacing() {
    assert!(declares_time_uniform(
        "uniform float time; half4 main(float2 p){ return half4(1); }"
    ));
    assert!(declares_time_uniform("uniform half time;"));
    assert!(declares_time_uniform("uniform   float\n  time ;"));
}

#[test]
fn declares_time_uniform_rejects_other_names_and_non_uniforms() {
    assert!(!declares_time_uniform("uniform float timeScale;"));
    assert!(!declares_time_uniform("uniform float2 time;"));
    assert!(!declares_time_uniform("float time = 1.0;"));
    assert!(!declares_time_uniform(
        "uniform half4 tint; half4 main(float2 p){ return tint; }"
    ));
}

#[test]
fn time_uniform_value_reads_the_frame_clock_in_seconds() {
    begin_frame(2_500);
    assert_eq!(time_uniform_value("uniform float time;"), Some(2.5));
}

#[test]
fn the_clock_wraps_every_hour() {
    begin_frame(60 * 60 * 1000 + 1_250);
    assert_eq!(time_uniform_value("uniform float time;"), Some(1.25));
}

#[test]
fn an_unpainted_thread_binds_time_zero() {
    assert_eq!(time_uniform_value("uniform float time;"), Some(0.0));
}

#[test]
fn only_a_declaring_shader_marks_the_frame_animated() {
    begin_frame(0);
    assert_eq!(time_uniform_value("uniform half4 tint;"), None);
    assert!(!animated_shader_painted());
    assert_eq!(next_deadline_ms(1_000), None);

    time_uniform_value("uniform float time;");
    assert!(animated_shader_painted());
    assert_eq!(next_deadline_ms(1_000), Some(1_000 + FRAME_INTERVAL_MS));
}

#[test]
fn begin_frame_clears_the_animated_flag() {
    time_uniform_value("uniform float time;");
    assert!(animated_shader_painted());
    begin_frame(16);
    assert!(!animated_shader_painted());
}

#[test]
fn with_fixed_clock_pins_time_and_restores_the_canvas_frame() {
    begin_frame(7_000);
    let pinned = with_fixed_clock(0.0, || time_uniform_value("uniform float time;"));
    assert_eq!(pinned, Some(0.0));
    assert!(
        !animated_shader_painted(),
        "a pinned export must not mark the canvas frame as animated"
    );
    assert_eq!(time_uniform_value("uniform float time;"), Some(7.0));
}
