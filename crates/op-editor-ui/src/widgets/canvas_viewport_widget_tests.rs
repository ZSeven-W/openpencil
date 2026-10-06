//! Sibling test file for `canvas_viewport_widget.rs` — asserts the
//! composite static visuals emitted for widget scene nodes on the
//! design surface (track + knob, box + check, bar, chevron, …).

use crate::layout_scene::{
    NodeKind, SceneNode, SceneStroke, SceneStrokeAlign, SceneWidget, SceneWidgetOption,
};
use crate::widgets::canvas_viewport_widget::{
    option_label, paint_widget_visual, text_field_display_text, widget_text_inset_left,
};
use crate::widgets::PaintCx;
use crate::{Color, ImageDrawMode, Point2D, Rect, RenderBackend, TextLayout};
use std::borrow::Cow;

/// Recording backend — captures round-rects (rect + fill colour),
/// stroke lines (endpoints + colour), and text runs (content + origin).
#[derive(Default)]
struct WidgetRecorder {
    round_rects: Vec<(Rect, Color)>,
    round_radii: Vec<f32>,
    stroke_round_rects: Vec<(Rect, Color)>,
    stroke_round_radii: Vec<f32>,
    stroke_round_widths: Vec<f32>,
    lines: Vec<(Point2D, Point2D, Color)>,
    texts: Vec<(String, Point2D)>,
    text_colors: Vec<(String, jian_core::scene::Color)>,
    clips: Vec<Rect>,
}

impl RenderBackend for WidgetRecorder {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
        if let Some(run) = layout.runs().first() {
            self.texts.push((run.content.clone(), origin));
            self.text_colors.push((run.content.clone(), run.color));
        }
    }
    fn clip_rect(&mut self, rect: Rect) {
        self.clips.push(rect);
    }
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn scale(&mut self, _: Point2D, _: Point2D) {}
    fn stroke_line(&mut self, from: Point2D, to: Point2D, color: Color, _: f32) {
        self.lines.push((from, to, color));
    }
    fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.round_rects.push((rect, color));
        self.round_radii.push(radius);
    }
    fn stroke_round_rect(&mut self, rect: Rect, radius: f32, color: Color, width: f32) {
        self.stroke_round_rects.push((rect, color));
        self.stroke_round_radii.push(radius);
        self.stroke_round_widths.push(width);
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

const WHITE: Color = Color::WHITE;
const DARK_PURPLE: Color = Color::rgb_u8(0x18, 0x0b, 0x2a);
const PURPLE_BORDER: Color = Color::rgb_u8(0x72, 0x4a, 0xa0);

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

fn authored_widget_node(kind: NodeKind, mut w: SceneWidget, rect: Rect, radius: f32) -> SceneNode {
    w.corner_radius_authored = true;
    let mut node = widget_node(kind, w, rect);
    node.fill = Some(DARK_PURPLE);
    node.stroke = Some(SceneStroke {
        color: PURPLE_BORDER,
        width: 2.0,
        sides: None,
        align: SceneStrokeAlign::Center,
    });
    node.corner_radius = radius;
    node
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
fn switch_on_paints_authored_track_and_right_knob() {
    let rect = Rect::xywh(0.0, 0.0, 40.0, 20.0);
    let node = authored_widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "switch".into(),
            checked: Some(true),
            ..Default::default()
        },
        rect,
        7.0,
    );
    let b = paint(&node, rect);
    // Track + contrast-derived knob = 2 filled round rects.
    assert_eq!(b.round_rects.len(), 2, "track + knob");
    assert_eq!(b.round_rects[0].1, DARK_PURPLE, "authored on-track");
    assert_eq!(b.round_rects[1].1, WHITE, "dark track gets white knob");
    assert_eq!(b.round_radii[0], 7.0, "authored switch radius");
    // Knob slid to the right half.
    let knob = b.round_rects[1].0;
    assert!(
        knob.origin.x > rect.size.x / 2.0,
        "on-knob sits on the right, got x={}",
        knob.origin.x
    );
}

#[test]
fn switch_off_paints_authored_inactive_track_and_left_knob() {
    let rect = Rect::xywh(0.0, 0.0, 40.0, 20.0);
    let node = authored_widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "switch".into(),
            checked: Some(false),
            ..Default::default()
        },
        rect,
        7.0,
    );
    let b = paint(&node, rect);
    assert_eq!(
        b.round_rects[0].1, PURPLE_BORDER,
        "authored stroke is the inactive track"
    );
    let knob = b.round_rects[1].0;
    assert!(
        knob.origin.x < rect.size.x / 2.0,
        "off-knob sits on the left"
    );
}

