//! Task 1 Step 20-21: prove the Jian re-export wrapper compiles & is usable.
//!
//! Two anchor invariants for spec v19 §2 / §5.2:
//! 1. `DrawOp::Rect` constructible **through the OP re-export path**
//!    (`openpencil_shell_core::jian::DrawOp` — used by shell-native's internal translation).
//! 2. `TextLayout::single_run` produces **exactly one** `TextRun` (spec §5.2,
//!    confirming the explicit-field replacement for `..Default::default()` is semantically equivalent).

use openpencil_shell_core::jian::{DrawOp, JianRect, Paint};
use openpencil_shell_core::render_backend::{Color, Point2D, TextLayout};

#[test]
fn drawop_rect_constructible_via_re_export() {
    // Construct a Jian DrawOp::Rect via the OP re-export path — proves shell-core's
    // jian module exposes jian_core::render::{DrawOp, Paint}.
    let rect = JianRect::new(
        jian_core::geometry::Point::new(0.0, 0.0),
        jian_core::geometry::Size::new(100.0, 50.0),
    );
    let paint = Paint::solid(jian_core::scene::Color::rgb(255, 0, 0));
    let op = DrawOp::Rect { rect, paint };

    match op {
        DrawOp::Rect { rect, .. } => {
            assert_eq!(rect.size.width, 100.0);
            assert_eq!(rect.size.height, 50.0);
        }
        _ => panic!("expected DrawOp::Rect"),
    }
}

#[test]
fn text_layout_single_run_creates_one_run() {
    // Explicit-field construction (TextRun has no Default impl) — proves the
    // spec §5.2 single_run path produces exactly one run with content /
    // font_family / font_size set from the caller's arguments.
    let layout = TextLayout::single_run(
        "Hello",
        "system-ui",
        16.0,
        jian_core::scene::Color::rgb(0, 0, 0),
        Point2D::new(10.0, 20.0),
    );

    assert_eq!(layout.runs().len(), 1);

    let run = &layout.runs()[0];
    assert_eq!(run.content, "Hello");
    assert_eq!(run.font_family, "system-ui");
    assert_eq!(run.font_size, 16.0);
    assert_eq!(run.font_weight, 400);
    assert_eq!(run.origin.x, 10.0);
    assert_eq!(run.origin.y, 20.0);
    assert_eq!(run.max_width, 0.0);
    assert_eq!(run.line_height, 0.0);
}

#[test]
fn text_layout_translated_offsets_origin() {
    // translated() adds offset to each run's origin; the original layout is unchanged.
    let layout = TextLayout::single_run(
        "World",
        "system-ui",
        14.0,
        jian_core::scene::Color::rgb(0, 0, 0),
        Point2D::new(5.0, 10.0),
    );
    let shifted = layout.translated(Point2D::new(100.0, 200.0));

    assert_eq!(shifted.runs()[0].origin.x, 105.0);
    assert_eq!(shifted.runs()[0].origin.y, 210.0);

    // Original layout untouched.
    assert_eq!(layout.runs()[0].origin.x, 5.0);
    assert_eq!(layout.runs()[0].origin.y, 10.0);
}

#[test]
fn op_color_constants_distinct() {
    // spec §5.2 names all six constants — RED/GREEN/BLUE/BLACK/WHITE/TRANSPARENT.
    assert_eq!(Color::RED.r, 1.0);
    assert_eq!(Color::GREEN.g, 1.0);
    assert_eq!(Color::BLUE.b, 1.0);
    assert_eq!(Color::BLACK.r, 0.0);
    assert_eq!(Color::WHITE.r, 1.0);
    assert_eq!(Color::TRANSPARENT.a, 0.0);
}
