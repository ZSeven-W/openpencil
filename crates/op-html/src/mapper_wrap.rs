//! `flex-wrap: wrap` emulation.
//!
//! `LayoutMode` has no wrapping mode, so a wrapping row is re-shaped at
//! import time into a vertical stack of synthetic "Wrap row" frames — the
//! same trick `mapper_grid` uses for implicit grid rows. Capacity is
//! estimated from the container's content width, the column gap and each
//! child's resolved main-axis size; when a child's size is indeterminate the
//! original single row survives with a warning.

use crate::import_warning::ImportWarning;
use jian_ops_schema::node::base::{NumberOrExpression, PenNodeBase};
use jian_ops_schema::node::container::{AlignItems, ContainerProps, LayoutMode, Padding};
use jian_ops_schema::node::{FrameNode, PenNode};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};

use crate::css::cascade::ComputedStyle;
use crate::length::{parse_length, LengthCtx};

use super::node_access::{node_base, numeric_width};
use super::MapCtx;

/// `mapper_offset::parent_accepts_alignment_row` suppresses the per-child
/// alignment row under any wrapping flex parent, and by the time this pass
/// runs the children are already mapped — a bail cannot restore them. Say so.
/// Chunk a wrapping flex row into synthetic rows, flipping the container to a
/// vertical stack when it succeeds.
///
/// Every bail path hands `children` straight back, UNTOUCHED. This pass runs
/// after `mapper_stack::layer_absolute_children`, so re-partitioning the list
/// would destroy the paint order that pass computed — negative-z absolutes sit
/// last in it, and Jian paints the first child on top.
pub(crate) fn apply_flex_wrap(
    context: &mut MapCtx<'_>,
    style: &ComputedStyle,
    container: &mut ContainerProps,
    mut children: Vec<PenNode>,
) -> Vec<PenNode> {
    let Some(mode) = style.get("flex-wrap").map(str::trim) else {
        return children;
    };
    if !matches!(mode, "wrap" | "wrap-reverse")
        || !matches!(style.get("display"), Some("flex" | "inline-flex"))
        || children.is_empty()
    {
        return children;
    }
    // Every path below this point must strip the private margin carriers the
    // children picked up on the way in, including the early returns.
    let margins = take_inline_margins(&mut children);
    if matches!(
        style.get("flex-direction"),
        Some("column" | "column-reverse")
    ) {
        context.warn_once(ImportWarning::FlexWrapColumnNotEmulated);
        return children;
    }
    if mode == "wrap-reverse" {
        context.warn_once(ImportWarning::FlexWrapReversePlain);
    }
    if !context.containing_width_is_definite {
        context.warn_once(ImportWarning::FlexWrapIndefiniteWidth);
        return children;
    }
    if style
        .get("align-content")
        .is_some_and(|value| !matches!(value.trim(), "" | "normal" | "stretch" | "flex-start"))
    {
        context.warn_once(ImportWarning::FlexAlignContentIgnored);
    }

    // Out-of-flow children keep their absolute placement and stay outside the
    // synthetic rows, exactly like the grid post-pass treats them.
    let flow_indices: Vec<usize> = children
        .iter()
        .enumerate()
        .filter(|(_, node)| !is_out_of_flow(node))
        .map(|(index, _)| index)
        .collect();
    let Some(first_flow) = flow_indices.first().copied() else {
        return children;
    };
    let Some(widths) = flow_indices
        .iter()
        .map(|index| {
            // Margins are what CSS reserves around the item; ignoring them
            // systematically under-wraps the `margin: 0 12px` gap idiom.
            numeric_width(&children[*index]).map(|width| width + margins[*index])
        })
        .collect::<Option<Vec<_>>>()
    else {
        context.warn_once(ImportWarning::FlexWrapIndeterminateChildren);
        return children;
    };

    let column_gap = axis_gap(style, context, "column-gap");
    let row_gap = axis_gap(style, context, "row-gap");
    // `containing_width` is this container's own resolved width. A
    // `FillContainer` container inherits the ancestor's width WITHOUT that
    // ancestor's padding subtracted — a pre-existing convention shared with
    // `mapper_grid`, so row capacity can be over-estimated under a padded
    // ancestor. Fixing it means threading a content width down the whole
    // descent, which is out of scope here.
    let available = (context.containing_width - horizontal_padding(container)).max(0.0);
    let rows = chunk_rows(&widths, available, column_gap);
    if rows.len() < 2 {
        // Nothing wrapped, so the container keeps its single horizontal row.
        // The auto-margin suppression is still in force for its children; a
        // warning here would fire on every non-wrapping `flex-wrap` container,
        // which is far too common to be useful.
        return children;
    }
    if context.node_count.saturating_add(rows.len()) > crate::MAX_OUTPUT_NODES {
        context.warn_once(ImportWarning::FlexWrapNodeLimit);
        return children;
    }

    let align_items = style
        .get("align-items")
        .and_then(AlignItems::from_css)
        // An inferred alignment (the all-`mx-auto` fast path, the `<button>`
        // control default) is just as load-bearing as an authored one.
        .or_else(|| container.align_items.clone())
        .or(Some(AlignItems::Start));
    let justify_content = container.justify_content.clone();
    let mut slots: Vec<Option<PenNode>> = children.into_iter().map(Some).collect();
    let flow: Vec<PenNode> = flow_indices
        .iter()
        .filter_map(|index| slots[*index].take())
        .collect();
    let mut row_children = build_rows(
        context,
        flow,
        &rows,
        &align_items,
        justify_content,
        column_gap,
    );
    // The container itself now stacks the synthetic rows vertically; its
    // former main-axis distribution moved onto each row.
    container.layout = Some(LayoutMode::Vertical);
    container.gap = (row_gap > 0.0).then_some(NumberOrExpression::Number(row_gap));
    container.justify_content = None;
    container.align_items = None;
    // Splice the rows in where the flow band started so the out-of-flow
    // siblings keep the paint order the stacking pass gave them.
    let mut output = Vec::with_capacity(slots.len());
    for (index, slot) in slots.into_iter().enumerate() {
        if index == first_flow {
            output.append(&mut row_children);
        }
        if let Some(node) = slot {
            output.push(node);
        }
    }
    output
}

