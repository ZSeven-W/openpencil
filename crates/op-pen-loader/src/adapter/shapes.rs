//! Per-variant `NodePayload` builders — one function per `PenNode`
//! variant, plus the text styling helpers they share.

use super::*;

pub(super) fn frame_to_payload(n: &FrameNode, _rects: &BTreeMap<String, [f32; 4]>) -> NodePayload {
    let mut p = base_payload(&n.base, "frame");
    p.clip_content = n.container.clip_content == Some(true);
    apply_container_style(
        &mut p,
        n.container.fill.as_deref(),
        n.container.stroke.as_ref(),
        n.container.corner_radius.as_ref(),
    );
    let child_text_center = text_child_center_context(&n.base, &n.container, _rects);
    p.children = n
        .children
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|c| node_to_payload_with_text_context(c, _rects, child_text_center))
        .collect();
    p
}

pub(super) fn group_to_payload(n: &GroupNode, _rects: &BTreeMap<String, [f32; 4]>) -> NodePayload {
    let mut p = base_payload(&n.base, "group");
    p.clip_content = n.container.clip_content == Some(true);
    apply_container_style(
        &mut p,
        n.container.fill.as_deref(),
        n.container.stroke.as_ref(),
        n.container.corner_radius.as_ref(),
    );
    let child_text_center = text_child_center_context(&n.base, &n.container, _rects);
    p.children = n
        .children
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|c| node_to_payload_with_text_context(c, _rects, child_text_center))
        .collect();
    p
}

pub(super) fn rect_to_payload(
    n: &RectangleNode,
    _rects: &BTreeMap<String, [f32; 4]>,
) -> NodePayload {
    let mut p = base_payload(&n.base, "rect");
    p.clip_content = n.container.clip_content == Some(true);
    apply_container_style(
        &mut p,
        n.container.fill.as_deref(),
        n.container.stroke.as_ref(),
        n.container.corner_radius.as_ref(),
    );
    let child_text_center = text_child_center_context(&n.base, &n.container, _rects);
    p.children = n
        .children
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|c| node_to_payload_with_text_context(c, _rects, child_text_center))
        .collect();
    p
}

fn text_child_center_context(
    base: &PenNodeBase,
    container: &ContainerProps,
    rects: &BTreeMap<String, [f32; 4]>,
) -> Option<TextChildCenterContext> {
    if !matches!(container.layout, Some(LayoutMode::Vertical))
        || !matches!(container.align_items, Some(AlignItems::Center))
    {
        return None;
    }
    let [x, _, w, _] = rects.get(&base.id).copied()?;
    let (padding_left, padding_right) = padding_left_right(container.padding.as_ref());
    Some(TextChildCenterContext {
        x: x + padding_left,
        w: (w - padding_left - padding_right).max(0.0),
    })
}

fn padding_left_right(padding: Option<&Padding>) -> (f32, f32) {
    match padding {
        Some(Padding::Uniform(v)) => {
            let v = *v as f32;
            (v, v)
        }
        Some(Padding::XY([_, h])) => {
            let h = *h as f32;
            (h, h)
        }
        Some(Padding::LtrB([_, r, _, l])) => (*l as f32, *r as f32),
        _ => (0.0, 0.0),
    }
}

pub(super) fn ellipse_to_payload(n: &EllipseNode) -> NodePayload {
    let mut p = base_payload(&n.base, "ellipse");
    // jian-core's `leaf_size` doesn't expose ellipse dimensions to
    // taffy, so the computed rect comes back zero. Seed authored
    // numeric size here as the fallback for `apply_computed_rect`.
    p.w = sizing_to_f32(&n.width);
    p.h = sizing_to_f32(&n.height);
    assign_first_fill(&mut p, n.fill.as_deref());
    p.stroke = stroke_to_payload(n.stroke.as_ref());
    p.corner_radius = n.corner_radius.unwrap_or(0.0) as f32;
    // Arc geometry — only carried when authored, so a plain ellipse
    // still paints as a full oval.
    p.arc_start_angle = n.start_angle.map(|a| a as f32);
    p.arc_sweep_angle = n.sweep_angle.map(|a| a as f32);
    p.arc_inner_radius = n.inner_radius.map(|r| r as f32);
    p
}

