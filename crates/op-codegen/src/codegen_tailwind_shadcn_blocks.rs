//! Multi-part shadcn/ui kit shapes (Tabs, Breadcrumb, Skeleton, Table)
//! for the `react-tailwind` target. Each emitter confirms the instance's
//! structure first and returns `false` without writing when it does not
//! fit, so the caller can fall back to generic markup.

use jian_ops_schema::node::PenNode;
use op_editor_core::pen_node_ext::PenNodeExt;

use super::jsx::{jsx_text, line};
use super::shadcn::{outer_classes, texts, visible_children};
use super::style::{self, Classes, Parent};
use super::Ctx;

/// Separator glyphs a drawn breadcrumb uses between items.
const BREADCRUMB_SEPARATORS: &[&str] = &["/", ">", "›", "»", "\\", "|", "·"];

/// `Tabs` from a row of tab frames, each holding one label.
pub(super) fn emit_tabs(
    ctx: &mut Ctx,
    out: &mut String,
    node: &PenNode,
    parent: Parent,
    depth: usize,
) -> bool {
    let tabs: Vec<(&PenNode, String)> = visible_children(node)
        .filter_map(|child| {
            let labels = texts(child);
            (labels.len() == 1 && child.children().is_some()).then(|| (child, labels[0].clone()))
        })
        .collect();
    if tabs.is_empty() || tabs.len() != visible_children(node).count() {
        return false;
    }
    let active = tabs
        .iter()
        .position(|(child, _)| {
            child
                .base()
                .name
                .as_deref()
                .is_some_and(|name| name.to_ascii_lowercase().contains("active"))
        })
        .unwrap_or(0);
    for name in ["Tabs", "TabsList", "TabsTrigger"] {
        ctx.ui(name);
    }
    let cls = outer_classes(node, parent, false).attr();
    line(
        out,
        depth,
        &format!("<Tabs defaultValue=\"tab-{}\"{cls}>", active + 1),
    );
    line(out, depth + 1, "<TabsList>");
    for (index, (_, label)) in tabs.iter().enumerate() {
        line(
            out,
            depth + 2,
            &format!(
                "<TabsTrigger value=\"tab-{}\">{}</TabsTrigger>",
                index + 1,
                jsx_text(label)
            ),
        );
    }
    line(out, depth + 1, "</TabsList>");
    line(out, depth, "</Tabs>");
    true
}

/// `Breadcrumb` from a row of texts separated by `/`-style glyphs.
pub(super) fn emit_breadcrumb(
    ctx: &mut Ctx,
    out: &mut String,
    node: &PenNode,
    parent: Parent,
    depth: usize,
) -> bool {
    if !visible_children(node).all(|child| matches!(child, PenNode::Text(_) | PenNode::IconFont(_)))
    {
        return false;
    }
    let items: Vec<String> = texts(node)
        .into_iter()
        .filter(|text| !BREADCRUMB_SEPARATORS.contains(&text.trim()))
        .collect();
    if items.len() < 2 {
        return false;
    }
    for name in [
        "Breadcrumb",
        "BreadcrumbItem",
        "BreadcrumbLink",
        "BreadcrumbList",
        "BreadcrumbPage",
        "BreadcrumbSeparator",
    ] {
        ctx.ui(name);
    }
    let cls = outer_classes(node, parent, false).attr();
    line(out, depth, &format!("<Breadcrumb{cls}>"));
    line(out, depth + 1, "<BreadcrumbList>");
    let last = items.len() - 1;
    for (index, item) in items.iter().enumerate() {
        line(out, depth + 2, "<BreadcrumbItem>");
        if index == last {
            line(
                out,
                depth + 3,
                &format!("<BreadcrumbPage>{}</BreadcrumbPage>", jsx_text(item)),
            );
        } else {
            line(
                out,
                depth + 3,
                &format!(
                    "<BreadcrumbLink href=\"#\">{}</BreadcrumbLink>",
                    jsx_text(item)
                ),
            );
        }
        line(out, depth + 2, "</BreadcrumbItem>");
        if index != last {
            line(out, depth + 2, "<BreadcrumbSeparator />");
        }
    }
    line(out, depth + 1, "</BreadcrumbList>");
    line(out, depth, "</Breadcrumb>");
    true
}

/// A stack of placeholder bars → one `Skeleton` per bar, keeping the
/// container's own layout classes.
pub(super) fn emit_skeleton(
    ctx: &mut Ctx,
    out: &mut String,
    node: &PenNode,
    parent: Parent,
    depth: usize,
) -> bool {
    let bars: Vec<&PenNode> = visible_children(node).collect();
    if bars.is_empty()
        || !bars.iter().all(|bar| {
            matches!(
                bar,
                PenNode::Rectangle(_) | PenNode::Ellipse(_) | PenNode::Frame(_)
            ) && bar.children().is_none_or(Vec::is_empty)
        })
    {
        return false;
    }
    ctx.ui("Skeleton");
    let mut wrapper = Classes::default();
    style::placement(node, parent, &mut wrapper);
    style::layout(ctx, node, &mut wrapper);
    style::size(ctx, node, parent, &mut wrapper);
    line(out, depth, &format!("<div{}>", wrapper.attr()));
    let child_parent = style::child_parent_kind(node);
    for bar in bars {
        let mut classes = Classes::default();
        style::placement(bar, child_parent, &mut classes);
        style::size(ctx, bar, child_parent, &mut classes);
        if matches!(bar, PenNode::Ellipse(_)) {
            classes.push("rounded-full");
        }
        line(out, depth + 1, &format!("<Skeleton{} />", classes.attr()));
    }
    line(out, depth, "</div>");
    true
}

/// A header row plus body rows of text cells → the shadcn `Table` family.
/// Rows without text (drawn separators) are dropped: `TableRow` borders
/// replace them.
pub(super) fn emit_table(
    ctx: &mut Ctx,
    out: &mut String,
    node: &PenNode,
    parent: Parent,
    depth: usize,
) -> bool {
    let mut rows: Vec<Vec<String>> = Vec::new();
    for child in visible_children(node) {
        let cells = texts(child);
        let is_text_row = visible_children(child).all(|cell| matches!(cell, PenNode::Text(_)));
        match (cells.is_empty(), is_text_row) {
            (true, _) => continue,
            (false, true) => rows.push(cells),
            (false, false) => return false,
        }
    }
    if rows.len() < 2 {
        return false;
    }
    for name in [
        "Table",
        "TableBody",
        "TableCell",
        "TableHead",
        "TableHeader",
        "TableRow",
    ] {
        ctx.ui(name);
    }
    let cls = outer_classes(node, parent, true).attr();
    line(out, depth, &format!("<Table{cls}>"));
    line(out, depth + 1, "<TableHeader>");
    line(out, depth + 2, "<TableRow>");
    for cell in &rows[0] {
        line(
            out,
            depth + 3,
            &format!("<TableHead>{}</TableHead>", jsx_text(cell)),
        );
    }
    line(out, depth + 2, "</TableRow>");
    line(out, depth + 1, "</TableHeader>");
    line(out, depth + 1, "<TableBody>");
    for row in &rows[1..] {
        line(out, depth + 2, "<TableRow>");
        for cell in row {
            line(
                out,
                depth + 3,
                &format!("<TableCell>{}</TableCell>", jsx_text(cell)),
            );
        }
        line(out, depth + 2, "</TableRow>");
    }
    line(out, depth + 1, "</TableBody>");
    line(out, depth, "</Table>");
    true
}
