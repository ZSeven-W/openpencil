//! Spec v19 §9.3 acceptance #6 / plan v7 Task 2 Step 16b:
//! 100 iterations of `{ create + frame + present + teardown × 3 }` must
//! not grow RSS by more than 5 % over baseline.
//!
//! On non-Linux runners we use the inert constructor (no live GL
//! surface) — it's still a sanity check that the public API doesn't
//! leak `Box<dyn GlContextProvider>` / `DirectContext` shadows. The
//! "real" RSS guarantee for live contexts comes from `gpu_smoke.rs`
//! tearing down in its own loop.

mod common;

use common::setup_headless_context;

#[test]
fn teardown_loop_no_memory_growth() {
    // sysinfo is the dev-dep recommended by spec §9.3.
    let mut sys = sysinfo::System::new();
    let pid = sysinfo::Pid::from_u32(std::process::id());

    sys.refresh_process(pid);
    let initial_rss = sys.process(pid).map(|p| p.memory()).unwrap_or(0);
    // Some sysinfo backends report 0 on first refresh until memory
    // sampling stabilises. Accept that and bail out gracefully —
    // the loop body still exercises the lifecycle path.
    assert!(initial_rss > 0, "sysinfo did not report initial RSS");

    for _ in 0..100 {
        let mut ctx = setup_headless_context();
        ctx.begin_frame();
        ctx.present();
        ctx.teardown().unwrap();
        ctx.teardown().unwrap();
        ctx.teardown().unwrap();
    }

    sys.refresh_process(pid);
    let final_rss = sys.process(pid).map(|p| p.memory()).unwrap_or(0);

    // 5 % budget per acceptance #6.
    let budget = initial_rss + initial_rss / 20;
    assert!(
        final_rss <= budget,
        "RSS grew > 5 % across 100 teardown loops: {} → {} (budget {})",
        initial_rss,
        final_rss,
        budget,
    );
}