pub(super) fn line_to_payload(n: &LineNode) -> NodePayload {
    let mut p = base_payload(&n.base, "line");
    // The shell's line painter draws from `bounds.origin` to
    // `bounds.origin + bounds.size`, so encoding `(x2, y2)` as
    // a signed size puts both endpoints exactly where the
    // canonical schema says they are. Negative components shift
    // `origin` so `aggregate_bounds` (which gates on `size > 0`)
    // still picks the rect up — a horizontal/vertical/diagonal
    // line is direction-invariant for paint purposes anyway.
    let x2 = n.x2.unwrap_or(0.0) as f32;
    let y2 = n.y2.unwrap_or(0.0) as f32;
    if x2 < 0.0 {
        p.x += x2;
        p.w = -x2;
    } else {
        p.w = x2;
    }
    if y2 < 0.0 {
        p.y += y2;
        p.h = -y2;
    } else {
        p.h = y2;
    }
    if p.w == 0.0 && p.h == 0.0 {
        p.w = 1.0;
    }
    p.stroke = stroke_to_payload(n.stroke.as_ref());
    if p.stroke.is_none() {
        p.stroke = Some(StrokePayload {
            color: Some([0.0, 0.0, 0.0, 1.0]),
            width: 1.0,
            sides: None,
            align: 0,
        });
    }
    p
}

pub(super) fn polygon_to_payload(n: &PolygonNode) -> NodePayload {
    let mut p = base_payload(&n.base, "polygon");
    // Same as ellipse — jian-core's leaf_size doesn't expose
    // polygon dimensions to taffy, so we must seed authored size.
    p.w = sizing_to_f32(&n.width);
    p.h = sizing_to_f32(&n.height);
    assign_first_fill(&mut p, n.fill.as_deref());
    p.stroke = stroke_to_payload(n.stroke.as_ref());
    p.corner_radius = n.corner_radius.unwrap_or(0.0) as f32;
    p.polygon_sides = n.polygon_count.clamp(3, 100);
    p
}

pub(super) fn path_to_payload(n: &PathNode) -> NodePayload {
    let mut p = base_payload(&n.base, "path");
    p.path_closed = n.closed.unwrap_or(false);
    // `PathNode.mask` predates the shared, typed mask field. Preserve old
    // documents by interpreting `true` as the historical/default ALPHA mode.
    if p.mask_type.is_none() && n.mask.unwrap_or(false) {
        p.mask_type = Some(jian_ops_schema::node::MaskType::Alpha);
    }
    p.is_mask = p.mask_type.is_some();
    p.even_odd_fill = matches!(
        n.fill_rule,
        Some(jian_ops_schema::node::PathFillRule::Evenodd)
    );
    p.w = sizing_to_f32(&n.width);
    p.h = sizing_to_f32(&n.height);
    assign_first_fill(&mut p, n.fill.as_deref());
    p.stroke = stroke_to_payload(n.stroke.as_ref());
    p.svg_path = n.d.clone();
    if let Some(anchors) = &n.anchors {
        // `points` is the path's anchor polyline — kept 1:1 with the
        // schema anchors so the pen-tool anchor hit-test (which maps a
        // `points` index straight onto an anchor index) stays correct.
        // Editable paths trace their anchors here. Imported SVG
        // paths that preserve `d` paint through `svg_path` instead.
        p.points = anchors.iter().map(|a| [a.x as f32, a.y as f32]).collect();
        // Anchor-bounded path: when width/height weren't authored,
        // derive size from the handle-aware anchor bounds (cubic
        // extrema included — endpoint-only bbox under-sizes a path
        // whose handles bow past its anchors).
        if p.w == 0.0 && p.h == 0.0 && !anchors.is_empty() {
            let (_, _, w, h) = path_bounds_from_anchors(anchors, p.path_closed);
            p.w = w;
            p.h = h;
        }
    }
    p
}

pub(super) fn text_to_payload(n: &TextNode) -> NodePayload {
    let mut p = base_payload(&n.base, "text");
    // Flat string + styled segment runs + node italic/underline/
    // strikethrough — see `text_style.rs`.
    crate::text_style::apply_text_content(&mut p, n);
    assign_first_fill(&mut p, n.fill.as_deref());
    p.font_family = n.font_family.clone().unwrap_or_default();
    p.font_size = n.font_size.unwrap_or(0.0) as f32;
    p.font_weight = resolve_font_weight(n.font_weight.as_ref());
    // Keep paint on the same canonical multiplier used by layout measurement.
    // In particular, text carrying a pixel-like lineHeight must not measure
    // with the default and then paint with hundreds of pixels of leading.
    p.line_height = n.layout_line_height_multiplier().unwrap_or(0.0) as f32;
    p.letter_spacing = n.letter_spacing.unwrap_or(0.0) as f32;
    p.text_align = n
        .text_align
        .as_ref()
        .map(text_align_keyword)
        .unwrap_or("")
        .to_string();
    p.text_vertical_align = n
        .text_align_vertical
        .as_ref()
        .map(text_vertical_align_keyword)
        .unwrap_or("")
        .to_string();
    // Only wrap text when the schema explicitly authored
    // `textGrowth: fixed-width` (or fixed-width-and-height) —
    // matches canonical paint behaviour. Default-growth text was
    // authored against the TS app's measureText for a font we
    // may not have bundled; wrapping it would mis-break lines
    // the TS app shows on one line.
    use jian_ops_schema::node::TextGrowth;
    p.text_wrap = matches!(
        n.text_growth,
        Some(TextGrowth::FixedWidth) | Some(TextGrowth::FixedWidthHeight)
    );
    p
}

