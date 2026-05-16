//! Spec v19 §3.3 / plan v7 Task 2 Step 16a:
//! `teardown` + lifecycle hooks must be idempotent.
//!
//! Three repeated calls of every hook (and `teardown`) must complete
//! without panic. Acceptance #6 covers this for the post-loop case;
//! this test pins the structural invariant on the empty-context
//! branch (the hardest path to get wrong is "already-`None` fields
//! shouldn't blow up").

mod common;

use common::setup_headless_context;

#[test]
fn teardown_three_times_no_panic() {
    let mut ctx = setup_headless_context();
    ctx.teardown().expect("first teardown");
    ctx.teardown().expect("second teardown");
    ctx.teardown().expect("third teardown");
}

#[test]
fn lifecycle_hooks_are_idempotent() {
    // Mirrors spec §11.4 invariant + plan v7 Task 2 Step 16a:
    // on_pause/on_resume/on_low_memory/teardown must each survive
    // back-to-back invocation without panic.
    let mut ctx = setup_headless_context();
    ctx.on_pause().expect("on_pause #1");
    ctx.on_pause().expect("on_pause #2");
    ctx.on_resume(None).expect("on_resume #1");
    ctx.on_resume(None).expect("on_resume #2");
    ctx.on_low_memory();
    ctx.on_low_memory();
    ctx.teardown().expect("teardown #1");
    ctx.teardown().expect("teardown #2");
}

#[test]
fn drop_after_teardown_is_safe() {
    // Drop's `teardown` fallback must coexist with explicit teardown
    // without double-free / panic (matches spec §3.3 Drop impl).
    let mut ctx = setup_headless_context();
    ctx.teardown().expect("explicit teardown");
    drop(ctx); // implicit teardown — must be no-op
}
