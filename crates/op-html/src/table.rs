//! Heuristic `<table>` layout.
//!
//! The user-agent sheet gives `table` / `tr` / `td` / `th` their CSS display
//! values and flattens `thead` / `tbody` / `tfoot` with `display: contents`, so
//! the ordinary element mapper already produces a vertical stack of horizontal
//! rows. What it cannot produce is a shared column grid: every cell would hug
//! its own content and the columns would not line up between rows.
//!
//! This module closes that gap the same way `mapper_grid` does for CSS grid:
//! each cell records its `colspan` on its own node while it is mapped, and the
//! table's post-pass reads those spans back, derives one column width vector
//! (from `<colgroup>` / `<col>` when authored, otherwise an even split) and
//! pins a concrete width on every cell.
//!
//! It is deliberately not a table layout algorithm: content-driven column
//! sizing, `rowspan` and collapsed-border edge cases are approximated and
//! reported rather than implemented.

use crate::import_warning::ImportWarning;
use std::collections::BTreeMap;

use jian_ops_schema::node::base::PenNodeBase;
use jian_ops_schema::node::container::AlignItems;
use jian_ops_schema::node::PenNode;

use crate::css::cascade::ComputedStyle;
use crate::dom::{DomElement, DomNode};
use crate::length::{parse_length, LengthCtx};

use super::grid_place::{cell_width, Cell};
use super::node_access::{node_base, node_base_mut, set_node_width};
use super::MapCtx;

/// Private carrier key for a cell's `colspan`. Always stripped by
/// [`finish_table`], which runs for every `<table>` element regardless of its
/// computed `display`, so the hint can never reach a serialized document.
const SPAN_KEY: &str = "__op_html_table_span";

/// Upper bound on synthesized columns. A pathological `colspan` must not turn
/// into a multi-thousand-entry width vector.
const MAX_TABLE_COLUMNS: usize = 64;

/// CSS `border-spacing` expressed as the frame gap the schema does have.
///
/// The table box contributes the vertical spacing as its row gap and each row
/// contributes the horizontal spacing as its column gap. `border-collapse:
/// collapse` removes both. Spacing around the table's outer edge has no
/// representation and is dropped.
pub(super) fn spacing_gap(style: &ComputedStyle, context: &MapCtx<'_>) -> Option<f64> {
    let vertical = match style.get("display")?.trim() {
        "table" | "inline-table" => true,
        "table-row" => false,
        _ => return None,
    };
    axis_spacing(style, context, vertical)
}

/// One axis of `border-spacing`. The table box needs the horizontal value too
/// — it is the gap the column widths have to make room for — even though its
/// own frame gap is the vertical one.
fn axis_spacing(style: &ComputedStyle, context: &MapCtx<'_>, vertical: bool) -> Option<f64> {
    if style
        .get("border-collapse")
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("collapse"))
    {
        return None;
    }
    let value = style.get("border-spacing")?;
    let mut parts = value.split_whitespace();
    let horizontal = parts.next()?;
    let axis = if vertical {
        parts.next().unwrap_or(horizontal)
    } else {
        horizontal
    };
    length_px(axis, style, context, 0.0).filter(|value| *value > 0.0)
}

/// Stash a cell's `colspan` on its own node for the table post-pass.
///
/// Only recorded for a cell that really is inside a `<table>` subtree, which is
/// exactly the condition under which [`finish_table`] is guaranteed to run and
/// strip the carrier again.
pub(super) fn record_cell_span(
    base: &mut PenNodeBase,
    element: &DomElement,
    path: &[&DomElement],
    style: &ComputedStyle,
    parent_style: Option<&ComputedStyle>,
    context: &mut MapCtx<'_>,
) {
    let in_row = parent_style.is_some_and(|parent| parent.get("display") == Some("table-row"));
    // `colspan` only means anything for a box that really is a table cell. An
    // authored `td{display:block}` takes the element out of the table box tree,
    // so its span must not consume the grid's tracks either.
    let is_cell = style
        .get("display")
        .is_none_or(|value| value.trim() == "table-cell");
    if !in_row || !is_cell || !path.iter().rev().skip(1).any(|node| node.tag == "table") {
        return;
    }
    if element
        .attr("rowspan")
        .is_some_and(|value| value.trim() != "1" && !value.trim().is_empty())
    {
        context.warn_once(ImportWarning::TableRowspanIgnored);
    }
    let Some(span) = element
        .attr("colspan")
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|span| *span > 1)
    else {
        return;
    };
    base.theme
        .get_or_insert_with(BTreeMap::new)
        .insert(SPAN_KEY.to_string(), span.to_string());
}

