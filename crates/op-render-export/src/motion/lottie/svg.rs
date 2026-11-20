//! Outlined canvas paint to Lottie shapes. Unsupported paint returns None.
use super::{
    path::contours,
    properties::{property, transform},
};
use serde_json::{json, Value};
use usvg::{
    tiny_skia_path::{Path, Point, Transform},
    FillRule, Group, Node, Paint, SpreadMethod,
};

pub(super) fn shapes(bytes: &[u8]) -> Option<Value> {
    let tree = usvg::Tree::from_data(bytes, &usvg::Options::default()).ok()?;
    Some(Value::Array(group(tree.root())?))
}

fn group(node: &Group) -> Option<Vec<Value>> {
    if node.clip_path().is_some()
        || node.mask().is_some()
        || !node.filters().is_empty()
        || node.blend_mode() != usvg::BlendMode::Normal
    {
        return None;
    }
    // Shape groups use painter order, the reverse of composition layers.
    node.children()
        .iter()
        .map(|child| match child {
            Node::Group(child) => {
                let mut items = group(child)?;
                let mut tr = transform();
                tr["ty"] = json!("tr");
                tr["o"] = property(child.opacity().get() * 100.0);
                items.push(tr);
                Some(json!({"ty":"gr", "it":items}))
            }
            Node::Path(path) => path_shapes(path),
            _ => None,
        })
        .collect()
}

fn path_shapes(path: &usvg::Path) -> Option<Value> {
    let mut items = Vec::new();
    if let Some(fill) = path.fill() {
        items.push(painted_path(
            path.data(),
            path.abs_transform(),
            fill.paint(),
            fill.opacity().get(),
            fill.rule(),
        )?);
    }
    if let Some(stroke) = path.stroke() {
        let matrix = path.abs_transform();
        let sx = matrix.sx.hypot(matrix.ky);
        let sy = matrix.kx.hypot(matrix.sy);
        if (sx - sy).abs() > 0.001 || (matrix.sx * matrix.kx + matrix.ky * matrix.sy).abs() > 0.001
        {
            return None;
        }
        let mut stroked = painted_path(
            path.data(),
            matrix,
            stroke.paint(),
            stroke.opacity().get(),
            FillRule::NonZero,
        )?;
        let shape_items = stroked["it"].as_array_mut()?;
        let style = shape_items
            .iter_mut()
            .find(|v| matches!(v["ty"].as_str(), Some("fl" | "gf")))?;
        style["ty"] = json!(if matches!(stroke.paint(), Paint::Color(_)) {
            "st"
        } else {
            "gs"
        });
        style["w"] = property(stroke.width().get() * sx);
        style["lc"] = json!(match stroke.linecap() {
            usvg::LineCap::Butt => 1,
            usvg::LineCap::Round => 2,
            usvg::LineCap::Square => 3,
        });
        style["lj"] = json!(match stroke.linejoin() {
            usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => 1,
            usvg::LineJoin::Round => 2,
            usvg::LineJoin::Bevel => 3,
        });
        style["ml"] = json!(stroke.miterlimit().get());
        if let Some(dashes) = stroke.dasharray() {
            let mut entries: Vec<_> = dashes
                .iter()
                .enumerate()
                .map(|(i, value)| {
                    json!({
                "n":if i%2==0 {"d"} else {"g"},"v":property(value*sx)})
                })
                .collect();
            entries.push(json!({"n":"o","v":property(stroke.dashoffset()*sx)}));
            style["d"] = json!(entries);
        }
        items.push(stroked);
    }
    if path.paint_order() == usvg::PaintOrder::StrokeAndFill {
        items.reverse();
    }
    let mut tr = transform();
    tr["ty"] = json!("tr");
    items.push(tr);
    Some(json!({"ty":"gr", "it":items}))
}

fn painted_path(
    path: &Path,
    matrix: Transform,
    paint: &Paint,
    opacity: f32,
    rule: FillRule,
) -> Option<Value> {
    let mut items: Vec<_> = contours(&path.clone().transform(matrix)?)
        .into_iter()
        .map(|shape| json!({"ty":"sh", "ks":property(shape)}))
        .collect();
    let mut fill = fill(paint, matrix, opacity)?;
    fill["r"] = json!(if rule == FillRule::EvenOdd { 2 } else { 1 });
    items.push(fill);
    let mut tr = transform();
    tr["ty"] = json!("tr");
    items.push(tr);
    Some(json!({"ty":"gr", "it":items}))
}

fn fill(paint: &Paint, matrix: Transform, opacity: f32) -> Option<Value> {
    let (gradient, start, end, kind) = match paint {
        Paint::Color(color) => {
            return Some(json!({"ty":"fl", "o":property(opacity*100.0),
            "c":property(json!([f32::from(color.red)/255.0,f32::from(color.green)/255.0,f32::from(color.blue)/255.0,1]))}))
        }
        Paint::LinearGradient(g) => (
            &***g as &usvg::BaseGradient,
            [g.x1(), g.y1()],
            [g.x2(), g.y2()],
            1,
        ),
        Paint::RadialGradient(g) if g.cx() == g.fx() && g.cy() == g.fy() => {
            let m = matrix.pre_concat(g.transform());
            let sx = m.sx.hypot(m.ky);
            let sy = m.kx.hypot(m.sy);
            if (sx - sy).abs() > 0.001 {
                return None;
            }
            (
                &***g as &usvg::BaseGradient,
                [g.cx(), g.cy()],
                [g.cx() + g.r().get(), g.cy()],
                2,
            )
        }
        _ => return None,
    };
    if gradient.spread_method() != SpreadMethod::Pad {
        return None;
    }
    let matrix = matrix.pre_concat(gradient.transform());
    let map = |xy: [f32; 2]| {
        let mut point = Point::from_xy(xy[0], xy[1]);
        matrix.map_point(&mut point);
        json!([point.x, point.y])
    };
    let mut stops = Vec::new();
    for stop in gradient.stops() {
        let c = stop.color();
        stops.extend([
            stop.offset().get(),
            f32::from(c.red) / 255.0,
            f32::from(c.green) / 255.0,
            f32::from(c.blue) / 255.0,
        ]);
    }
    for stop in gradient.stops() {
        stops.extend([stop.offset().get(), stop.opacity().get()]);
    }
    Some(
        json!({"ty":"gf","t":kind,"o":property(opacity*100.0), "s":property(map(start)),"e":property(map(end)),
        "g":{"p":gradient.stops().len(),"k":property(json!(stops))},"h":property(0),"a":property(0)}),
    )
}