fn is_out_of_flow(node: &PenNode) -> bool {
    let base = node_base(node);
    base.x.is_some() || base.y.is_some()
}

/// Private carrier key for a flow child's resolved inline margins, mirroring
/// the grid-placement carrier. Never reaches a serialized document:
/// `apply_flex_wrap` strips it from every child before returning, on every
/// path, and it is only ever written when the parent is a wrapping flex row —
/// exactly the case `apply_flex_wrap` handles.
const INLINE_MARGIN_KEY: &str = "__op_html_wrap_inline_margin";

/// True for the parent styles that make `record_inline_margins` worthwhile.
/// Must stay in lock-step with `apply_flex_wrap`'s own opening gates.
pub(super) fn is_wrapping_flex_row(style: &ComputedStyle) -> bool {
    matches!(style.get("display"), Some("flex" | "inline-flex"))
        && style
            .get("flex-wrap")
            .is_some_and(|value| matches!(value.trim(), "wrap" | "wrap-reverse"))
        && !matches!(
            style.get("flex-direction"),
            Some("column" | "column-reverse")
        )
}

/// Record `margin-left + margin-right` on the child so the parent's wrap pass
/// can count the space CSS reserves around it. Only the positive part counts:
/// negative margins are dropped elsewhere in the importer too.
pub(super) fn record_inline_margins(
    base: &mut PenNodeBase,
    style: &ComputedStyle,
    context: &MapCtx<'_>,
) {
    let inline: f64 = ["margin-left", "margin-right"]
        .into_iter()
        .filter_map(|name| {
            let value = style.get(name)?;
            if value.trim().eq_ignore_ascii_case("auto") {
                return None;
            }
            let length = parse_length(
                value,
                &LengthCtx {
                    font_size: style.font_size,
                    root_font_size: context.opts.base_font_size,
                    viewport_w: context.opts.viewport_width,
                    viewport_h: context.opts.viewport_height(),
                },
            )?;
            let value = length.resolve(context.containing_width);
            value.is_finite().then(|| value.max(0.0))
        })
        .sum();
    if inline <= 0.0 {
        return;
    }
    base.theme
        .get_or_insert_with(std::collections::BTreeMap::new)
        .insert(INLINE_MARGIN_KEY.to_string(), inline.to_string());
}

/// Strip every inline-margin carrier, returning the values positionally.
fn take_inline_margins(children: &mut [PenNode]) -> Vec<f64> {
    children
        .iter_mut()
        .map(|node| {
            let base = super::node_access::node_base_mut(node);
            let Some(theme) = base.theme.as_mut() else {
                return 0.0;
            };
            let margin = theme
                .remove(INLINE_MARGIN_KEY)
                .and_then(|encoded| encoded.parse::<f64>().ok())
                .filter(|value| value.is_finite() && *value >= 0.0)
                .unwrap_or(0.0);
            if theme.is_empty() {
                base.theme = None;
            }
            margin
        })
        .collect()
}

