//! JSX tree walker for the `react-tailwind` target: one element per
//! visible node, Tailwind classes from [`super::style`], shadcn/ui usages
//! for recognized kit instances and first-class widgets.

use jian_ops_schema::node::text::TextContent;
use jian_ops_schema::node::{ImageFitMode, PenNode};
use jian_ops_schema::style::{PenFill, StyledTextSegment};
use op_editor_core::pen_node_ext::PenNodeExt;

use super::style::{self, Classes, Parent};
use super::{shadcn, widgets, Ctx};
use crate::fmt_num;

/// Append one indented line (two spaces per depth level).
pub(super) fn line(out: &mut String, depth: usize, text: &str) {
    for _ in 0..depth {
        out.push_str("  ");
    }
    out.push_str(text);
    out.push('\n');
}

/// Escape text for a JSX child position. Braces would open an
/// expression and `<`/`>` a tag, so all four are entity-encoded;
/// newlines become `<br />`.
pub(super) fn jsx_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '{' => out.push_str("&#123;"),
            '}' => out.push_str("&#125;"),
            '\n' => out.push_str("<br />"),
            '\r' => {}
            c => out.push(c),
        }
    }
    out
}

/// Escape text for a double-quoted JSX attribute value.
pub(super) fn jsx_attr(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\n' | '\r' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// Emit one node (and its subtree) at `depth`.
pub(super) fn emit_node(
    ctx: &mut Ctx,
    out: &mut String,
    node: &PenNode,
    parent: Parent,
    depth: usize,
) {
    if node.base().visible == Some(false) {
        return;
    }
    if let Some(kit) = shadcn::detect(node) {
        if shadcn::emit(ctx, out, node, kit, parent, depth) {
            return;
        }
    }
    if widgets::emit(ctx, out, node, parent, depth) {
        return;
    }
    match node {
        PenNode::Text(_) => emit_text(ctx, out, node, parent, depth),
        PenNode::Image(_) => emit_image(ctx, out, node, parent, depth),
        PenNode::IconFont(_) => emit_icon(ctx, out, node, parent, depth),
        PenNode::Path(_) => emit_path(ctx, out, node, parent, depth),
        PenNode::Line(_) => emit_line(ctx, out, node, parent, depth),
        PenNode::Ref(reference) => line(
            out,
            depth,
            &format!(
                "{{/* unresolved component instance: {} */}}",
                jsx_attr(&reference.target).replace("*/", "* /")
            ),
        ),
        _ => emit_box(ctx, out, node, parent, depth),
    }
}

/// Emit a node's children with the layout kind it imposes.
pub(super) fn emit_children(ctx: &mut Ctx, out: &mut String, node: &PenNode, depth: usize) {
    let parent = style::child_parent_kind(node);
    for child in node.children().map(Vec::as_slice).unwrap_or_default() {
        emit_node(ctx, out, child, parent, depth);
    }
}

fn box_classes(ctx: &mut Ctx, node: &PenNode, parent: Parent) -> Classes {
    let mut classes = Classes::default();
    style::placement(node, parent, &mut classes);
    style::layout(ctx, node, &mut classes);
    style::size(ctx, node, parent, &mut classes);
    style::visuals(ctx, node, &mut classes);
    classes
}

/// Frames, groups, rectangles, ellipses, polygons → `<div>`.
fn emit_box(ctx: &mut Ctx, out: &mut String, node: &PenNode, parent: Parent, depth: usize) {
    let classes = box_classes(ctx, node, parent);
    let has_children = node
        .children()
        .is_some_and(|children| children.iter().any(|c| c.base().visible != Some(false)));
    if !has_children {
        line(out, depth, &format!("<div{} />", classes.attr()));
        return;
    }
    line(out, depth, &format!("<div{}>", classes.attr()));
    emit_children(ctx, out, node, depth + 1);
    line(out, depth, "</div>");
}

fn emit_text(ctx: &mut Ctx, out: &mut String, node: &PenNode, parent: Parent, depth: usize) {
    let PenNode::Text(text) = node else {
        return;
    };
    let mut classes = Classes::default();
    style::placement(node, parent, &mut classes);
    style::size(ctx, node, parent, &mut classes);
    style::typography(ctx, text, node, &mut classes);
    style::visuals(ctx, node, &mut classes);
    let body = match &text.content {
        TextContent::Plain(s) => jsx_text(s),
        TextContent::Styled(segments) => styled_segments(ctx, segments),
    };
    line(out, depth, &format!("<p{}>{body}</p>", classes.attr()));
}

fn styled_segments(ctx: &mut Ctx, segments: &[StyledTextSegment]) -> String {
    let mut body = String::new();
    for segment in segments {
        let mut classes = Classes::default();
        if let Some(family) = segment.font_family.as_deref() {
            classes.push_opt(style::font_family(ctx, family));
        }
        if let Some(size) = segment.font_size.filter(|s| s.is_finite() && *s > 0.0) {
            classes.push(super::scale::font_size(f64::from(size)));
        }
        if let Some(weight) = segment.font_weight {
            classes.push_opt(super::scale::font_weight(weight));
        }
        if segment.font_style == Some(jian_ops_schema::style::FontStyleKind::Italic) {
            classes.push("italic");
        }
        if segment.underline == Some(true) {
            classes.push("underline");
        }
        if segment.strikethrough == Some(true) {
            classes.push("line-through");
        }
        if let Some(fill) = segment.fill.as_deref() {
            classes.push_opt(style::color_class(ctx, "text", fill, None));
        }
        let text = jsx_text(&segment.text);
        match (&segment.href, classes.is_empty()) {
            (Some(href), _) => body.push_str(&format!(
                "<a href=\"{}\"{}>{text}</a>",
                jsx_attr(href),
                classes.attr()
            )),
            (None, true) => body.push_str(&text),
            (None, false) => body.push_str(&format!("<span{}>{text}</span>", classes.attr())),
        }
    }
    body
}

fn emit_image(ctx: &mut Ctx, out: &mut String, node: &PenNode, parent: Parent, depth: usize) {
    let PenNode::Image(image) = node else {
        return;
    };
    let mut classes = Classes::default();
    style::placement(node, parent, &mut classes);
    style::size(ctx, node, parent, &mut classes);
    classes.push(match image.object_fit.as_ref() {
        Some(ImageFitMode::Fit) => "object-contain",
        Some(ImageFitMode::Crop) => "object-cover",
        Some(ImageFitMode::Tile) => "object-none",
        Some(ImageFitMode::Fill) | None => "object-fill",
    });
    style::visuals(ctx, node, &mut classes);
    let src: &str = image.src.as_ref();
    line(
        out,
        depth,
        &format!(
            "<img src=\"{}\" alt=\"{}\"{} />",
            jsx_attr(src),
            jsx_attr(image.base.name.as_deref().unwrap_or_default()),
            classes.attr()
        ),
    );
}

/// lucide-react component name for a kebab-case icon name
/// (`arrow-right` → `ArrowRightIcon`). `None` for names that cannot
/// form a JS identifier.
pub(super) fn lucide_component(name: &str) -> Option<String> {
    let name = name.trim();
    let name = name.strip_prefix("lucide:").unwrap_or(name);
    let mut out = String::new();
    for part in name.split(['-', '_', ' ', ':']).filter(|p| !p.is_empty()) {
        if !part.chars().all(|c| c.is_ascii_alphanumeric()) {
            return None;
        }
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.push_str(chars.as_str());
        }
    }
    if !out.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }
    out.push_str("Icon");
    Some(out)
}