#[test]
fn switch_half_progress_places_knob_at_midpoint() {
    let rect = Rect::xywh(0.0, 0.0, 40.0, 20.0);
    let node = authored_widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "switch".into(),
            checked: Some(false),
            toggle_progress: Some(0.5),
            ..Default::default()
        },
        rect,
        7.0,
    );
    let b = paint(&node, rect);
    let knob = b.round_rects[1].0;
    let pad = 2.0;
    let d = (rect.size.y - pad * 2.0).max(2.0);
    let off_x = rect.origin.x + pad;
    let on_x = rect.origin.x + rect.size.x - d - pad;
    let mid_x = off_x + (on_x - off_x) * 0.5;
    assert!(
        (knob.origin.x - mid_x).abs() < 0.01,
        "toggle_progress=0.5 must place the knob at the midpoint, got x={} expected {mid_x}",
        knob.origin.x
    );
}

#[test]
fn zero_radius_legacy_widgets_keep_intrinsic_rounding() {
    let switch_rect = Rect::xywh(0.0, 0.0, 40.0, 20.0);
    let switch = widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "switch".into(),
            ..Default::default()
        },
        switch_rect,
    );
    assert_eq!(paint(&switch, switch_rect).round_radii[0], 10.0);

    let slider_rect = Rect::xywh(0.0, 0.0, 100.0, 16.0);
    let slider = widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "slider".into(),
            ..Default::default()
        },
        slider_rect,
    );
    assert_eq!(paint(&slider, slider_rect).round_radii[0], 2.0);

    let progress_rect = Rect::xywh(0.0, 0.0, 100.0, 8.0);
    let progress = widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "progress".into(),
            ..Default::default()
        },
        progress_rect,
    );
    assert_eq!(paint(&progress, progress_rect).round_radii[0], 4.0);

    let checkbox_rect = Rect::xywh(0.0, 0.0, 18.0, 18.0);
    let checkbox = widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "checkbox".into(),
            ..Default::default()
        },
        checkbox_rect,
    );
    assert_eq!(paint(&checkbox, checkbox_rect).stroke_round_radii[0], 2.0);
}

#[test]
fn corner_radius_distinguishes_absent_explicit_zero_and_positive() {
    let rect = Rect::xywh(0.0, 0.0, 40.0, 20.0);
    let absent = widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "switch".into(),
            ..Default::default()
        },
        rect,
    );
    let explicit_zero = authored_widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "switch".into(),
            ..Default::default()
        },
        rect,
        0.0,
    );
    let positive = authored_widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "switch".into(),
            ..Default::default()
        },
        rect,
        7.0,
    );

    assert_eq!(paint(&absent, rect).round_radii[0], 10.0);
    assert_eq!(paint(&explicit_zero, rect).round_radii[0], 0.0);
    assert_eq!(paint(&positive, rect).round_radii[0], 7.0);
}

#[test]
fn checkbox_checked_paints_accent_box_and_check_polyline() {
    let rect = Rect::xywh(0.0, 0.0, 18.0, 18.0);
    let node = authored_widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "checkbox".into(),
            checked: Some(true),
            ..Default::default()
        },
        rect,
        5.0,
    );
    let b = paint(&node, rect);
    // Accent-filled box.
    assert_eq!(
        b.round_rects[0].1, DARK_PURPLE,
        "checked box uses authored fill"
    );
    // Two line segments form the white check (✓).
    assert_eq!(b.lines.len(), 2, "check polyline is 2 segments");
    assert_eq!(
        b.stroke_round_rects[0].1, PURPLE_BORDER,
        "checked box retains authored outline"
    );
    assert!(
        b.lines.iter().all(|(_, _, c)| *c == WHITE),
        "check is white"
    );
}