/// A margin wrapper makes the child's used width include this spacing, so the
/// older flex-wrap-only carrier would count it a second time.
pub(super) fn discard_recorded_inline_margin(node: &mut PenNode) {
    let base = super::node_access::node_base_mut(node);
    let Some(theme) = base.theme.as_mut() else {
        return;
    };
    theme.remove(INLINE_MARGIN_KEY);
    if theme.is_empty() {
        base.theme = None;
    }
}

/// Greedy line breaking: an item starts a new row as soon as it no longer
/// fits the remaining content width, matching the flex line algorithm for
/// items that neither grow nor shrink.
pub(super) fn chunk_rows(widths: &[f64], available: f64, gap: f64) -> Vec<usize> {
    let mut rows = Vec::new();
    let mut count = 0usize;
    let mut used = 0.0f64;
    for width in widths {
        let needed = if count == 0 {
            *width
        } else {
            used + gap + width
        };
        if count > 0 && needed > available + 0.5 {
            rows.push(count);
            count = 1;
            used = *width;
        } else {
            count += 1;
            used = needed;
        }
    }
    if count > 0 {
        rows.push(count);
    }
    rows
}

fn build_rows(
    context: &mut MapCtx<'_>,
    flow: Vec<PenNode>,
    rows: &[usize],
    align_items: &Option<AlignItems>,
    justify_content: Option<jian_ops_schema::node::container::JustifyContent>,
    column_gap: f64,
) -> Vec<PenNode> {
    let gap = (column_gap > 0.0).then_some(NumberOrExpression::Number(column_gap));
    let mut source = flow.into_iter();
    let mut output = Vec::with_capacity(rows.len());
    for count in rows {
        let row_children: Vec<_> = source.by_ref().take(*count).collect();
        if row_children.is_empty() {
            break;
        }
        output.push(PenNode::Frame(FrameNode {
            base: PenNodeBase {
                id: context.generate_id(),
                name: Some("Wrap row".to_string()),
                ..Default::default()
            },
            container: ContainerProps {
                width: Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)),
                height: Some(SizingBehavior::Keyword(SizingKeyword::FitContent)),
                layout: Some(LayoutMode::Horizontal),
                gap: gap.clone(),
                align_items: align_items.clone(),
                justify_content: justify_content.clone(),
                ..Default::default()
            },
            children: Some(row_children),
            image_search_query: None,
            reusable: None,
            slot: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
            screen: None,
            breakpoint: None,
        }));
        context.node_count += 1;
    }
    output
}

fn axis_gap(style: &ComputedStyle, context: &MapCtx<'_>, axis: &str) -> f64 {
    let Some(value) = style.get(axis).or_else(|| style.get("gap")) else {
        return 0.0;
    };
    if value.eq_ignore_ascii_case("normal") {
        return 0.0;
    }
    parse_length(
        value,
        &LengthCtx {
            font_size: style.font_size,
            root_font_size: context.opts.base_font_size,
            viewport_w: context.opts.viewport_width,
            viewport_h: context.opts.viewport_height(),
        },
    )
    .map(|length| length.resolve(context.containing_width))
    .filter(|value| value.is_finite() && *value >= 0.0)
    .unwrap_or(0.0)
}

