//! Tests for `property_panel_code` — split into a sibling file (via
//! `#[path]`) to keep `property_panel_code.rs` under the 800-line cap.

use super::*;
use crate::widgets::PaintCx;
use op_editor_core::codegen::{
    AssetMeta, ChunkProgress, ChunkStatus, CodeGenProgress, CodeSelection, CodegenHover, Framework,
};

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

#[derive(Default)]
struct CaptureBackend {
    rect_fills: Vec<(Rect, Color)>,
    round_fills: Vec<(Rect, Color)>,
    texts: Vec<(String, jian_core::scene::Color)>,
    rotations: Vec<(f32, Point2D)>,
}

impl crate::RenderBackend for CaptureBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, r: Rect, c: Color) {
        self.rect_fills.push((r, c));
    }
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, layout: &TextLayout, _: Point2D) {
        if let Some(run) = layout.runs().first() {
            self.texts.push((run.content.clone(), run.color));
        }
    }
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, r: Rect, _: f32, c: Color) {
        self.round_fills.push((r, c));
    }
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn rotate(&mut self, radians: f32, pivot: Point2D) {
        self.rotations.push((radians, pivot));
    }
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

fn paint_capture(state: &CodegenState, theme: &Theme, w: f32) -> CaptureBackend {
    paint_capture_at(state, theme, w, 0)
}

fn paint_capture_at(state: &CodegenState, theme: &Theme, w: f32, now_ms: u64) -> CaptureBackend {
    let mut backend = CaptureBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        paint_code_panel_at(&mut cx, theme, state, 0.0, 0.0, w, now_ms);
    }
    backend
}

fn paint_capture_in_panel(
    state: &CodegenState,
    theme: &Theme,
    panel_rect: Rect,
    now_ms: u64,
) -> CaptureBackend {
    let mut backend = CaptureBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        paint_code_panel_in_panel(&mut cx, theme, state, panel_rect, now_ms);
    }
    backend
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

fn rect_eq(a: Rect, b: Rect) -> bool {
    (a.origin.x - b.origin.x).abs() < 0.01
        && (a.origin.y - b.origin.y).abs() < 0.01
        && (a.size.x - b.size.x).abs() < 0.01
        && (a.size.y - b.size.y).abs() < 0.01
}

fn color_eq(a: Color, b: Color) -> bool {
    (a.r - b.r).abs() < 0.001
        && (a.g - b.g).abs() < 0.001
        && (a.b - b.b).abs() < 0.001
        && (a.a - b.a).abs() < 0.001
}

#[test]
fn code_hover_at_maps_idle_generate_button() {
    let s = CodegenState::default();
    let panel_rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(300.0, 700.0),
    };
    let rects = code_action_rects(panel_rect.origin.x, TAB_HEIGHT, panel_rect.size.x, &s);
    let (_, gen_rect) = rects
        .iter()
        .find(|(a, _)| matches!(a, CodegenAction::Generate))
        .expect("Generate rect present");

    assert_eq!(
        code_hover_at(panel_rect, &s, center(*gen_rect)),
        (None, Some(CodegenHover::Generate))
    );
}

#[test]
fn idle_generate_hover_paints_primary_feedback() {
    let theme = Theme::light();
    let s = CodegenState {
        action_hover: Some(CodegenHover::Generate),
        ..CodegenState::default()
    };
    let (_, gen_rect) = code_action_rects(0.0, 0.0, 300.0, &s)
        .into_iter()
        .find(|(a, _)| matches!(a, CodegenAction::Generate))
        .expect("Generate rect present");

    let backend = paint_capture(&s, &theme, 300.0);
    let fills: Vec<_> = backend
        .round_fills
        .iter()
        .filter(|(r, _)| rect_eq(*r, gen_rect))
        .collect();

    assert!(
        fills.len() >= 2,
        "hovered primary button should paint a visible overlay on top of its base fill"
    );
}

#[test]
fn idle_bundle_hover_paints_neutral_feedback() {
    let theme = Theme::light();
    let s = CodegenState {
        action_hover: Some(CodegenHover::ExportBundle),
        ..CodegenState::default()
    };
    let (_, bundle_rect) = code_action_rects(0.0, 0.0, 300.0, &s)
        .into_iter()
        .find(|(a, _)| matches!(a, CodegenAction::ExportBundle))
        .expect("Export bundle rect present");

    let backend = paint_capture(&s, &theme, 300.0);

    assert!(
        backend
            .round_fills
            .iter()
            .any(|(r, c)| rect_eq(*r, bundle_rect)
                && !color_eq(*c, theme.muted)
                && c.a > theme.button_hover.a + 0.01),
        "hovered borderless Export AI Bundle should paint a visible wash over the transparent button"
    );
}