#[test]
fn checkbox_180x24_label_uses_in_bounds_control_geometry_and_label_role() {
    let rect = Rect::xywh(10.0, 20.0, 180.0, 24.0);
    let mut node = authored_widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "checkbox".into(),
            checked: Some(true),
            label: Some("Agree".into()),
            ..Default::default()
        },
        rect,
        4.0,
    );
    // Dark fill would produce white field foreground, while a light authored
    // stroke produces black external-label foreground. This catches accidental
    // reuse of the fill/surface contrast role for adjacent labels.
    node.stroke.as_mut().unwrap().color = Color::rgb_u8(0xf4, 0xf4, 0xf5);

    let b = paint(&node, rect);
    let (label_color, origin) = b
        .text_colors
        .iter()
        .find(|(text, _)| text == "Agree")
        .map(|(_, color)| *color)
        .zip(
            b.texts
                .iter()
                .find(|(text, _)| text == "Agree")
                .map(|(_, origin)| *origin),
        )
        .expect("checkbox label paint");
    assert_eq!(label_color, jian_core::scene::Color::rgb(0x00, 0x00, 0x00));
    assert_eq!(
        b.round_rects[0].0,
        Rect::xywh(10.0, 20.0, 24.0, 24.0),
        "labelled checkbox paints a square box inside the whole control"
    );
    assert_eq!(
        b.stroke_round_rects[0].0,
        Rect::xywh(10.0, 20.0, 24.0, 24.0)
    );
    assert_eq!(origin.x, 42.0, "label starts box-right + 8px");
    assert_eq!(
        b.clips,
        vec![Rect::xywh(42.0, 20.0, 148.0, 24.0)],
        "label paint is clipped to the remaining authored control width"
    );
    assert!(
        b.lines
            .iter()
            .flat_map(|(from, to, _)| [from, to])
            .all(|point| rect.contains(*point)),
        "check geometry stays within the full hit bounds"
    );
}

#[test]
fn checkbox_unchecked_paints_outlined_box_no_check() {
    let rect = Rect::xywh(0.0, 0.0, 18.0, 18.0);
    let node = authored_widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "checkbox".into(),
            checked: Some(false),
            ..Default::default()
        },
        rect,
        5.0,
    );
    let b = paint(&node, rect);
    assert!(b.lines.is_empty(), "unchecked box has no check mark");
    assert_eq!(
        b.stroke_round_rects[0].1, PURPLE_BORDER,
        "unchecked box uses authored stroke"
    );
}

#[test]
fn slider_paints_track_filled_portion_and_knob() {
    let rect = Rect::xywh(0.0, 0.0, 100.0, 16.0);
    let node = authored_widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "slider".into(),
            value_num: Some(50.0),
            min: Some(0.0),
            max: Some(100.0),
            ..Default::default()
        },
        rect,
        10.0,
    );
    let b = paint(&node, rect);
    // authored inactive track + active fill + contrast knob.
    assert_eq!(b.round_rects.len(), 3, "track + fill + knob");
    assert_eq!(b.round_rects[0].1, PURPLE_BORDER, "authored track");
    assert_eq!(b.round_rects[1].1, DARK_PURPLE, "authored fill");
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
    let node = authored_widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "slider".into(),
            value_num: Some(0.0),
            min: Some(0.0),
            max: Some(100.0),
            ..Default::default()
        },
        rect,
        10.0,
    );
    let b = paint(&node, rect);
    // Only track + knob; no accent fill at 0%.
    assert!(
        b.round_rects.iter().all(|(_, c)| *c != DARK_PURPLE),
        "no authored active fill at value=min"
    );
}

#[test]
fn progress_paints_track_and_filled_portion() {
    let rect = Rect::xywh(0.0, 0.0, 200.0, 8.0);
    let node = authored_widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "progress".into(),
            value_num: Some(25.0),
            max: Some(100.0),
            ..Default::default()
        },
        rect,
        4.0,
    );
    let b = paint(&node, rect);
    assert_eq!(b.round_rects[0].1, PURPLE_BORDER, "authored progress track");
    let fill = &b.round_rects[1];
    assert_eq!(fill.1, DARK_PURPLE, "authored progress fill");
    assert!(
        (fill.0.size.x - 50.0).abs() < 0.5,
        "25/100 of 200px ~= 50px, got {}",
        fill.0.size.x
    );
}

