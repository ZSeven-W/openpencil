//! Tests for `property_panel_code` — split into a sibling file (via
//! `#[path]`) to keep `property_panel_code.rs` under the 800-line cap.

use super::*;
use crate::widgets::PaintCx;
use op_editor_core::codegen::{AssetMeta, ChunkProgress, ChunkStatus, CodeGenProgress, Framework};

/// No-op `RenderBackend` so `paint_code_panel` runs headless.
#[derive(Default)]
struct StubBackend;
impl crate::RenderBackend for StubBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn paint(state: &CodegenState) -> f32 {
    let mut backend = StubBackend;
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    paint_code_panel(&mut cx, &Theme::dark(), state, 0.0, 0.0, 300.0)
}

#[test]
fn paint_code_panel_does_not_panic_each_phase() {
    // Idle.
    let idle = CodegenState::default();
    assert!(paint(&idle) > 0.0);

    // Generating with 2 chunks (one running, one pending).
    let chunk = |id: &str, name: &str, status| ChunkProgress {
        chunk_id: id.into(),
        name: name.into(),
        status,
    };
    let generating = CodegenState {
        phase: CodegenPhase::Generating,
        framework: Framework::Vue,
        progress: CodeGenProgress {
            planning_done: Some(true),
            chunks: vec![
                chunk("a", "Header", ChunkStatus::Running),
                chunk("b", "Footer", ChunkStatus::Pending),
            ],
            assembly_done: None,
        },
        ..CodegenState::default()
    };
    assert!(paint(&generating) > 0.0);

    // Complete with code + one asset, degraded.
    let complete = CodegenState {
        phase: CodegenPhase::Complete,
        code: "fn main() {\n    println!(\"hi\");\n}\n".into(),
        degraded: true,
        assets: vec![AssetMeta {
            relative_path: "logo.svg".into(),
            byte_len: 42,
        }],
        ..CodegenState::default()
    };
    assert!(paint(&complete) > 0.0);

    // Error.
    let error = CodegenState {
        phase: CodegenPhase::Error,
        error: Some("model timeout".into()),
        ..CodegenState::default()
    };
    assert!(paint(&error) > 0.0);
}

fn contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.y
}

fn center(r: Rect) -> Point2D {
    Point2D::new(r.origin.x + r.size.x / 2.0, r.origin.y + r.size.y / 2.0)
}

#[test]
fn code_action_rects_idle_has_generate_and_bundle() {
    let s = CodegenState::default(); // Idle, React
                                     // Wide width so all 8 chips fit the single row (no scroll clipping).
    let rects = code_action_rects(0.0, 0.0, 2000.0, &s);
    assert!(rects
        .iter()
        .any(|(a, _)| matches!(a, CodegenAction::Generate)));
    assert!(rects
        .iter()
        .any(|(a, _)| matches!(a, CodegenAction::ExportBundle)));
    // all 8 framework chips present
    let chips = rects
        .iter()
        .filter(|(a, _)| matches!(a, CodegenAction::SelectFramework(_)))
        .count();
    assert_eq!(chips, 8);
    // No overflow at 2000px → no scroll chevrons.
    assert_eq!(framework_row_overflow(2000.0), 0.0);
    assert!(!rects
        .iter()
        .any(|(a, _)| matches!(a, CodegenAction::ScrollFrameworksLeft)));
    assert!(!rects
        .iter()
        .any(|(a, _)| matches!(a, CodegenAction::ScrollFrameworksRight)));
}

/// At a narrow width the strip overflows, so `code_action_rects` exposes the
/// left + right scroll-chevron zones (the FIX-1 step buttons).
#[test]
fn code_action_rects_narrow_has_scroll_chevrons() {
    let s = CodegenState::default(); // Idle, React.
    assert!(framework_row_overflow(280.0) > 0.0);
    let rects = code_action_rects(0.0, 0.0, 280.0, &s);
    assert!(rects
        .iter()
        .any(|(a, _)| matches!(a, CodegenAction::ScrollFrameworksLeft)));
    assert!(rects
        .iter()
        .any(|(a, _)| matches!(a, CodegenAction::ScrollFrameworksRight)));
}

