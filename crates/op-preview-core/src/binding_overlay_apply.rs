//! Scene-side application of one bound / animated value to a
//! [`SceneNode`] — the `BindingTarget` → scene-field table shared by the
//! binding overlay pass and the animation override pass. Split out of
//! `binding_overlay.rs` to keep it under the 800-line cap.

use crate::scene_helpers::display_string;
use jian_core::binding::BindingTarget;
use jian_core::value::RuntimeValue;
use op_editor_ui::layout_scene::{SceneFillLayer, SceneNode, SceneStroke, SceneStrokeAlign};
use op_editor_ui::Color;

pub(crate) fn apply_value(node: &mut SceneNode, target: BindingTarget, value: &RuntimeValue) {
    match target {
        BindingTarget::Content => {
            node.text = Some(display_string(value));
            node.text_runs.clear();
        }
        BindingTarget::Value => apply_widget_value(node, value),
        BindingTarget::Checked => {
            if let (Some(widget), Some(checked)) = (node.widget.as_mut(), value.as_bool()) {
                widget.checked = Some(checked);
            }
        }
        BindingTarget::SelectedValue => {
            if let (Some(widget), Some(selected)) = (node.widget.as_mut(), value.as_str()) {
                widget.value_str = Some(selected.to_owned());
            }
        }
        BindingTarget::Visible => {
            if let Some(visible) = value.as_bool() {
                node.hidden = !visible;
            }
        }
        BindingTarget::Opacity => {
            if let Some(opacity) = number(value) {
                node.opacity = opacity.clamp(0.0, 1.0);
            }
        }
        BindingTarget::Fill | BindingTarget::TextColor => {
            if let Some(color) = color(value) {
                node.fill = Some(color);
                if let Some(SceneFillLayer::Solid {
                    color: layer_color, ..
                }) = node.fill_layers.first_mut()
                {
                    *layer_color = color;
                }
            }
        }
        BindingTarget::Stroke => {
            if let Some(color) = color(value) {
                if let Some(stroke) = node.stroke.as_mut() {
                    stroke.color = color;
                } else {
                    node.stroke = Some(SceneStroke {
                        color,
                        width: 1.0,
                        sides: None,
                        align: SceneStrokeAlign::Center,
                    });
                }
            }
        }
        BindingTarget::CornerRadius => {
            if let Some(radius) = number(value) {
                node.corner_radius = radius.max(0.0);
                node.corner_radii = None;
            }
        }
        BindingTarget::X => {
            if let Some(x) = number(value) {
                node.bounds.origin.x = x;
            }
        }
        BindingTarget::Y => {
            if let Some(y) = number(value) {
                node.bounds.origin.y = y;
            }
        }
        // Paint-only visual offsets (the preview's `transform: translate`):
        // the whole subtree shifts on screen while the solved layout and
        // the hit-test tree stay where they are, which is what lets
        // `$scroll`-driven parallax and rise-in entrances pass the
        // PaintOnly gate.
        BindingTarget::TranslateX => {
            if let Some(dx) = number(value) {
                translate_subtree(node, dx, 0.0);
            }
        }
        BindingTarget::TranslateY => {
            if let Some(dy) = number(value) {
                translate_subtree(node, 0.0, dy);
            }
        }
        BindingTarget::Width => {
            if let Some(width) = number(value) {
                node.bounds.size.x = width.max(0.0);
            }
        }
        BindingTarget::Height => {
            if let Some(height) = number(value) {
                node.bounds.size.y = height.max(0.0);
            }
        }
        BindingTarget::Rotation => {
            if let Some(degrees) = number(value) {
                node.rotation = degrees.to_radians();
            }
        }
        BindingTarget::ScaleX => {
            if let Some(scale) = number(value) {
                scale_axis(node, scale, true);
            }
        }
        BindingTarget::ScaleY => {
            if let Some(scale) = number(value) {
                scale_axis(node, scale, false);
            }
        }
        BindingTarget::Variant | BindingTarget::ActiveState => {
            apply_structural_state(node, value);
        }
    }
}

fn apply_widget_value(node: &mut SceneNode, value: &RuntimeValue) {
    let Some(widget) = node.widget.as_mut() else {
        return;
    };
    match widget.kind.as_str() {
        "switch" | "checkbox" => widget.checked = value.as_bool(),
        "slider" | "progress" | "number_input" => widget.value_num = number(value),
        _ => widget.value_str = value.as_str().map(str::to_owned),
    }
}

fn apply_structural_state(node: &mut SceneNode, value: &RuntimeValue) {
    if let Some(widget) = node.widget.as_mut() {
        match widget.kind.as_str() {
            "switch" | "checkbox" => widget.checked = value.as_bool(),
            _ => widget.value_str = value.as_str().map(str::to_owned),
        }
        return;
    }
    let Some(active) = value.as_str() else {
        return;
    };
    for child in &mut node.children {
        child.hidden = child.id != active;
    }
}

fn number(value: &RuntimeValue) -> Option<f32> {
    value
        .as_f64()
        .map(|number| number as f32)
        .or_else(|| value.as_i64().map(|number| number as f32))
}

fn color(value: &RuntimeValue) -> Option<Color> {
    let color = jian_core::scene::Color::from_hex(value.as_str()?)?;
    Some(Color::rgba_u8(
        color.r(),
        color.g(),
        color.b(),
        f32::from(color.a()) / 255.0,
    ))
}

fn scale_axis(node: &mut SceneNode, scale: f32, horizontal: bool) {
    let scale = scale.max(0.0);
    if horizontal {
        let center = node.bounds.origin.x + node.bounds.size.x / 2.0;
        node.bounds.size.x *= scale;
        node.bounds.origin.x = center - node.bounds.size.x / 2.0;
    } else {
        let center = node.bounds.origin.y + node.bounds.size.y / 2.0;
        node.bounds.size.y *= scale;
        node.bounds.origin.y = center - node.bounds.size.y / 2.0;
    }
}

/// Shift a scene subtree by a visual offset: bounds, cached aggregate
/// bounds, polygon points and path anchors all move together so
/// strokes, fills and text stay attached to the node.
fn translate_subtree(node: &mut SceneNode, dx: f32, dy: f32) {
    if !dx.is_finite() || !dy.is_finite() || (dx == 0.0 && dy == 0.0) {
        return;
    }
    node.bounds.origin.x += dx;
    node.bounds.origin.y += dy;
    if node.aggregate_bounds_cache.size.x > 0.0 || node.aggregate_bounds_cache.size.y > 0.0 {
        node.aggregate_bounds_cache.origin.x += dx;
        node.aggregate_bounds_cache.origin.y += dy;
    }
    for point in &mut node.points {
        point.x += dx;
        point.y += dy;
    }
    for anchor in &mut node.path_anchors {
        anchor.pos.x += dx;
        anchor.pos.y += dy;
        if let Some(handle) = &mut anchor.handle_in {
            handle.x += dx;
            handle.y += dy;
        }
        if let Some(handle) = &mut anchor.handle_out {
            handle.x += dx;
            handle.y += dy;
        }
    }
    for child in &mut node.children {
        translate_subtree(child, dx, dy);
    }
}
