use op_editor_ui::layout_scene::SceneNode;
use op_editor_ui::{Color, Rect};
use skia_safe::{Canvas, Paint, PaintStyle};

pub(super) fn paint(canvas: &Canvas, node: &SceneNode) {
    let Some(d) = node.svg_path.as_deref() else {
        return;
    };
    let Some(mut path) = fitted_path(d, node.bounds) else {
        return;
    };
    if let Some(fill) = node.fill {
        if has_multiple_close_commands(d) {
            path.set_fill_type(skia_safe::PathFillType::EvenOdd);
        }
        canvas.draw_path(&path, &make_fill(fill));
    }
    if let Some(stroke) = node.stroke {
        let mut paint = make_stroke(stroke.color, stroke.width);
        paint.set_stroke_cap(skia_safe::PaintCap::Round);
        paint.set_stroke_join(skia_safe::PaintJoin::Round);
        canvas.draw_path(&path, &paint);
    }
}

fn fitted_path(d: &str, rect: Rect) -> Option<skia_safe::Path> {
    let path = skia_safe::utils::parse_path::from_svg(d)?;
    Some(fit_path_to_rect(&path, rect))
}

fn fit_path_to_rect(path: &skia_safe::Path, rect: Rect) -> skia_safe::Path {
    let bounds = path.compute_tight_bounds();
    if !bounds.is_finite()
        || !rect.size.x.is_finite()
        || !rect.size.y.is_finite()
        || rect.size.x <= 0.0
        || rect.size.y <= 0.0
    {
        let mut matrix = skia_safe::Matrix::new_identity();
        matrix.set_translate((rect.origin.x, rect.origin.y));
        return path.with_transform(&matrix);
    }
    let native_w = bounds.width();
    let native_h = bounds.height();
    let sx = if native_w.abs() > 0.01 {
        rect.size.x / native_w
    } else {
        1.0
    };
    let sy = if native_h.abs() > 0.01 {
        rect.size.y / native_h
    } else {
        1.0
    };
    let tx = rect.origin.x - bounds.left() * sx;
    let ty = rect.origin.y - bounds.top() * sy;
    let mut matrix = skia_safe::Matrix::new_identity();
    matrix.set_scale_translate((sx, sy), (tx, ty));
    path.with_transform(&matrix)
}

fn make_fill(c: Color) -> Paint {
    let mut paint = Paint::new(color_to_sk(c), None);
    paint.set_anti_alias(true);
    paint
}

fn make_stroke(c: Color, width: f32) -> Paint {
    let mut paint = Paint::new(color_to_sk(c), None);
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(width.max(0.5));
    paint
}

fn color_to_sk(c: Color) -> skia_safe::Color4f {
    skia_safe::Color4f::new(c.r, c.g, c.b, c.a)
}

fn has_multiple_close_commands(d: &str) -> bool {
    let mut closes = 0;
    for byte in d.bytes() {
        if byte == b'Z' || byte == b'z' {
            closes += 1;
            if closes > 1 {
                return true;
            }
        }
    }
    false
}