/// Table post-pass: give every cell the width of the column band it occupies.
///
/// Runs for every `<table>` element — including one the author re-styled to a
/// non-table `display` — so the private span carriers are always removed.
///
/// `width_is_definite` reports whether `context.containing_width` — which the
/// caller has already narrowed to the table's own resolved width — really is
/// that width, or the parent containing block standing in for a shrink-to-fit
/// the importer never solves.
pub(super) fn finish_table(
    context: &mut MapCtx<'_>,
    path: &[&DomElement],
    style: &ComputedStyle,
    width_is_definite: bool,
    mut children: Vec<PenNode>,
) -> Vec<PenNode> {
    let Some(element) = path.last() else {
        return children;
    };
    if element.tag != "table" {
        return children;
    }
    let row_indices: Vec<usize> = children
        .iter()
        .enumerate()
        .filter(|(_, node)| layout_subject_name(node) == Some("tr"))
        .map(|(index, _)| index)
        .collect();
    let mut row_spans = Vec::with_capacity(row_indices.len());
    for index in &row_indices {
        row_spans.push(take_cell_spans(child_nodes_mut(layout_subject_mut(
            &mut children[*index],
        ))));
    }
    // Safety net: a row that was re-parented (an offset or auto-margin wrapper)
    // is not matched above, so sweep whatever carriers are left.
    strip_span_carriers(&mut children);
    if !matches!(style.get("display"), Some("table" | "inline-table")) {
        return children;
    }
    move_caption_to_bottom(style, &mut children);
    if row_spans.is_empty() {
        // Author rules can take the row groups out of `display: contents`
        // (`tbody{display:block}`), which re-parents every `<tr>` under a frame
        // of its own. The rows are still imported, but they no longer share one
        // column grid, so their cells hug their own content instead.
        if has_row_descendant(element) {
            context.warn_once(ImportWarning::TableRowGroupsUnflattened);
        }
        return children;
    }
    if !width_is_definite {
        // Real table layout measures the columns against the table's own used
        // width, which is a shrink-to-fit of the cells' content when nothing
        // pins it. Nothing in this pipeline resolves that, so the columns are
        // measured against the containing block instead — right for the common
        // full-bleed table, too wide for a table nested in a narrower cell.
        context.warn_once(ImportWarning::TableIndefiniteWidthApproximated);
    }

    let columns = row_spans
        .iter()
        .map(|spans| spans.iter().sum::<usize>())
        .max()
        .unwrap_or(0)
        .max(authored_columns(element))
        .clamp(1, MAX_TABLE_COLUMNS);
    let gap = axis_spacing(style, context, false).unwrap_or(0.0);
    let widths = column_widths(context, style, element, columns, gap);
    for (index, spans) in row_indices.iter().zip(&row_spans) {
        apply_row(&mut children[*index], spans, &widths, gap);
    }
    children
}

/// `caption-side: bottom` places the caption box after the table's rows. The
/// caption maps to an ordinary child frame, so honouring it is a reorder.
fn move_caption_to_bottom(style: &ComputedStyle, children: &mut Vec<PenNode>) {
    if !style
        .get("caption-side")
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("bottom"))
    {
        return;
    }
    let mut captions: Vec<PenNode> = Vec::new();
    let mut index = 0;
    while index < children.len() {
        if layout_subject_name(&children[index]) == Some("caption") {
            captions.push(children.remove(index));
        } else {
            index += 1;
        }
    }
    children.append(&mut captions);
}

/// Whether the table still contains a `<tr>` somewhere below it, used to tell
/// an empty table apart from one whose rows were re-parented by author rules.
fn has_row_descendant(element: &DomElement) -> bool {
    element.children.iter().any(|child| match child {
        DomNode::Element(child) => child.tag == "tr" || has_row_descendant(child),
        _ => false,
    })
}