/// Padding already resolved onto the container, so the row capacity uses the
/// content box rather than the border box. The canonical axis order is
/// `XY = [vertical, horizontal]` and `LtrB = [top, right, bottom, left]`
/// (jian-core `layout/resolve.rs`), so the inline axis is the SECOND slot of
/// `XY` and the second + fourth of `LtrB`.
fn horizontal_padding(container: &ContainerProps) -> f64 {
    match &container.padding {
        Some(Padding::Uniform(value)) => value * 2.0,
        Some(Padding::XY([_, horizontal])) => horizontal * 2.0,
        Some(Padding::LtrB([_, right, _, left])) => left + right,
        Some(Padding::Expression(_)) | None => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greedy_chunking_respects_gaps_and_capacity() {
        // Three 300px cards with a 24px gap fit twice in 700px.
        assert_eq!(chunk_rows(&[300.0, 300.0, 300.0], 700.0, 24.0), [2, 1]);
        // Exactly-fitting rows do not spill.
        assert_eq!(chunk_rows(&[300.0, 300.0], 624.0, 24.0), [2]);
        // A single oversized item still occupies its own row.
        assert_eq!(chunk_rows(&[900.0, 100.0], 500.0, 0.0), [1, 1]);
        assert!(chunk_rows(&[], 500.0, 0.0).is_empty());
    }

    /// B1. The inline axis is the second slot of `XY` and the second + fourth
    /// of `LtrB`; both arms used to read the block axis instead.
    #[test]
    fn horizontal_padding_reads_the_inline_axis() {
        let with = |padding| ContainerProps {
            padding: Some(padding),
            ..Default::default()
        };
        assert_eq!(horizontal_padding(&with(Padding::Uniform(8.0))), 16.0);
        // XY is [vertical, horizontal].
        assert_eq!(horizontal_padding(&with(Padding::XY([100.0, 20.0]))), 40.0);
        // LtrB is [top, right, bottom, left].
        assert_eq!(
            horizontal_padding(&with(Padding::LtrB([1.0, 2.0, 3.0, 4.0]))),
            6.0
        );
        assert_eq!(horizontal_padding(&ContainerProps::default()), 0.0);
    }
}

#[cfg(test)]
mod integration_tests {
    use crate::{import_html, HtmlImportOptions};
    use jian_ops_schema::node::container::LayoutMode;
    use jian_ops_schema::node::PenNode;

    fn wrap_container(html: &str) -> (Option<LayoutMode>, usize, Vec<String>) {
        let result = import_html(html, &HtmlImportOptions::default());
        let PenNode::Frame(root) = &result.nodes[0] else {
            panic!("expected a root frame")
        };
        let PenNode::Frame(flex) = &root.children.as_ref().unwrap()[0] else {
            panic!("expected the flex container")
        };
        (
            flex.container.layout.clone(),
            flex.children.as_ref().map(Vec::len).unwrap_or(0),
            result.warnings,
        )
    }

    /// C2. Margins are part of what CSS reserves around a flex item, so
    /// ignoring them systematically under-wraps the `margin: 0 12px` idiom.
    #[test]
    fn child_inline_margins_count_towards_the_row_capacity() {
        let card = "<div style='width:220px;height:10px;margin:0 20px'>x</div>";
        let (layout, rows, _) = wrap_container(&format!(
            "<div style='display:flex;flex-wrap:wrap;width:700px'>{card}{card}{card}</div>"
        ));
        // 3 × (220 + 40) = 780 > 700, so the row breaks even though the bare
        // widths (3 × 220 = 660) would have fitted.
        assert_eq!(layout, Some(LayoutMode::Vertical));
        assert_eq!(rows, 2);

        // Without margins the same three cards stay on one row.
        let bare = "<div style='width:220px;height:10px'>x</div>";
        let (layout, _, _) = wrap_container(&format!(
            "<div style='display:flex;flex-wrap:wrap;width:700px'>{bare}{bare}{bare}</div>"
        ));
        assert_eq!(layout, Some(LayoutMode::Horizontal));

        // The synthetic Margin frame already includes 40px around each card.
        // Keeping the old private carrier as well would measure 900px and
        // incorrectly wrap this 780px row in an 800px container.
        let (layout, rows, _) = wrap_container(&format!(
            "<div style='display:flex;flex-wrap:wrap;width:800px'>{card}{card}{card}</div>"
        ));
        assert_eq!(layout, Some(LayoutMode::Horizontal));
        assert_eq!(rows, 3);
    }

    /// C1. A container with no definite width has no capacity to measure
    /// against; the percentages `map_sizing` bakes for its children come from
    /// a stale ancestor width, so chunking them is worse than not wrapping.
    #[test]
    fn an_indefinite_container_width_bails_with_a_warning() {
        let card = "<div style='width:300px;height:10px'>x</div>";
        let (layout, _, warnings) = wrap_container(&format!(
            "<div style='display:flex'>\
               <div style='display:flex;flex-wrap:wrap'>{card}{card}{card}{card}{card}</div>\
             </div>"
        ));
        assert_eq!(layout, Some(LayoutMode::Horizontal), "{warnings:?}");
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("no definite width")),
            "{warnings:?}"
        );
    }

    /// N1 + N2. An inferred `align-items` is as load-bearing as an authored
    /// one, and a dropped `align-content` is a degradation like any other.
    #[test]
    fn rows_keep_inferred_alignment_and_report_dropped_align_content() {
        let card = "<div style='width:300px;height:10px'>x</div>";
        let result = import_html(
            &format!(
                "<button style='display:flex;flex-wrap:wrap;width:700px;\
                 align-content:space-between'>{card}{card}{card}</button>"
            ),
            &HtmlImportOptions::default(),
        );
        let PenNode::Frame(root) = &result.nodes[0] else {
            panic!()
        };
        let PenNode::Frame(button) = &root.children.as_ref().unwrap()[0] else {
            panic!()
        };
        let PenNode::Frame(row) = &button.children.as_ref().unwrap()[0] else {
            panic!("expected a wrap row")
        };
        assert_eq!(row.base.name.as_deref(), Some("Wrap row"));
        assert_eq!(
            row.container.align_items,
            Some(jian_ops_schema::node::container::AlignItems::Center),
            "the <button> control default must survive onto the rows"
        );
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.contains("align-content")),
            "{:?}",
            result.warnings
        );
    }
}
