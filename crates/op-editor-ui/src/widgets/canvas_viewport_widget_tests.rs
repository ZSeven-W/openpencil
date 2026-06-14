//! Sibling test file for `canvas_viewport_widget.rs` — asserts the
//! composite static visuals emitted for widget scene nodes on the
//! design surface (track + knob, box + check, bar, chevron, …).

use crate::layout_scene::{NodeKind, SceneNode, SceneWidget, SceneWidgetOption};
use crate::widgets::canvas_viewport_widget::paint_widget_visual;
use crate::widgets::PaintCx;
use crate::{Color, ImageDrawMode, Point2D, Rect, RenderBackend, TextLayout};

/// Recording backend — captures round-rects (rect + fill colour),
/// stroke lines (endpoints + colour), and text runs (content + origin).
#[derive(Default)]
struct WidgetRecorder {
    round_rects: Vec<(Rect, Color)>,
    stroke_round_rects: Vec<(Rect, Color)>,
    lines: Vec<(Point2D, Point2D, Color)>,
    texts: Vec<(String, Point2D)>,
}

impl RenderBackend for WidgetRecorder {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
        if let Some(run) = layout.runs().first() {
            self.texts.push((run.content.clone(), origin));
        }
    }
    fn clip_rect(&mut self, _: Rect) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn scale(&mut self, _: Point2D, _: Point2D) {}
    fn stroke_line(&mut self, from: Point2D, to: Point2D, color: Color, _: f32) {
        self.lines.push((from, to, color));
    }
    fn fill_round_rect(&mut self, rect: Rect, _: f32, color: Color) {
        self.round_rects.push((rect, color));
    }
    fn stroke_round_rect(&mut self, rect: Rect, _: f32, color: Color, _: f32) {
        self.stroke_round_rects.push((rect, color));
    }
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn draw_image(&mut self, _: Rect, _: u64, _: &[u8]) {}
    fn draw_image_with_mode(&mut self, _: Rect, _: u64, _: &[u8], _: ImageDrawMode) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
    fn measure_text_weighted(&mut self, text: &str, font_size: f32, _: u16) -> f32 {
        text.chars().count() as f32 * font_size * 0.5
    }
}

const ACCENT: Color = Color::rgb_u8(0x3b, 0x82, 0xf6);
const TRACK_OFF: Color = Color::rgb_u8(0xd1, 0xd5, 0xdb);
const WHITE: Color = Color::WHITE;

fn paint(node: &SceneNode, rect: Rect) -> WidgetRecorder {
    let mut backend = WidgetRecorder::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    let painted = paint_widget_visual(&mut cx, node, rect, 1.0);
    assert!(painted, "widget visual should report painted");
    backend
}

fn widget_node(kind: NodeKind, w: SceneWidget, rect: Rect) -> SceneNode {
    let mut n = SceneNode::leaf("w", kind);
    n.bounds = rect;
    n.widget = Some(w);
    n
}

#[test]
fn unknown_or_absent_widget_returns_false() {
    let mut backend = WidgetRecorder::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    // No widget descriptor at all.
    let plain = SceneNode::leaf("p", NodeKind::Rect);
    assert!(!paint_widget_visual(
        &mut cx,
        &plain,
        Rect::xywh(0.0, 0.0, 40.0, 20.0),
        1.0
    ));
}

#[test]
fn switch_on_paints_accent_track_and_right_knob() {
    let rect = Rect::xywh(0.0, 0.0, 40.0, 20.0);
    let node = widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "switch".into(),
            checked: Some(true),
            ..Default::default()
        },
        rect,
    );
    let b = paint(&node, rect);
    // Track (accent) + knob (white) = 2 filled round rects.
    assert_eq!(b.round_rects.len(), 2, "track + knob");
    assert_eq!(b.round_rects[0].1, ACCENT, "on-track is accent");
    assert_eq!(b.round_rects[1].1, WHITE, "knob is white");
    // Knob slid to the right half.
    let knob = b.round_rects[1].0;
    assert!(
        knob.origin.x > rect.size.x / 2.0,
        "on-knob sits on the right, got x={}",
        knob.origin.x
    );
}

