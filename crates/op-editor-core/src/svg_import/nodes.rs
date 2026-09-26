//! Element -> `PenNode` conversion: the style-aware recursive builder
//! plus the per-shape leaf/path constructors.

use super::*;

/// Build a `PenNode` for an SVG element with inherited style
/// context + the composed transform applied. `<g>` recurses into its
/// children and wraps them in a `PenNode::Group`.
///
/// `parent_mat` is everything above this element — the canvas
/// offset, the viewBox scale, and every ancestor `transform` — and
/// the element's own `transform` composes onto it here. What the
/// leaf does with the result depends on its shape: a translate plus
/// uniform scale keeps a `<rect>` a Rect and a `<circle>` an Ellipse
/// (attrs scaled in place, as the root scale always was); a rotation,
/// skew or non-uniform scale traces the shape as a Path whose points
/// carry the matrix, since the node model has no axis-independent
/// scale or skew to hold it otherwise.
pub(super) fn element_to_node_ctx(
    el: &SvgTree,
    parent_ctx: &StyleCtx,
    parent_mat: Mat,
    allocator: &mut dyn crate::IdAllocator,
    taken: &mut std::collections::HashSet<NodeId>,
) -> Result<Option<PenNode>, crate::IdAllocError> {
    let ctx = merge_style_ctx(parent_ctx, &el.attrs);
    let mat = match el.attr("transform") {
        Some(raw) => parent_mat.then(parse_transform(raw)),
        None => parent_mat,
    };
    if el.tag == "g" || el.tag == "svg" {
        let mut kids: Vec<PenNode> = Vec::new();
        for child in &el.children {
            if let Some(node) = element_to_node_ctx(child, &ctx, mat, allocator, taken)? {
                kids.push(node);
            }
        }
        if kids.is_empty() {
            return Ok(None);
        }
        if kids.len() == 1 {
            return Ok(kids.into_iter().next());
        }
        let id = allocator.allocate(taken)?;
        use jian_ops_schema::node::container::ContainerProps;
        use jian_ops_schema::node::GroupNode;
        let name = el
            .attr("id")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Group".to_string());
        return Ok(Some(PenNode::Group(GroupNode {
            base: PenNodeBase {
                id: id.into(),
                name: Some(name),
                ..Default::default()
            },
            container: ContainerProps::default(),
            children: Some(kids),
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        })));
    }
    let leaf = SvgElement {
        tag: el.tag.clone(),
        attrs: el.attrs.clone(),
    };
    let fill_hex = resolve_svg_fill_hex(&leaf.attrs, &ctx);
    let stroke = resolve_svg_stroke(&leaf.attrs, &ctx, mat.stroke_scale());
    // Preserve `<path d>` as SVG path data, carried through the
    // matrix. That keeps arcs and compound subpaths in the same
    // representation the TS renderer sends to CanvasKit.
    if leaf.tag == "path" {
        if let Some(d) = leaf.attr("d") {
            let id = allocator.allocate(taken)?;
            return Ok(transformed_path_node(
                id,
                el.attr("id").unwrap_or("Path"),
                d,
                mat,
                fill_hex,
                stroke,
                ctx.fill_rule,
            ));
        }
    }
    let Some((scale, ox, oy)) = mat.as_translate_uniform_scale() else {
        // Rotated / skewed / anisotropically scaled: the shape becomes
        // a path so its geometry can carry the matrix exactly.
        let Some(d) = shape_to_path_d(&leaf) else {
            return Ok(None);
        };
        let id = allocator.allocate(taken)?;
        return Ok(transformed_path_node(
            id,
            shape_name(&leaf),
            &d,
            mat,
            fill_hex,
            stroke,
            ctx.fill_rule,
        ));
    };
    // Scale the geometry by writing scaled values back into the
    // element's attrs (so existing `element_to_node` sees scaled
    // numbers) — TS parity with `scaleSvgPath`.
    let mut scaled = leaf;
    apply_scale_to_attrs(&mut scaled, scale);
    let id = allocator.allocate(taken)?;
    let Some(mut node) = element_to_node(&scaled, id, (ox, oy)) else {
        return Ok(None);
    };
    if let PenNode::Path(path) = &mut node {
        path.fill_rule = ctx.fill_rule;
    }
    if let Some(stroke) = stroke {
        set_node_stroke(&mut node, stroke);
    }
    if fill_hex.is_none() && scaled.tag != "line" && scaled.tag != "polyline" {
        // Explicit fill="none" / fill="transparent" should not leave
        // the old element builder's attribute fill behind.
        clear_node_fill(&mut node);
    } else if let Some(hex) = fill_hex.as_deref() {
        set_primary_fill_hex(&mut node, hex);
    }
    Ok(Some(node))
}