/// Pin cell widths and give the row the stretch alignment a table row implies.
fn apply_row(row: &mut PenNode, spans: &[usize], widths: &[f64], gap: f64) {
    let row = layout_subject_mut(row);
    if let PenNode::Frame(frame) = row {
        if frame.container.align_items.is_none() {
            frame.container.align_items = Some(AlignItems::Stretch);
        }
    }
    let mut start = 0usize;
    for (cell, span) in child_nodes_mut(row).iter_mut().zip(spans) {
        let span = (*span).clamp(1, widths.len().saturating_sub(start).max(1));
        if let Some(width) = cell_width(widths, Cell { start, span }, gap) {
            set_node_width(cell, width);
        }
        start = (start + span).min(widths.len());
    }
}

fn layout_subject_name(node: &PenNode) -> Option<&str> {
    let node = layout_subject(node);
    node_base(node).name.as_deref()
}

fn layout_subject(node: &PenNode) -> &PenNode {
    let PenNode::Frame(frame) = node else {
        return node;
    };
    if !matches!(
        frame.base.name.as_deref(),
        Some("Margin" | "Offset" | "Auto margin")
    ) {
        return node;
    }
    frame
        .children
        .as_deref()
        .and_then(|children| children.first())
        .map(layout_subject)
        .unwrap_or(node)
}

fn layout_subject_mut(node: &mut PenNode) -> &mut PenNode {
    let descend = matches!(
        node,
        PenNode::Frame(frame)
            if matches!(frame.base.name.as_deref(), Some("Margin" | "Offset" | "Auto margin"))
                && frame.children.as_ref().is_some_and(|children| !children.is_empty())
    );
    if descend {
        let PenNode::Frame(frame) = node else {
            unreachable!()
        };
        return layout_subject_mut(&mut frame.children.as_mut().expect("checked above")[0]);
    }
    node
}

/// One width per column, in CSS px.
///
/// `<col width>` / `<col style="width:…">` pin individual tracks; whatever is
/// left over is split evenly across the unpinned ones. Without a `<colgroup>`
/// every column gets an equal share, which is what an even pricing or
/// comparison table wants.
///
/// Two deliberate approximations live here, both bounded by the same missing
/// piece — a real used-width solve for the table box:
///
/// * The table's own percentage `padding` resolves against
///   `context.containing_width`, which at this point is the TABLE's width, not
///   the CSS containing block a percentage padding is defined against. They
///   agree for the common `width: 100%` table and diverge for a pinned one.
/// * `border-spacing` around the table's outer edge has no representation (see
///   [`spacing_gap`]) and is therefore left out of the column budget as well,
///   so a spaced table's columns are each `2 * spacing / columns` too wide.
fn column_widths(
    context: &MapCtx<'_>,
    style: &ComputedStyle,
    element: &DomElement,
    columns: usize,
    gap: f64,
) -> Vec<f64> {
    let padding: f64 = ["padding-left", "padding-right"]
        .into_iter()
        .filter_map(|name| length_px(style.get(name)?, style, context, context.containing_width))
        .sum();
    let content =
        (context.containing_width - padding - gap * columns.saturating_sub(1) as f64).max(0.0);
    let authored = authored_track_widths(context, style, element, columns, content);
    let pinned: f64 = authored.iter().flatten().sum();
    let free_tracks = authored.iter().filter(|width| width.is_none()).count();
    let share = if free_tracks > 0 {
        (content - pinned).max(0.0) / free_tracks as f64
    } else {
        0.0
    };
    authored
        .into_iter()
        .map(|width| width.unwrap_or(share))
        .collect()
}