fn emit_icon(ctx: &mut Ctx, out: &mut String, node: &PenNode, parent: Parent, depth: usize) {
    let PenNode::IconFont(icon) = node else {
        return;
    };
    let mut classes = Classes::default();
    style::placement(node, parent, &mut classes);
    style::size(ctx, node, parent, &mut classes);
    classes.push_opt(style::foreground(ctx, node));
    style::visuals(ctx, node, &mut classes);
    let family = icon.icon_font_family.as_deref().unwrap_or("lucide");
    let lucide = family.to_ascii_lowercase().contains("lucide");
    match lucide_component(&icon.icon_font_name).filter(|_| lucide) {
        Some(component) => {
            ctx.import("lucide-react", &component);
            line(out, depth, &format!("<{component}{} />", classes.attr()));
        }
        None => {
            // Ligature icon fonts (Material Symbols …) render their name.
            classes.push(format!(
                "font-['{}']",
                family.replace([' ', '\'', '"'], "_")
            ));
            line(
                out,
                depth,
                &format!(
                    "<span aria-hidden=\"true\"{}>{}</span>",
                    classes.attr(),
                    jsx_text(&icon.icon_font_name)
                ),
            );
        }
    }
}

fn emit_path(ctx: &mut Ctx, out: &mut String, node: &PenNode, parent: Parent, depth: usize) {
    let PenNode::Path(path) = node else {
        return;
    };
    let Some(d) = path.d.as_deref().filter(|d| !d.trim().is_empty()) else {
        emit_box(ctx, out, node, parent, depth);
        return;
    };
    let mut classes = Classes::default();
    style::placement(node, parent, &mut classes);
    style::size(ctx, node, parent, &mut classes);
    let (w, h) = (
        node.width_px().unwrap_or(24.0),
        node.height_px().unwrap_or(24.0),
    );
    let has_fill =
        style::fills(node).is_some_and(|f| f.iter().any(|f| matches!(f, PenFill::Solid(_))));
    let stroke = style::stroke(node).and_then(|s| {
        let width = match &s.thickness {
            jian_ops_schema::style::StrokeThickness::Uniform(w) => *w,
            _ => 1.0,
        };
        let color = s.fill.as_ref()?.iter().find_map(|f| match f {
            PenFill::Solid(body) => Some(body.color.clone()),
            _ => None,
        })?;
        Some((color, width))
    });
    let mut attrs = String::new();
    if has_fill {
        classes.push_opt(style::foreground(ctx, node));
        attrs.push_str(" fill=\"currentColor\"");
    } else {
        attrs.push_str(" fill=\"none\"");
    }
    if let Some((color, width)) = stroke {
        if !has_fill {
            classes.push_opt(style::color_class(ctx, "text", &color, None));
        }
        attrs.push_str(&format!(
            " stroke=\"currentColor\" strokeWidth={{{}}}",
            fmt_num(super::scale::round_px(f64::from(width)))
        ));
    }
    style::visuals(ctx, node, &mut classes);
    line(
        out,
        depth,
        &format!(
            "<svg viewBox=\"0 0 {} {}\"{attrs}{}><path d=\"{}\" /></svg>",
            fmt_num(super::scale::round_px(w)),
            fmt_num(super::scale::round_px(h)),
            classes.attr(),
            jsx_attr(d)
        ),
    );
}