/// Build a `PenNode` from one parsed SVG element. `None` for
/// unsupported tags (`g` / `svg` / `defs` / `style` / …) and for
/// degenerate geometry.
fn element_to_node(el: &SvgElement, id: NodeId, offset: (f64, f64)) -> Option<PenNode> {
    let (ox, oy) = offset;
    let fill = el.attr("fill").and_then(parse_svg_color);
    let mut node = match el.tag.as_str() {
        "rect" => {
            let (w, h) = (el.num("width"), el.num("height"));
            if w <= 0.0 || h <= 0.0 {
                return None;
            }
            build_leaf_node(
                "rect",
                id.as_str(),
                "Rect",
                (el.num("x") + ox).round() as i32,
                (el.num("y") + oy).round() as i32,
                w.round() as i32,
                h.round() as i32,
            )?
        }
        "circle" => {
            let r = el.num("r");
            if r <= 0.0 {
                return None;
            }
            build_leaf_node(
                "ellipse",
                id.as_str(),
                "Ellipse",
                (el.num("cx") - r + ox).round() as i32,
                (el.num("cy") - r + oy).round() as i32,
                (r * 2.0).round() as i32,
                (r * 2.0).round() as i32,
            )?
        }
        "ellipse" => {
            let (rx, ry) = (el.num("rx"), el.num("ry"));
            if rx <= 0.0 || ry <= 0.0 {
                return None;
            }
            build_leaf_node(
                "ellipse",
                id.as_str(),
                "Ellipse",
                (el.num("cx") - rx + ox).round() as i32,
                (el.num("cy") - ry + oy).round() as i32,
                (rx * 2.0).round() as i32,
                (ry * 2.0).round() as i32,
            )?
        }
        "line" => {
            let p0 = (el.num("x1") + ox, el.num("y1") + oy);
            let p1 = (el.num("x2") + ox, el.num("y2") + oy);
            path_node_from_anchors(id, "Line", &[p0, p1], false)?
        }
        "polyline" | "polygon" => {
            let pts: Vec<(f64, f64)> = parse_point_list(el.attr("points").unwrap_or(""))
                .into_iter()
                .map(|(x, y)| (x + ox, y + oy))
                .collect();
            if pts.len() < 2 {
                return None;
            }
            path_node_from_anchors(id, "Path", &pts, el.tag == "polygon")?
        }
        "path" => {
            // Multi-subpath handling lives in `element_to_node_ctx`
            // where we have the id allocator + can build a Group;
            // this legacy single-id path serves only the first subpath
            // so existing callers keep working.
            let d = el.attr("d")?;
            let mut subpaths = parse_path_d(d, offset);
            let (anchors, closed) = subpaths.drain(..).next()?;
            if anchors.len() < 2 {
                return None;
            }
            path_node_from_pen_anchors(id, anchors, closed)?
        }
        // <svg> / <g> / <defs> / <style> / <title> / … — skipped.
        _ => return None,
    };
    if let Some(hex) = fill {
        set_primary_fill_hex(&mut node, &hex);
    }
    Some(node)
}

