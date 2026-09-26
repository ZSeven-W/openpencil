//! Adjacent-label colour for checkbox / radio on the design canvas: the label
//! is page text, so checked state and accent paint must not decide it.
//!
//! Split out of the `canvas_viewport_widget_tests.rs` spine (800-line
//! ceiling); the shared recorder + node fixtures come in via `use super::*`.

use super::*;

const ACCENT: Color = Color::rgb_u8(0x25, 0x63, 0xeb);
const NEUTRAL_BORDER: Color = Color::rgb_u8(0xd1, 0xd5, 0xdb);

fn luminance(c: jian_core::scene::Color) -> f64 {
    fn linear(byte: u8) -> f64 {
        let v = f64::from(byte) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * linear(c.r()) + 0.7152 * linear(c.g()) + 0.0722 * linear(c.b())
}

fn contrast_vs_white(c: jian_core::scene::Color) -> f64 {
    1.05 / (luminance(c) + 0.05)
}

fn accent_stroke(color: Color) -> SceneStroke {
    SceneStroke {
        color,
        width: 1.0,
        sides: None,
        align: SceneStrokeAlign::Center,
    }
}

fn checkbox(checked: bool, fill: Color, stroke: Color, page: Option<Color>) -> SceneNode {
    let mut node = widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "checkbox".into(),
            checked: Some(checked),
            label: Some("Task".into()),
            label_foreground: page,
            ..Default::default()
        },
        Rect::xywh(0.0, 0.0, 120.0, 22.0),
    );
    node.fill = Some(fill);
    node.stroke = Some(accent_stroke(stroke));
    node
}

fn label_color(node: &SceneNode, text: &str) -> jian_core::scene::Color {
    let b = paint(node, node.bounds);
    b.text_colors
        .iter()
        .find(|(t, _)| t == text)
        .map(|(_, c)| *c)
        .expect("label painted")
}

#[test]
fn checked_accent_checkbox_label_is_readable_on_a_white_page() {
    let node = checkbox(true, ACCENT, ACCENT, None);
    let color = label_color(&node, "Task");
    assert!(
        contrast_vs_white(color) >= 4.5,
        "checked label {color:?} must reach 4.5:1 on white"
    );
}

#[test]
fn unchecked_neutral_border_label_stays_black() {
    let node = checkbox(false, WHITE, NEUTRAL_BORDER, None);
    assert_eq!(
        label_color(&node, "Task"),
        jian_core::scene::Color::rgb(0x00, 0x00, 0x00)
    );
}

#[test]
fn document_foreground_is_the_label_colour_for_both_states() {
    let page = Color::rgb_u8(0x11, 0x18, 0x27);
    let expected = jian_core::scene::Color::rgb(0x11, 0x18, 0x27);
    let checked = checkbox(true, ACCENT, ACCENT, Some(page));
    let unchecked = checkbox(false, WHITE, NEUTRAL_BORDER, Some(page));
    assert_eq!(label_color(&checked, "Task"), expected);
    assert_eq!(label_color(&unchecked, "Task"), expected);
}

#[test]
fn dark_page_foreground_paints_light_labels() {
    let page = Color::rgb_u8(0xf1, 0xf5, 0xf9);
    let node = checkbox(true, ACCENT, ACCENT, Some(page));
    assert_eq!(
        label_color(&node, "Task"),
        jian_core::scene::Color::rgb(0xf1, 0xf5, 0xf9)
    );
}

#[test]
fn selected_radio_with_accent_stroke_keeps_a_readable_label() {
    let rect = Rect::xywh(0.0, 0.0, 160.0, 28.0);
    let mut node = widget_node(
        NodeKind::Rect,
        SceneWidget {
            kind: "radio_group".into(),
            value_str: Some("a".into()),
            options: vec![SceneWidgetOption {
                value: "a".into(),
                label: "Alpha".into(),
            }],
            ..Default::default()
        },
        rect,
    );
    node.fill = Some(ACCENT);
    node.stroke = Some(accent_stroke(ACCENT));
    assert!(contrast_vs_white(label_color(&node, "Alpha")) >= 4.5);
}

#[test]
fn labelled_checkbox_label_region_holds_the_measured_label() {
    use jian_core::render::widget_metrics::{labelled_checkbox_fit_width, CHECKBOX_LABEL_GAP};

    // A width sized by the shared fit rule must leave the whole label unclipped.
    let label_w = 84.0;
    let width = labelled_checkbox_fit_width(22.0, label_w);
    let mut node = checkbox(false, WHITE, NEUTRAL_BORDER, None);
    node.bounds = Rect::xywh(0.0, 0.0, width, 22.0);
    let b = paint(&node, node.bounds);
    assert_eq!(b.clips.len(), 1);
    assert_eq!(b.clips[0].origin.x, 22.0 + CHECKBOX_LABEL_GAP);
    assert!(b.clips[0].size.x >= label_w);
}