/// `<col>` elements in document order, `span` expanded, capped at `columns`.
///
/// A percentage `<col width>` resolves against `content`, i.e. the width the
/// gaps have already been taken out of. CSS resolves it against the table's
/// content box before the spacing is deducted, so `<col width="50%">` on a
/// spaced table comes out marginally narrow; the difference is the layer's
/// share of `border-spacing` and disappears entirely under
/// `border-collapse: collapse`.
fn authored_track_widths(
    context: &MapCtx<'_>,
    style: &ComputedStyle,
    element: &DomElement,
    columns: usize,
    content: f64,
) -> Vec<Option<f64>> {
    let mut widths = vec![None; columns];
    let mut index = 0usize;
    for column in columns_of(element) {
        let width = column
            .attr("width")
            .map(str::to_string)
            .or_else(|| inline_width(column))
            .and_then(|value| length_px(&numeric_as_px(&value), style, context, content))
            .filter(|value| value.is_finite() && *value >= 0.0);
        let span = column
            .attr("span")
            .and_then(|value| value.trim().parse::<usize>().ok())
            .unwrap_or(1)
            .clamp(1, MAX_TABLE_COLUMNS);
        for _ in 0..span {
            if index >= columns {
                return widths;
            }
            widths[index] = width;
            index += 1;
        }
    }
    widths
}

fn authored_columns(element: &DomElement) -> usize {
    columns_of(element)
        .map(|column| {
            column
                .attr("span")
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(1)
                .clamp(1, MAX_TABLE_COLUMNS)
        })
        .sum()
}

/// `<col>` children of the table, whether or not they sit in a `<colgroup>`.
fn columns_of(element: &DomElement) -> impl Iterator<Item = &DomElement> {
    element.children.iter().flat_map(|child| {
        let DomNode::Element(child) = child else {
            return Vec::new();
        };
        match child.tag.as_str() {
            "col" => vec![child],
            "colgroup" => child
                .children
                .iter()
                .filter_map(|inner| match inner {
                    DomNode::Element(inner) if inner.tag == "col" => Some(inner),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    })
}

/// The legacy `width` attribute is a bare number of pixels; CSS lengths and
/// percentages are accepted verbatim.
fn numeric_as_px(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.parse::<f64>().is_ok() {
        format!("{trimmed}px")
    } else {
        trimmed.to_string()
    }
}

fn inline_width(element: &DomElement) -> Option<String> {
    let declarations = crate::css::declarations::parse_declarations(element.attr("style")?);
    declarations
        .iter()
        .rev()
        .find(|declaration| declaration.name == "width")
        .map(|declaration| declaration.value.clone())
}

fn take_cell_spans(cells: &mut [PenNode]) -> Vec<usize> {
    cells
        .iter_mut()
        .map(|cell| take_span(node_base_mut(cell)).unwrap_or(1))
        .collect()
}

fn take_span(base: &mut PenNodeBase) -> Option<usize> {
    let theme = base.theme.as_mut()?;
    let span = theme.remove(SPAN_KEY)?.parse::<usize>().ok();
    if theme.is_empty() {
        base.theme = None;
    }
    span.filter(|span| *span > 0)
}

fn strip_span_carriers(nodes: &mut [PenNode]) {
    for node in nodes {
        take_span(node_base_mut(node));
        strip_span_carriers(child_nodes_mut(node));
    }
}

fn child_nodes_mut(node: &mut PenNode) -> &mut [PenNode] {
    match node {
        PenNode::Frame(frame) => frame.children.as_deref_mut().unwrap_or_default(),
        PenNode::Group(group) => group.children.as_deref_mut().unwrap_or_default(),
        PenNode::Rectangle(rectangle) => rectangle.children.as_deref_mut().unwrap_or_default(),
        PenNode::Ref(reference) => reference.children.as_deref_mut().unwrap_or_default(),
        PenNode::Tabs(tabs) => tabs.children.as_deref_mut().unwrap_or_default(),
        _ => &mut [],
    }
}

fn length_px(
    value: &str,
    style: &ComputedStyle,
    context: &MapCtx<'_>,
    reference: f64,
) -> Option<f64> {
    let length = parse_length(
        value,
        &LengthCtx {
            font_size: style.font_size,
            root_font_size: context.opts.base_font_size,
            viewport_w: context.opts.viewport_width,
            viewport_h: context.opts.viewport_height(),
        },
    )?;
    let resolved = length.resolve(reference);
    resolved.is_finite().then_some(resolved)
}

#[cfg(test)]
#[path = "table_tests.rs"]
mod tests;