#[test]
fn generating_phase_paints_compact_progress_card() {
    let theme = Theme::light();
    let state = CodegenState {
        phase: CodegenPhase::Generating,
        progress: CodeGenProgress {
            planning_done: Some(true),
            chunks: vec![ChunkProgress {
                chunk_id: "header".into(),
                name: "Header".into(),
                status: ChunkStatus::Running,
            }],
            assembly_done: None,
        },
        ..CodegenState::default()
    };
    let body_y = chips_body_top(SECTION_HEADER_HEIGHT);
    let backend = paint_capture(&state, &theme, 300.0);

    assert!(
        backend.round_fills.iter().any(|(r, color)| {
            rect_eq(
                *r,
                Rect {
                    origin: Point2D::new(PAD_X, body_y),
                    size: Point2D::new(300.0 - PAD_X * 2.0, r.size.y),
                },
            ) && r.size.y >= 120.0
                && color_eq(*color, theme.muted)
        }),
        "Generating phase should paint a compact progress card around the status rows"
    );
}

#[test]
fn generating_running_step_rotates_loader_icon_from_host_clock() {
    let theme = Theme::light();
    let state = CodegenState {
        phase: CodegenPhase::Generating,
        progress: CodeGenProgress {
            planning_done: Some(false),
            chunks: vec![],
            assembly_done: None,
        },
        ..CodegenState::default()
    };
    let backend = paint_capture_at(&state, &theme, 300.0, 250);

    assert!(
        backend
            .rotations
            .iter()
            .any(|(radians, _)| radians.abs() > 0.01),
        "Running progress step should rotate the loader icon using now_ms"
    );
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
    let chips = framework_chip_rects(0.0, 0.0, 2000.0, 0.0);
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
fn framework_chips_use_ts_compact_padding() {
    let chips = framework_chip_rects(0.0, 0.0, 2000.0, 0.0);
    let (_, flutter) = chips
        .iter()
        .find(|(fw, _)| *fw == Framework::Flutter)
        .expect("Flutter chip should be present");
    let expected_w = "flutter".chars().count() as f32 * 11.0 * 0.55 + 16.0;

    assert!(
        (flutter.size.x - expected_w).abs() < 0.01,
        "framework chips should use TS px-2 horizontal padding and 11px text"
    );
    assert!(
        (flutter.size.y - 22.0).abs() < 0.01,
        "framework chips should match the compact TS tab height"
    );
}

#[test]
fn react_native_chip_uses_short_ts_label() {
    let chips = framework_chip_rects(0.0, 0.0, 2000.0, 0.0);
    let (_, react_native) = chips
        .iter()
        .find(|(fw, _)| *fw == Framework::ReactNative)
        .expect("React Native chip should be present");
    let expected_w = "RN".chars().count() as f32 * 11.0 * 0.55 + 16.0;

    assert!(
        (react_native.size.x - expected_w).abs() < 0.01,
        "React Native framework chip should use the TS short label"
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
fn complete_actions_use_compact_toolbar_on_wide_panels() {
    let state = CodegenState {
        phase: CodegenPhase::Complete,
        code: "x".into(),
        ..CodegenState::default()
    };
    let panel_rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(860.0, 760.0),
    };
    let rects = code_action_rects_in_panel(panel_rect, &state);
    let action_rect = |action| {
        rects
            .iter()
            .find(|(candidate, _)| *candidate == action)
            .map(|(_, rect)| *rect)
            .expect("complete action rect should be present")
    };
    let actions = [
        action_rect(CodegenAction::Copy),
        action_rect(CodegenAction::Download),
        action_rect(CodegenAction::ExportBundle),
        action_rect(CodegenAction::Regenerate),
    ];
    let total_w = actions[3].origin.x + actions[3].size.x - actions[0].origin.x;

    assert!(
        total_w <= 360.0,
        "complete action row should not stretch across wide panels, got {total_w}"
    );
    assert!(
        actions.iter().all(|rect| rect.size.x <= 96.0),
        "complete action chips should stay content-sized, got {actions:?}"
    );
    assert!(
        actions.iter().all(|rect| (rect.size.y - 30.0).abs() < 0.01),
        "complete action chips should use the compact toolbar height, got {actions:?}"
    );
}

#[test]
fn complete_code_preview_hit_test_maps_points_to_code_offsets() {
    let state = CodegenState {
        phase: CodegenPhase::Complete,
        code: "import React\nconst n = 1".into(),
        ..CodegenState::default()
    };
    let panel_rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(360.0, 700.0),
    };
    let char_w = 11.0 * 0.55;

    assert_eq!(
        code_text_offset_at(panel_rect, &state, Point2D::new(66.0 + char_w * 6.0, 112.0)),
        Some(6)
    );
    assert_eq!(
        code_text_offset_at(panel_rect, &state, Point2D::new(66.0 + char_w * 5.0, 128.0)),
        Some("import React\n".len() + 5)
    );
    assert_eq!(
        code_text_offset_at(panel_rect, &state, Point2D::new(24.0, 40.0)),
        None
    );
}

#[test]
fn complete_code_preview_expands_to_panel_height() {
    let theme = Theme::light();
    let state = CodegenState {
        phase: CodegenPhase::Complete,
        code: (0..80)
            .map(|i| format!("const line{i} = {i};"))
            .collect::<Vec<_>>()
            .join("\n"),
        ..CodegenState::default()
    };
    let panel_rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(360.0, 760.0),
    };

    let backend = paint_capture_in_panel(&state, &theme, panel_rect, 0);
    let code_rect = backend
        .round_fills
        .iter()
        .map(|(rect, _)| *rect)
        .find(|rect| rect.size.x > 300.0 && rect.size.y > 220.0)
        .expect("code preview surface should paint");
    let rects = code_action_rects_in_panel(panel_rect, &state);
    let (_, copy_rect) = rects
        .iter()
        .find(|(action, _)| matches!(action, CodegenAction::Copy))
        .expect("copy action should be present");

    assert!(
        code_rect.size.y > 560.0,
        "code preview should grow with the panel height, got {code_rect:?}"
    );
    assert!(
        copy_rect.origin.y > code_rect.origin.y + code_rect.size.y,
        "action row should stay below the expanded preview"
    );
    assert!(
        copy_rect.origin.y + copy_rect.size.y <= panel_rect.origin.y + panel_rect.size.y - 8.0,
        "action row should remain inside the panel"
    );
}