#[test]
fn switch_off_paints_grey_track_and_left_knob() {
    let rect = Rect::xywh(0.0, 0.0, 40.0, 20.0);
    let node = widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "switch".into(),
            checked: Some(false),
            ..Default::default()
        },
        rect,
    );
    let b = paint(&node, rect);
    assert_eq!(b.round_rects[0].1, TRACK_OFF, "off-track is grey");
    let knob = b.round_rects[1].0;
    assert!(
        knob.origin.x < rect.size.x / 2.0,
        "off-knob sits on the left"
    );
}

#[test]
fn checkbox_checked_paints_accent_box_and_check_polyline() {
    let rect = Rect::xywh(0.0, 0.0, 18.0, 18.0);
    let node = widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "checkbox".into(),
            checked: Some(true),
            ..Default::default()
        },
        rect,
    );
    let b = paint(&node, rect);
    // Accent-filled box.
    assert_eq!(b.round_rects[0].1, ACCENT, "checked box fills accent");
    // Two line segments form the white check (✓).
    assert_eq!(b.lines.len(), 2, "check polyline is 2 segments");
    assert!(
        b.lines.iter().all(|(_, _, c)| *c == WHITE),
        "check is white"
    );
}

#[test]
fn checkbox_unchecked_paints_outlined_box_no_check() {
    let rect = Rect::xywh(0.0, 0.0, 18.0, 18.0);
    let node = widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "checkbox".into(),
            checked: Some(false),
            ..Default::default()
        },
        rect,
    );
    let b = paint(&node, rect);
    assert!(b.lines.is_empty(), "unchecked box has no check mark");
    assert_eq!(
        b.stroke_round_rects[0].1, TRACK_OFF,
        "unchecked box is outlined grey"
    );
}

#[test]
fn slider_paints_track_filled_portion_and_knob() {
    let rect = Rect::xywh(0.0, 0.0, 100.0, 16.0);
    let node = widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "slider".into(),
            value_num: Some(50.0),
            min: Some(0.0),
            max: Some(100.0),
            ..Default::default()
        },
        rect,
    );
    let b = paint(&node, rect);
    // track (grey) + filled (accent) + knob (white) = 3 round rects.
    assert_eq!(b.round_rects.len(), 3, "track + fill + knob");
    assert_eq!(b.round_rects[0].1, TRACK_OFF, "track grey");
    assert_eq!(b.round_rects[1].1, ACCENT, "filled accent");
    // 50% fill spans half the width.
    let fill = b.round_rects[1].0;
    assert!(
        (fill.size.x - 50.0).abs() < 0.5,
        "half-value fill ~= half width, got {}",
        fill.size.x
    );
    assert_eq!(b.round_rects[2].1, WHITE, "knob white");
}

#[test]
fn slider_with_zero_value_has_no_accent_fill() {
    let rect = Rect::xywh(0.0, 0.0, 100.0, 16.0);
    let node = widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "slider".into(),
            value_num: Some(0.0),
            min: Some(0.0),
            max: Some(100.0),
            ..Default::default()
        },
        rect,
    );
    let b = paint(&node, rect);
    // Only track + knob; no accent fill at 0%.
    assert!(
        b.round_rects.iter().all(|(_, c)| *c != ACCENT),
        "no accent fill at value=min"
    );
}

#[test]
fn progress_paints_track_and_filled_portion() {
    let rect = Rect::xywh(0.0, 0.0, 200.0, 8.0);
    let node = widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "progress".into(),
            value_num: Some(25.0),
            max: Some(100.0),
            ..Default::default()
        },
        rect,
    );
    let b = paint(&node, rect);
    assert_eq!(b.round_rects[0].1, TRACK_OFF, "progress track grey");
    let fill = &b.round_rects[1];
    assert_eq!(fill.1, ACCENT, "progress fill accent");
    assert!(
        (fill.0.size.x - 50.0).abs() < 0.5,
        "25/100 of 200px ~= 50px, got {}",
        fill.0.size.x
    );
}

#[test]
fn select_paints_box_value_text_and_chevron() {
    let rect = Rect::xywh(0.0, 0.0, 160.0, 36.0);
    let node = widget_node(
        NodeKind::Text,
        SceneWidget {
            kind: "select".into(),
            value_str: Some("us".into()),
            options: vec![
                SceneWidgetOption {
                    value: "us".into(),
                    label: "United States".into(),
                },
                SceneWidgetOption {
                    value: "ca".into(),
                    label: "Canada".into(),
                },
            ],
            ..Default::default()
        },
        rect,
    );
    let b = paint(&node, rect);
    // Selected option label is painted.
    assert!(
        b.texts.iter().any(|(t, _)| t == "United States"),
        "selected label painted, got {:?}",
        b.texts
    );
    // Chevron = 2 line segments.
    assert_eq!(b.lines.len(), 2, "down chevron is 2 segments");
    // Outlined box.
    assert!(!b.stroke_round_rects.is_empty(), "select box outlined");
}