#[test]
fn select_paints_box_value_text_and_chevron() {
    let rect = Rect::xywh(0.0, 0.0, 160.0, 36.0);
    let node = authored_widget_node(
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
        11.0,
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
    assert_eq!(b.round_rects[0].1, DARK_PURPLE, "authored select surface");
    assert_eq!(
        b.stroke_round_rects[0].1, PURPLE_BORDER,
        "authored select border"
    );
    assert_eq!(b.round_radii[0], 11.0, "authored select radius");
    assert_eq!(
        b.stroke_round_radii[0], 11.0,
        "fill and border share authored radius"
    );
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
    assert!(
        b.round_rects.is_empty(),
        "unstyled select must not fabricate a white surface"
    );
    assert!(
        b.stroke_round_rects.is_empty(),
        "unstyled select must not fabricate a grey border"
    );
}

#[test]
fn select_placeholder_is_muted_but_value_uses_authored_contrast_foreground() {
    let rect = Rect::xywh(0.0, 0.0, 160.0, 36.0);
    let placeholder = authored_widget_node(
        NodeKind::Text,
        SceneWidget {
            kind: "select".into(),
            placeholder: Some("Choose…".into()),
            ..Default::default()
        },
        rect,
        8.0,
    );
    let value = authored_widget_node(
        NodeKind::Text,
        SceneWidget {
            kind: "select".into(),
            value_str: Some("night".into()),
            options: vec![SceneWidgetOption {
                value: "night".into(),
                label: "Night mode".into(),
            }],
            ..Default::default()
        },
        rect,
        8.0,
    );

    let placeholder_paint = paint(&placeholder, rect);
    let value_paint = paint(&value, rect);
    let placeholder_color = placeholder_paint
        .text_colors
        .iter()
        .find(|(text, _)| text == "Choose…")
        .map(|(_, color)| *color)
        .expect("placeholder color");
    let value_color = value_paint
        .text_colors
        .iter()
        .find(|(text, _)| text == "Night mode")
        .map(|(_, color)| *color)
        .expect("value color");

    assert_ne!(placeholder_color, value_color, "placeholder stays muted");
    assert!(placeholder_color.a() < value_color.a());
}

#[test]
fn select_option_label_borrows_matching_option_text() {
    let widget = SceneWidget {
        value_str: Some("pro".into()),
        options: vec![
            SceneWidgetOption {
                value: "basic".into(),
                label: "Basic".into(),
            },
            SceneWidgetOption {
                value: "pro".into(),
                label: "Pro Plan".into(),
            },
        ],
        ..Default::default()
    };

    let label = option_label(&widget, "pro").expect("selected label");

    assert!(std::ptr::eq(
        label.as_ptr(),
        widget.options[1].label.as_ptr()
    ));
}

#[test]
fn text_field_display_text_borrows_value_and_placeholder() {
    let with_value = SceneWidget {
        value_str: Some("hello".into()),
        placeholder: Some("Type here".into()),
        ..Default::default()
    };
    let (value, _) = text_field_display_text(&with_value).expect("value text");
    match value {
        Cow::Borrowed(text) => assert!(std::ptr::eq(
            text.as_ptr(),
            with_value.value_str.as_deref().unwrap().as_ptr()
        )),
        Cow::Owned(_) => panic!("value_str should be borrowed during paint"),
    }

    let with_placeholder = SceneWidget {
        placeholder: Some("Type here".into()),
        ..Default::default()
    };
    let (placeholder, _) = text_field_display_text(&with_placeholder).expect("placeholder text");
    match placeholder {
        Cow::Borrowed(text) => assert!(std::ptr::eq(
            text.as_ptr(),
            with_placeholder.placeholder.as_deref().unwrap().as_ptr()
        )),
        Cow::Owned(_) => panic!("placeholder should be borrowed during paint"),
    }
}

#[test]
fn radio_group_paints_circle_and_dot_for_selected() {
    let rect = Rect::xywh(0.0, 0.0, 120.0, 56.0);
    let node = authored_widget_node(
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
        7.0,
    );
    let b = paint(&node, rect);
    // Selected circle + contrast inner dot; unselected is border-only.
    assert_eq!(b.round_rects.len(), 2, "selected circle + inner dot");
    assert!(
        b.round_rects.iter().any(|(_, c)| *c == DARK_PURPLE),
        "selected circle uses authored fill"
    );
    assert_eq!(b.stroke_round_widths[0], 2.0, "authored radio stroke width");
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
fn widget_text_inset_accounts_for_leading_icon() {
    let mut w = SceneWidget {
        kind: "text_input".into(),
        ..Default::default()
    };
    assert_eq!(widget_text_inset_left(&w), 8.0);
    w.leading_icon = Some("mail".into());
    assert_eq!(widget_text_inset_left(&w), 36.0); // 8 + 20 + 8
}

#[test]
fn text_input_with_leading_icon_insets_text() {
    let rect = Rect::xywh(0.0, 0.0, 280.0, 48.0);
    let node = widget_node(
        NodeKind::Text,
        SceneWidget {
            kind: "text_input".into(),
            placeholder: Some("you@example.com".into()),
            leading_icon: Some("mail".into()),
            ..Default::default()
        },
        rect,
    );
    let b = paint(&node, rect);
    let (_, origin) = b
        .texts
        .iter()
        .find(|(t, _)| t == "you@example.com")
        .expect("placeholder painted");
    assert!(
        (origin.x - 36.0).abs() < 0.001,
        "text should be inset past the leading icon (8+20+8), got x={}",
        origin.x
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

#[path = "canvas_viewport_widget_tests/contract_closure.rs"]
mod contract_closure;
#[path = "canvas_viewport_widget_tests/stroke_roles.rs"]
mod stroke_roles;
#[path = "canvas_viewport_widget_tests/tabs.rs"]
mod tabs;
