//! Element -> `PenNode` conversion: the style-aware recursive builder
//! plus the per-shape leaf/path constructors.

use super::*;

/// Build a `PenNode` for an SVG element with inherited style
/// context + viewBox scaling applied. `<g>` recurses into its
/// children and wraps them in a `PenNode::Group`.
pub(super) fn element_to_node_ctx(
    el: &SvgTree,
    parent_ctx: &StyleCtx,
    scale: f64,
    offset: (f64, f64),
    allocator: &mut dyn crate::IdAllocator,
    taken: &mut std::collections::HashSet<NodeId>,
) -> Result<Option<PenNode>, crate::IdAllocError> {
    let ctx = merge_style_ctx(parent_ctx, &el.attrs);
    if el.tag == "g" || el.tag == "svg" {
        let mut kids: Vec<PenNode> = Vec::new();
        for child in &el.children {
            if let Some(node) = element_to_node_ctx(child, &ctx, scale, offset, allocator, taken)? {
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
    // Convert via the legacy element builder, then scale + apply
    // inherited fill/stroke.
    let legacy = SvgElement {
        tag: el.tag.clone(),
        attrs: el.attrs.clone(),
    };
    // Scale the geometry by writing scaled values back into the
    // element's attrs (so existing `element_to_node` sees scaled
    // numbers). For `<path>` we tokenise the `d` string + scale
    // every coord — TS parity with `scaleSvgPath`.
    let mut scaled = legacy;
    apply_scale_to_attrs(&mut scaled, scale);
    // Preserve `<path d>` as SVG path data. That keeps arcs and
    // compound subpaths in the same representation the TS renderer
    // sends to CanvasKit.
    let fill_hex = resolve_svg_fill_hex(&scaled.attrs, &ctx);
    let stroke = resolve_svg_stroke(&scaled.attrs, &ctx, scale);
    if scaled.tag == "path" {
        if let Some(d) = scaled.attr("d") {
            let id = allocator.allocate(taken)?;
            return Ok(path_node_from_svg_d(
                id,
                el.attr("id").unwrap_or("Path"),
                d,
                offset,
                fill_hex,
                stroke,
                ctx.fill_rule,
            ));
        }
    }
    let id = allocator.allocate(taken)?;
    let Some(mut node) = element_to_node(&scaled, id, offset) else {
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