#[test]
fn select_empty_paints_placeholder() {
    let rect = Rect::xywh(0.0, 0.0, 160.0, 36.0);
    let node = widget_node(
        NodeKind::Text,
        SceneWidget {
            kind: "select".into(),
            placeholder: Some("Choose…".into()),
            ..Default::default()
        },
        rect,
    );
    let b = paint(&node, rect);
    assert!(
        b.texts.iter().any(|(t, _)| t == "Choose…"),
        "placeholder painted"
    );
}

#[test]
fn radio_group_paints_circle_and_dot_for_selected() {
    let rect = Rect::xywh(0.0, 0.0, 120.0, 56.0);
    let node = widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "radio_group".into(),
            value_str: Some("a".into()),
            options: vec![
                SceneWidgetOption {
                    value: "a".into(),
                    label: "Apple".into(),
                },
                SceneWidgetOption {
                    value: "b".into(),
                    label: "Banana".into(),
                },
            ],
            ..Default::default()
        },
        rect,
    );
    let b = paint(&node, rect);
    // Selected circle filled accent + inner white dot; unselected
    // circle filled white. 2 options → 3 fills (2 circles + 1 dot).
    assert_eq!(b.round_rects.len(), 3, "2 circles + 1 inner dot");
    assert!(
        b.round_rects.iter().any(|(_, c)| *c == ACCENT),
        "selected circle is accent"
    );
    // Both labels painted.
    assert!(b.texts.iter().any(|(t, _)| t == "Apple"));
    assert!(b.texts.iter().any(|(t, _)| t == "Banana"));
}

#[test]
fn text_input_paints_value_then_placeholder() {
    let rect = Rect::xywh(0.0, 0.0, 200.0, 36.0);
    let with_value = widget_node(
        NodeKind::Text,
        SceneWidget {
            kind: "text_input".into(),
            value_str: Some("hello".into()),
            placeholder: Some("Type…".into()),
            ..Default::default()
        },
        rect,
    );
    let b = paint(&with_value, rect);
    assert!(
        b.texts.iter().any(|(t, _)| t == "hello"),
        "value wins over placeholder"
    );
    assert!(!b.texts.iter().any(|(t, _)| t == "Type…"));

    let empty = widget_node(
        NodeKind::Text,
        SceneWidget {
            kind: "text_input".into(),
            placeholder: Some("Type…".into()),
            ..Default::default()
        },
        rect,
    );
    let b2 = paint(&empty, rect);
    assert!(
        b2.texts.iter().any(|(t, _)| t == "Type…"),
        "placeholder shown when empty"
    );
}

#[test]
fn number_input_renders_numeric_value() {
    let rect = Rect::xywh(0.0, 0.0, 120.0, 36.0);
    let node = widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "number_input".into(),
            value_num: Some(42.0),
            ..Default::default()
        },
        rect,
    );
    let b = paint(&node, rect);
    assert!(
        b.texts.iter().any(|(t, _)| t == "42"),
        "integer value renders without decimals, got {:?}",
        b.texts
    );
}

#[test]
fn tabs_paints_bar_labels_with_active_underline() {
    let rect = Rect::xywh(0.0, 0.0, 240.0, 120.0);
    let node = widget_node(
        NodeKind::Frame,
        SceneWidget {
            kind: "tabs".into(),
            value_str: Some("one".into()),
            options: vec![
                SceneWidgetOption {
                    value: "one".into(),
                    label: "One".into(),
                },
                SceneWidgetOption {
                    value: "two".into(),
                    label: "Two".into(),
                },
            ],
            ..Default::default()
        },
        rect,
    );
    let b = paint(&node, rect);
    assert!(b.texts.iter().any(|(t, _)| t == "One"));
    assert!(b.texts.iter().any(|(t, _)| t == "Two"));
    // Bottom border + active-tab accent underline.
    assert!(
        b.lines.iter().any(|(_, _, c)| *c == ACCENT),
        "active tab has an accent underline"
    );
}