/// `framework_at` maps a point inside the first chip (scroll 0, wide width →
/// no chevron inset) back to that chip's framework.
#[test]
fn framework_at_hits_first_chip() {
    let chips = framework_chip_rects(0.0, 0.0, 0.0);
    let (first_fw, first_rect) = chips[0];
    assert_eq!(first_fw, Framework::React);
    let p = center(first_rect);
    // Wide width so there's no overflow → no chevron zones eat the edge.
    assert_eq!(
        framework_at(0.0, 0.0, 2000.0, p, 0.0),
        Some(Framework::React)
    );
}

#[test]
fn code_action_rects_complete_has_copy_and_regen() {
    let s = CodegenState {
        phase: CodegenPhase::Complete,
        code: "x".into(),
        ..CodegenState::default()
    };
    let rects = code_action_rects(0.0, 0.0, 280.0, &s);
    assert!(rects.iter().any(|(a, _)| matches!(a, CodegenAction::Copy)));
    assert!(rects
        .iter()
        .any(|(a, _)| matches!(a, CodegenAction::Regenerate)));
}

#[test]
fn code_action_rects_generating_has_cancel_and_chips() {
    let s = CodegenState {
        phase: CodegenPhase::Generating,
        ..CodegenState::default()
    };
    // Wide width so all 8 chips fit the single row (no scroll clipping).
    let rects = code_action_rects(0.0, 0.0, 2000.0, &s);
    assert!(rects
        .iter()
        .any(|(a, _)| matches!(a, CodegenAction::Cancel)));
    assert_eq!(
        rects
            .iter()
            .filter(|(a, _)| matches!(a, CodegenAction::SelectFramework(_)))
            .count(),
        8
    );
}

#[test]
fn code_action_rects_error_has_regenerate_only_button() {
    let s = CodegenState {
        phase: CodegenPhase::Error,
        error: Some("boom".into()),
        ..CodegenState::default()
    };
    let rects = code_action_rects(0.0, 0.0, 280.0, &s);
    // Phase-body buttons only — exclude the framework chips + the scroll
    // chevrons (strip controls present at this narrow, overflowing width).
    let buttons: Vec<_> = rects
        .iter()
        .filter(|(a, _)| {
            !matches!(
                a,
                CodegenAction::SelectFramework(_)
                    | CodegenAction::ScrollFrameworksLeft
                    | CodegenAction::ScrollFrameworksRight
            )
        })
        .collect();
    assert_eq!(buttons.len(), 1);
    assert!(matches!(buttons[0].0, CodegenAction::Regenerate));
}

/// A click at the Generate button's centre hits Generate and no other
/// action's rect (the layout the host's `hit_test_action` walks).
#[test]
fn code_action_rects_generate_center_round_trips() {
    let s = CodegenState::default(); // Idle.
    let rects = code_action_rects(0.0, 0.0, 280.0, &s);
    let (_, gen_rect) = rects
        .iter()
        .find(|(a, _)| matches!(a, CodegenAction::Generate))
        .expect("Generate rect present");
    let p = center(*gen_rect);
    // Exactly one action contains the Generate button's centre.
    let hits: Vec<_> = rects
        .iter()
        .filter(|(_, r)| contains(*r, p))
        .map(|(a, _)| *a)
        .collect();
    assert_eq!(hits, vec![CodegenAction::Generate]);
}

/// The 8 framework chips lay out in a SINGLE row (one shared y), each with
/// a positive-size rect; the strip overflows the 280px panel (so it
/// scrolls), and a positive scroll shifts every chip left by that amount.
#[test]
fn framework_chips_single_row_and_scroll() {
    let chips = framework_chip_rects(0.0, 0.0, 0.0);
    assert_eq!(chips.len(), 8);
    for (_, r) in &chips {
        assert!(r.size.x > 0.0 && r.size.y > 0.0);
    }
    let rows: std::collections::BTreeSet<i32> =
        chips.iter().map(|(_, r)| r.origin.y as i32).collect();
    assert_eq!(rows.len(), 1);
    // Wider than a 280px panel → must scroll to reach the trailing chips.
    assert!(framework_row_overflow(280.0) > 0.0);
    // A positive scroll shifts each chip left by exactly that amount.
    let scrolled = framework_chip_rects(0.0, 0.0, 40.0);
    for ((_, a), (_, b)) in chips.iter().zip(scrolled.iter()) {
        assert!((a.origin.x - b.origin.x - 40.0).abs() < 0.01);
    }
}