/// CSS-style numeric weight from the schema's `FontWeight` union.
/// Returns 0 when the field is absent so the renderer falls back
/// to its default — keeps the payload free of duplicate defaults.
///
/// Real `.op` files often emit `"fontWeight":"700"` as a JSON
/// STRING (the canonical untagged enum picks `Keyword(String)`
/// when the JSON type is a string, even with numeric contents).
/// Parse numeric keywords first, then fall back to lucide-style
/// named weights. Stays in sync with `jian_core::layout::resolve_weight`.
fn resolve_font_weight(w: Option<&FontWeight>) -> u16 {
    match w {
        Some(FontWeight::Number(n)) => *n as u16,
        Some(FontWeight::Keyword(s)) => {
            if let Ok(n) = s.parse::<u16>() {
                return n;
            }
            match s.as_str() {
                "bold" => 700,
                "semibold" | "semi-bold" | "demibold" => 600,
                "medium" => 500,
                "normal" | "regular" => 400,
                "light" => 300,
                "extralight" | "extra-light" | "ultralight" | "ultra-light" => 200,
                "thin" | "hairline" => 100,
                "black" | "heavy" => 900,
                "extrabold" | "extra-bold" | "ultrabold" | "ultra-bold" => 800,
                _ => 0,
            }
        }
        None => 0,
    }
}

fn text_align_keyword(value: &jian_ops_schema::node::TextAlign) -> &'static str {
    match value {
        jian_ops_schema::node::TextAlign::Left => "left",
        jian_ops_schema::node::TextAlign::Center => "center",
        jian_ops_schema::node::TextAlign::Right => "right",
        jian_ops_schema::node::TextAlign::Justify => "justify",
    }
}

fn text_vertical_align_keyword(value: &jian_ops_schema::node::TextAlignVertical) -> &'static str {
    match value {
        jian_ops_schema::node::TextAlignVertical::Top => "top",
        jian_ops_schema::node::TextAlignVertical::Middle => "middle",
        jian_ops_schema::node::TextAlignVertical::Bottom => "bottom",
    }
}

pub(super) fn image_to_payload(n: &ImageNode) -> NodePayload {
    let mut p = base_payload(&n.base, "rect");
    if let Some(CornerRadius::Uniform(r)) = &n.corner_radius {
        p.corner_radius = *r as f32;
    } else if let Some(CornerRadius::PerCorner(corners)) = &n.corner_radius {
        p.corner_radius = corners[0] as f32;
    }
    // Carry the image source so the canvas painter can decode +
    // draw the bitmap. `fill` stays at a neutral grey so the
    // placeholder reads correctly when the bytes fail to decode
    // (corrupt url / unsupported codec).
    p.image_src = Some(n.src.clone());
    p.video = n.video.clone();
    p.image_fit = n.object_fit.as_ref().map(image_node_fit_to_payload);
    p.image_adjustments = image_node_adjustments(n);
    p.fill = Some([0.85, 0.86, 0.88, 1.0]);
    p.name = if n.base.name.as_deref().unwrap_or("").is_empty() {
        format!("Image ({})", short_src(&n.src))
    } else {
        p.name
    };
    p
}

pub(super) fn icon_font_to_payload(n: &IconFontNode) -> NodePayload {
    // Route through a dedicated `icon_font` kind tag so the canvas
    // renderer can look up the lucide glyph by name. TS parity:
    // `node-renderer.ts::drawIconFont` resolves `iconFontName` via
    // `lookupIconByName` (icon-dictionary.ts) and paints with
    // scale-to-fit + stroke style.
    let mut p = base_payload(&n.base, "icon_font");
    p.text = Some(n.icon_font_name.clone());
    p.font_family = n
        .icon_font_family
        .clone()
        .unwrap_or_else(|| "lucide".to_string());
    assign_first_fill(&mut p, n.fill.as_deref());
    p
}

pub(super) fn empty_group(base: &PenNodeBase, kind: &str) -> NodePayload {
    let mut p = base_payload(base, kind);
    p.children = Vec::new();
    p
}
