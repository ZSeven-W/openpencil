//! Task 1 Step 20-21: prove the Jian re-export wrapper compiles & is usable.
//!
//! Two anchor invariants for spec v19 §2 / §5.2：
//! 1. `DrawOp::Rect` constructible **through OP re-export path**
//!    (`openpencil_shell_core::jian::DrawOp` —— shell-native 内部翻译用)。
//! 2. `TextLayout::single_run` 产生**恰好一个** `TextRun`（spec §5.2，
//!    确认 `..Default::default()` 替代成显式字段后语义等价）。

use openpencil_shell_core::jian::{DrawOp, JianRect, Paint};
use openpencil_shell_core::render_backend::{Color, Point2D, TextLayout};

#[test]
fn drawop_rect_constructible_via_re_export() {
    // 通过 OP re-export 路径构造 Jian DrawOp::Rect —— 证明 shell-core 的
    // jian 模块把 jian_core::render::{DrawOp, Paint} 暴露出来了。
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
    // 显式字段构造（TextRun 没 Default impl）—— 证明 spec §5.2 的
    // single_run 路径产生恰好一个 run，content/font_family/font_size
    // 都按调用方传入设置。
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
    // translated() 把 offset 加到每 run origin 上；原 layout 不变。
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

    // 原 layout 不动
    assert_eq!(layout.runs()[0].origin.x, 5.0);
    assert_eq!(layout.runs()[0].origin.y, 10.0);
}

#[test]
fn op_color_constants_distinct() {
    // spec §5.2 命名常量 6 全 —— RED/GREEN/BLUE/BLACK/WHITE/TRANSPARENT。
    assert_eq!(Color::RED.r, 1.0);
    assert_eq!(Color::GREEN.g, 1.0);
    assert_eq!(Color::BLUE.b, 1.0);
    assert_eq!(Color::BLACK.r, 0.0);
    assert_eq!(Color::WHITE.r, 1.0);
    assert_eq!(Color::TRANSPARENT.a, 0.0);
}