#[test]
fn complete_code_preview_hit_test_respects_vertical_scroll() {
    let code = (0..80)
        .map(|i| format!("line-{i:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    let state = CodegenState {
        phase: CodegenPhase::Complete,
        code_scroll: 16.0 * 10.0,
        code,
        ..CodegenState::default()
    };
    let panel_rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(360.0, 700.0),
    };
    let expected = state
        .code
        .lines()
        .take(10)
        .map(|line| line.len() + 1)
        .sum::<usize>()
        + 4;

    assert_eq!(
        code_text_offset_at(
            panel_rect,
            &state,
            Point2D::new(66.0 + 11.0 * 0.55 * 4.0, 112.0)
        ),
        Some(expected)
    );
    assert!(
        code_preview_max_scroll(panel_rect, &state).unwrap() > 0.0,
        "long generated code should expose a vertical scroll range"
    );
}

#[test]
fn complete_code_preview_paints_selection_highlight() {
    let theme = Theme::light();
    let state = CodegenState {
        phase: CodegenPhase::Complete,
        code: "import React\nconst n = 1".into(),
        code_selection: Some(CodeSelection {
            anchor: 0,
            focus: "import".len(),
        }),
        ..CodegenState::default()
    };

    let backend = paint_capture(&state, &theme, 360.0);
    let selection = crate::widgets::text_selection::selection_color(&theme);

    assert!(
        backend
            .rect_fills
            .iter()
            .any(|(_, color)| color_eq(*color, selection)),
        "selected code text should paint a visible selection background"
    );
}

#[test]
fn complete_code_preview_uses_syntax_highlight_colors() {
    let theme = Theme::light();
    let state = CodegenState {
        phase: CodegenPhase::Complete,
        code: "import React from \"react\";\nconst count = 3;\nreturn <View />;".into(),
        ..CodegenState::default()
    };

    let backend = paint_capture(&state, &theme, 360.0);
    let code_texts: Vec<_> = backend
        .texts
        .iter()
        .filter(|(text, _)| {
            matches!(
                text.as_str(),
                "import" | "\"react\"" | "const" | "3" | "return"
            )
        })
        .collect();
    let unique_colors: std::collections::BTreeSet<u32> =
        code_texts.iter().map(|(_, color)| color.0).collect();

    assert!(
        unique_colors.len() >= 3,
        "code preview should draw keywords and strings with distinct syntax colors, got {code_texts:?}"
    );
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
    let chips = framework_chip_rects(0.0, 0.0, 2000.0, 0.0);
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
    let scrolled = framework_chip_rects(0.0, 0.0, 2000.0, 40.0);
    for ((_, a), (_, b)) in chips.iter().zip(scrolled.iter()) {
        assert!((a.origin.x - b.origin.x - 40.0).abs() < 0.01);
    }
}