/// Build a straight-segment `Path` node from doc-space points.
fn path_node_from_anchors(
    id: NodeId,
    name: &str,
    pts: &[(f64, f64)],
    closed: bool,
) -> Option<PenNode> {
    let anchors: Vec<PenPathAnchor> = pts
        .iter()
        .map(|&(x, y)| PenPathAnchor {
            x,
            y,
            handle_in: None,
            handle_out: None,
            point_type: None,
        })
        .collect();
    path_node_from_pen_anchors(id, anchors, closed).map(|mut n| {
        n.base_mut().name = Some(name.to_string());
        n
    })
}

/// Build a `Path` node from ready `PenPathAnchor`s, fitting the base
/// rect to the anchor bounding box.
fn path_node_from_pen_anchors(
    id: NodeId,
    anchors: Vec<PenPathAnchor>,
    closed: bool,
) -> Option<PenNode> {
    if anchors.len() < 2 {
        return None;
    }
    let (min_x, min_y, max_x, max_y) = crate::svg_path_bounds::path_anchor_bounds(&anchors, closed);
    Some(PenNode::Path(PathNode {
        base: PenNodeBase {
            id: id.into(),
            name: Some("Path".to_string()),
            x: Some(min_x),
            y: Some(min_y),
            ..Default::default()
        },
        icon_id: None,
        d: None,
        anchors: Some(anchors),
        closed: Some(closed),
        fill_rule: None,
        mask: None,
        width: Some(SizingBehavior::Number((max_x - min_x).max(0.0))),
        height: Some(SizingBehavior::Number((max_y - min_y).max(0.0))),
        fill: None,
        stroke: None,
        effects: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
        limits: Default::default(),
    }))
}

/// A `<path>` (or a shape traced as one) with `mat` applied to its
/// `d`, built by the preserve-`d` constructor.
fn transformed_path_node(
    id: NodeId,
    name: &str,
    d: &str,
    mat: Mat,
    fill_hex: Option<String>,
    stroke: Option<PenStroke>,
    fill_rule: Option<PathFillRule>,
) -> Option<PenNode> {
    let d =
        crate::svg_path_data::transform_svg_path(d, [mat.a, mat.b, mat.c, mat.d, mat.e, mat.f])?;
    path_node_from_svg_d(id, name, &d, (0.0, 0.0), fill_hex, stroke, fill_rule)
}

/// The layer name a shape keeps when it has to become a path.
fn shape_name(el: &SvgElement) -> &str {
    el.attr("id").unwrap_or(match el.tag.as_str() {
        "rect" => "Rect",
        "circle" | "ellipse" => "Ellipse",
        "line" => "Line",
        _ => "Path",
    })
}

