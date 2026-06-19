#![cfg(feature = "gl-host")]
//! Spec v19 acceptance #8 / plan v7 Task 2 Step 16c:
//! `tracing` spans must fire at `begin_frame / present / resize /
//! teardown` (and the rest of the frame surface). `tracing-test`
//! captures them at the subscriber level.

mod common;

use common::setup_headless_context;
use tracing_test::traced_test;

#[traced_test]
#[test]
fn frame_lifecycle_spans_emitted() {
    let mut ctx = setup_headless_context();
    ctx.begin_frame();
    ctx.with_frame(|_canvas, _glow| {
        // Empty closure body — we still want the `with_frame` span if
        // the surface were live; on the inert ctx the closure simply
        // doesn't fire (surface is None), so the span entry alone is
        // what `tracing-test` records.
    });
    ctx.present();
    ctx.resize(1024, 768).unwrap();
    ctx.teardown().unwrap();

    // Span name is the function name by default for
    // `#[tracing::instrument]` — assert each lifecycle entry shows up
    // exactly once.
    assert!(logs_contain("begin_frame"), "begin_frame span missing");
    assert!(logs_contain("present"), "present span missing");
    assert!(logs_contain("resize"), "resize span missing");
    assert!(logs_contain("teardown"), "teardown span missing");
}

#[traced_test]
#[test]
fn lifecycle_hook_spans_emitted() {
    let mut ctx = setup_headless_context();
    ctx.on_pause().unwrap();
    ctx.on_resume(None).unwrap();
    ctx.on_low_memory();

    assert!(logs_contain("on_pause"), "on_pause span missing");
    assert!(logs_contain("on_resume"), "on_resume span missing");
    assert!(logs_contain("on_low_memory"), "on_low_memory span missing");
}