fn emit_line(ctx: &mut Ctx, out: &mut String, node: &PenNode, parent: Parent, depth: usize) {
    let PenNode::Line(segment) = node else {
        return;
    };
    let base = node.base();
    let dx = segment.x2.unwrap_or(base.x.unwrap_or(0.0)) - base.x.unwrap_or(0.0);
    let dy = segment.y2.unwrap_or(base.y.unwrap_or(0.0)) - base.y.unwrap_or(0.0);
    let mut classes = Classes::default();
    style::placement(node, parent, &mut classes);
    let vertical = dy.abs() > dx.abs();
    let length = dx.hypot(dy);
    let width = style::stroke(node)
        .map(|s| match &s.thickness {
            jian_ops_schema::style::StrokeThickness::Uniform(w) => f64::from(*w),
            _ => 1.0,
        })
        .unwrap_or(1.0);
    let side = if vertical { "l" } else { "t" };
    if vertical {
        classes.push(super::scale::spacing("h", length));
    } else {
        classes.push(super::scale::spacing("w", length));
    }
    classes.push_opt(super::scale::border_width(side, width));
    if let Some(color) = style::stroke(node)
        .and_then(|s| s.fill.as_ref())
        .and_then(|fills| {
            fills.iter().find_map(|f| match f {
                PenFill::Solid(body) => Some(body.color.clone()),
                _ => None,
            })
        })
    {
        classes.push_opt(style::color_class(ctx, "border", &color, None));
    }
    if !vertical && dy.abs() > f64::EPSILON {
        classes.push_opt(super::scale::rotate(dy.atan2(dx).to_degrees()));
        classes.push("origin-top-left");
    }
    line(out, depth, &format!("<div{} />", classes.attr()));
}