/// The shape's outline as SVG path data in its own (untransformed)
/// coordinates: rects with optional rounded corners, circles and
/// ellipses as four arcs, lines and point lists as straight runs.
/// `None` for degenerate geometry, matching what `element_to_node`
/// rejects.
fn shape_to_path_d(el: &SvgElement) -> Option<String> {
    fn num(n: f64) -> String {
        let n = if n.abs() < 1e-9 { 0.0 } else { n };
        format!("{n:.6}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
    let d = match el.tag.as_str() {
        "rect" => {
            let (x, y, w, h) = (el.num("x"), el.num("y"), el.num("width"), el.num("height"));
            if w <= 0.0 || h <= 0.0 {
                return None;
            }
            // SVG: a missing radius copies the other; both clamp to half the side.
            let rx_attr = el.attr("rx").and_then(|v| v.trim().parse::<f64>().ok());
            let ry_attr = el.attr("ry").and_then(|v| v.trim().parse::<f64>().ok());
            let rx = rx_attr.or(ry_attr).unwrap_or(0.0).max(0.0).min(w / 2.0);
            let ry = ry_attr.or(rx_attr).unwrap_or(0.0).max(0.0).min(h / 2.0);
            if rx <= 0.0 || ry <= 0.0 {
                format!(
                    "M {} {} L {} {} L {} {} L {} {} Z",
                    num(x),
                    num(y),
                    num(x + w),
                    num(y),
                    num(x + w),
                    num(y + h),
                    num(x),
                    num(y + h)
                )
            } else {
                let arc = |to_x: f64, to_y: f64| {
                    format!(
                        "A {} {} 0 0 1 {} {}",
                        num(rx),
                        num(ry),
                        num(to_x),
                        num(to_y)
                    )
                };
                format!(
                    "M {} {} L {} {} {} L {} {} {} L {} {} {} L {} {} {} Z",
                    num(x + rx),
                    num(y),
                    num(x + w - rx),
                    num(y),
                    arc(x + w, y + ry),
                    num(x + w),
                    num(y + h - ry),
                    arc(x + w - rx, y + h),
                    num(x + rx),
                    num(y + h),
                    arc(x, y + h - ry),
                    num(x),
                    num(y + ry),
                    arc(x + rx, y),
                )
            }
        }
        "circle" | "ellipse" => {
            let (cx, cy) = (el.num("cx"), el.num("cy"));
            let (rx, ry) = if el.tag == "circle" {
                (el.num("r"), el.num("r"))
            } else {
                (el.num("rx"), el.num("ry"))
            };
            if rx <= 0.0 || ry <= 0.0 {
                return None;
            }
            let arc = |to_x: f64, to_y: f64| {
                format!(
                    "A {} {} 0 0 1 {} {}",
                    num(rx),
                    num(ry),
                    num(to_x),
                    num(to_y)
                )
            };
            format!(
                "M {} {} {} {} {} {} Z",
                num(cx + rx),
                num(cy),
                arc(cx, cy + ry),
                arc(cx - rx, cy),
                arc(cx, cy - ry),
                arc(cx + rx, cy),
            )
        }
        "line" => format!(
            "M {} {} L {} {}",
            num(el.num("x1")),
            num(el.num("y1")),
            num(el.num("x2")),
            num(el.num("y2"))
        ),
        "polyline" | "polygon" => {
            let pts = parse_point_list(el.attr("points").unwrap_or(""));
            if pts.len() < 2 {
                return None;
            }
            let mut d = String::new();
            for (i, (x, y)) in pts.iter().enumerate() {
                if i > 0 {
                    d.push(' ');
                }
                d.push(if i == 0 { 'M' } else { 'L' });
                d.push(' ');
                d.push_str(&num(*x));
                d.push(' ');
                d.push_str(&num(*y));
            }
            if el.tag == "polygon" {
                d.push_str(" Z");
            }
            d
        }
        _ => return None,
    };
    Some(d)
}

fn path_node_from_svg_d(
    id: NodeId,
    name: &str,
    d: &str,
    offset: (f64, f64),
    fill_hex: Option<String>,
    stroke: Option<PenStroke>,
    fill_rule: Option<PathFillRule>,
) -> Option<PenNode> {
    let (local_d, bounds) = crate::svg_path_data::localize_svg_path(d)?;
    Some(PenNode::Path(PathNode {
        base: PenNodeBase {
            id: id.into(),
            name: Some(name.to_string()),
            x: Some(bounds.x + offset.0),
            y: Some(bounds.y + offset.1),
            ..Default::default()
        },
        icon_id: None,
        d: Some(local_d.clone()),
        anchors: None,
        closed: Some(local_d.contains('Z') || local_d.contains('z')),
        fill_rule,
        mask: None,
        width: Some(SizingBehavior::Number(bounds.w)),
        height: Some(SizingBehavior::Number(bounds.h)),
        fill: fill_hex.map(|hex| vec![solid_fill(&hex)]),
        stroke,
        effects: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
        limits: Default::default(),
    }))
}
