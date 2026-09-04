//! CSS margin boxes represented as transparent outer frames.
//!
//! Jian has padding but no margin. Applying CSS margins to the authored
//! frame's padding changes its paint box, so every represented non-zero
//! margin lives on a synthetic parent instead. A short-lived theme carrier
//! lets the containing block collapse adjoining block-axis margins before
//! grid/flex-wrap reshape the child list; the carrier never reaches output.

use crate::css::cascade::ComputedStyle;
use crate::import_warning::ImportWarning;
use crate::length::{parse_length, LengthCtx};
use jian_ops_schema::node::base::PenNodeBase;
use jian_ops_schema::node::container::{ContainerProps, LayoutMode, Padding};
use jian_ops_schema::node::{FrameNode, PenNode};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};

use super::node_access::{node_base, node_base_mut, node_height, node_width};
use super::MapCtx;

#[path = "mapper_margin_collapse.rs"]
mod collapse;
use collapse::{choose_component_edge, edge_index, DisjointSets};

#[path = "mapper_margin_root.rs"]
mod root;
pub(crate) use root::apply_root_margins;

const MARGIN_HINT_KEY: &str = "__op_html_margin";

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct CollapsedMargins {
    leading: CollapseSet,
    trailing: CollapseSet,
    through: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct CollapseSet {
    positive: f64,
    negative: f64,
}

impl CollapseSet {
    fn from_value(value: f64) -> Self {
        Self {
            positive: value.max(0.0),
            negative: value.min(0.0),
        }
    }

    fn merge(self, other: Self) -> Self {
        Self {
            positive: self.positive.max(other.positive),
            negative: self.negative.min(other.negative),
        }
    }

    fn value(self) -> f64 {
        self.positive + self.negative
    }

    fn is_empty(self) -> bool {
        self.positive <= f64::EPSILON && self.negative >= -f64::EPSILON
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Edges {
    top: f64,
    right: f64,
    bottom: f64,
    left: f64,
}

impl Edges {
    fn any(self) -> bool {
        [self.top, self.right, self.bottom, self.left]
            .into_iter()
            .any(|value| value.abs() > f64::EPSILON)
    }

    fn has_negative(self) -> bool {
        [self.top, self.right, self.bottom, self.left]
            .into_iter()
            .any(|value| value < 0.0)
    }

    fn non_negative(self) -> Self {
        Self {
            top: self.top.max(0.0),
            right: self.right.max(0.0),
            bottom: self.bottom.max(0.0),
            left: self.left.max(0.0),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Hint {
    edges: Edges,
    top: CollapseSet,
    bottom: CollapseSet,
    block: bool,
    empty: bool,
}

impl Hint {
    fn encode(self) -> String {
        format!(
            "{},{},{},{},{},{},{},{}",
            self.top.positive,
            self.top.negative,
            self.edges.right,
            self.bottom.positive,
            self.bottom.negative,
            self.edges.left,
            u8::from(self.block),
            u8::from(self.empty)
        )
    }

    fn decode(value: &str) -> Option<Self> {
        let mut parts = value.split(',');
        let top = CollapseSet {
            positive: parts.next()?.parse().ok()?,
            negative: parts.next()?.parse().ok()?,
        };
        let right = parts.next()?.parse().ok()?;
        let bottom = CollapseSet {
            positive: parts.next()?.parse().ok()?,
            negative: parts.next()?.parse().ok()?,
        };
        let left = parts.next()?.parse().ok()?;
        let block = parts.next()? == "1";
        let empty = parts.next()? == "1";
        let hint = Self {
            edges: Edges {
                top: top.value(),
                right,
                bottom: bottom.value(),
                left,
            },
            top,
            bottom,
            block,
            empty,
        };
        (parts.next().is_none()
            && [
                top.positive,
                top.negative,
                right,
                bottom.positive,
                bottom.negative,
                left,
            ]
            .into_iter()
            .all(f64::is_finite))
        .then_some(hint)
    }
}

/// Finish a containing block's direct margin boxes before grid/flex-wrap
/// introduce synthetic rows. Flex/grid items keep every margin independently;
/// ordinary block siblings collapse adjoining vertical margins.
pub(super) fn finish_children(
    context: &mut MapCtx<'_>,
    style: &ComputedStyle,
    parent_style: Option<&ComputedStyle>,
    children: &mut [PenNode],
) -> CollapsedMargins {
    let mut hints: Vec<Option<Hint>> = children.iter_mut().map(take_hint).collect();
    if !has_collapsing_block_flow(style) {
        return CollapsedMargins::default();
    }

    let mut sets = DisjointSets::new(children.len().saturating_mul(2));
    let mut active = vec![false; children.len().saturating_mul(2)];
    let mut previous: Option<usize> = None;
    let mut all_flow_is_empty = true;
    for index in 0..children.len() {
        if is_out_of_flow(&children[index]) {
            continue;
        }
        let Some(hint) = hints[index] else {
            previous = None;
            all_flow_is_empty = false;
            continue;
        };
        if !hint.block {
            previous = None;
            all_flow_is_empty = false;
            continue;
        }
        let top = edge_index(index, true);
        let bottom = edge_index(index, false);
        active[top] = true;
        active[bottom] = true;
        if hint.empty {
            sets.join(top, bottom);
        } else {
            all_flow_is_empty = false;
        }
        if let Some(previous_index) = previous {
            sets.join(edge_index(previous_index, false), top);
        }
        previous = Some(index);
    }

    let mut aggregate = vec![CollapseSet::default(); active.len()];
    for (index, hint) in hints.iter().enumerate() {
        let Some(hint) = hint else {
            continue;
        };
        for (top, value) in [(true, hint.top), (false, hint.bottom)] {
            let edge = edge_index(index, top);
            if active[edge] {
                let root = sets.root(edge);
                aggregate[root] = aggregate[root].merge(value);
            }
        }
    }

    let first = first_flow_block(children, &hints);
    let last = last_flow_block(children, &hints);
    let first_root = first.map(|index| sets.root(edge_index(index, true)));
    let last_root = last.map(|index| sets.root(edge_index(index, false)));
    let through_edges = parent_edges_collapse(style, parent_style, context);
    let mut suppressed = vec![false; active.len()];
    let mut collapsed = CollapsedMargins {
        through: through_edges.0 && through_edges.1 && all_flow_is_empty && first_root == last_root,
        ..Default::default()
    };
    if through_edges.0 {
        if let Some(root) = first_root {
            collapsed.leading = aggregate[root];
            suppressed[root] = true;
        }
    }
    if through_edges.1 {
        if let Some(root) = last_root {
            if Some(root) != first_root || !through_edges.0 {
                collapsed.trailing = aggregate[root];
            }
            suppressed[root] = true;
        }
    }

    let originals = hints.clone();
    for hint in hints.iter_mut().flatten() {
        hint.top = CollapseSet::default();
        hint.bottom = CollapseSet::default();
        hint.edges.top = 0.0;
        hint.edges.bottom = 0.0;
    }
    for root in 0..aggregate.len() {
        let value = aggregate[root];
        if value.is_empty() || suppressed[root] || sets.root(root) != root {
            continue;
        }
        if let Some((index, top)) =
            choose_component_edge(&mut sets, &active, &originals, root, value)
        {
            if let Some(hint) = hints[index].as_mut() {
                if top {
                    hint.top = value;
                    hint.edges.top = value.value();
                } else {
                    hint.bottom = value;
                    hint.edges.bottom = value.value();
                }
            }
        }
    }

    for (node, hint) in children.iter_mut().zip(hints) {
        if let Some(hint) = hint {
            reconfigure_margin_wrapper(node, hint.edges);
        }
    }
    collapsed
}

/// Wrap one in-flow box in an outer transparent margin frame. Eligible child
/// margins that collapsed through this element join its own top/bottom edge.
pub(super) fn wrap_margins(
    context: &mut MapCtx<'_>,
    node: PenNode,
    tag: &str,
    style: &ComputedStyle,
    parent_style: Option<&ComputedStyle>,
    collapsed: CollapsedMargins,
) -> PenNode {
    let mut edges = resolve_edges(style, context);
    let non_atomic_inline = is_non_atomic_inline(tag, style, parent_style);
    if non_atomic_inline {
        // Vertical margins on a non-replaced inline box do not participate in
        // line-box height. Inline-start/end still create visible spacing.
        edges.top = 0.0;
        edges.bottom = 0.0;
    }
    let block = is_block_level(style) && !non_atomic_inline;
    let mut top = CollapseSet::from_value(edges.top).merge(collapsed.leading);
    let mut bottom = CollapseSet::from_value(edges.bottom).merge(collapsed.trailing);
    let empty = block && collapsed.through;
    if empty {
        top = top.merge(bottom);
        bottom = CollapseSet::default();
    }
    edges.top = top.value();
    edges.bottom = bottom.value();

    if is_internal_table_display(style) {
        // CSS margins do not apply to internal table boxes, regardless of the
        // source tag. The outer `table` and `table-caption` boxes still do.
        return node;
    }
    if matches!(style.get("position"), Some("absolute" | "fixed")) {
        return node;
    }

    let mut represented = edges;
    if edges.has_negative() && !negative_geometry_is_definite(&node, edges) {
        context.warn_once(ImportWarning::NegativeMarginsIgnored);
        represented = edges.non_negative();
        if edges.top < 0.0 {
            top = CollapseSet::from_value(represented.top);
        }
        if edges.bottom < 0.0 {
            bottom = CollapseSet::from_value(represented.bottom);
        }
    }

    let mut node = if represented.any() {
        if context.node_count.saturating_add(1) > crate::MAX_OUTPUT_NODES {
            context.warn_once(ImportWarning::NodeLimitMapping);
            return node;
        }
        wrap(context, node, represented)
    } else {
        node
    };
    // A top-level compatibility call to `map_element` has no containing box
    // that could consume this private hint.
    if parent_style.is_some() && block {
        attach_hint(&mut node, represented, top, bottom, block, empty);
    }
    node
}

fn resolve_edges(style: &ComputedStyle, context: &MapCtx<'_>) -> Edges {
    let value = |name: &str| {
        let raw = style.get(name)?;
        if raw.trim().eq_ignore_ascii_case("auto") {
            return None;
        }
        let length = parse_length(
            raw,
            &LengthCtx {
                font_size: style.font_size,
                root_font_size: context.opts.base_font_size,
                viewport_w: context.opts.viewport_width,
                viewport_h: context.opts.viewport_height(),
            },
        )?;
        let value = length.resolve(context.containing_width);
        value.is_finite().then_some(value)
    };
    Edges {
        top: value("margin-top").unwrap_or(0.0),
        right: value("margin-right").unwrap_or(0.0),
        bottom: value("margin-bottom").unwrap_or(0.0),
        left: value("margin-left").unwrap_or(0.0),
    }
}

fn wrap(context: &mut MapCtx<'_>, mut node: PenNode, edges: Edges) -> PenNode {
    context.node_count += 1;
    // The wrapper's numeric width already includes its inline margins. The
    // flex-wrap capacity pass must not add its legacy margin carrier again.
    super::wrap::discard_recorded_inline_margin(&mut node);
    let (explain, theme) = {
        let base = node_base_mut(&mut node);
        (base.explain.take(), base.theme.take())
    };
    let mut frame = FrameNode {
        base: PenNodeBase {
            id: context.generate_id(),
            name: Some("Margin".to_string()),
            explain,
            theme,
            ..Default::default()
        },
        container: ContainerProps::default(),
        children: Some(vec![node]),
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
    };
    configure_frame(&mut frame, edges);
    PenNode::Frame(frame)
}

fn configure_frame(frame: &mut FrameNode, edges: Edges) {
    let Some(child) = frame
        .children
        .as_mut()
        .and_then(|children| children.first_mut())
    else {
        return;
    };
    let child_width = node_width(child).cloned();
    let child_height = node_height(child).cloned();
    let numeric = match (&child_width, &child_height) {
        (Some(SizingBehavior::Number(width)), Some(SizingBehavior::Number(height)))
            if width.is_finite() && height.is_finite() =>
        {
            Some((*width, *height))
        }
        _ => None,
    };

    frame.container = if edges.has_negative() {
        let Some((width, height)) = numeric else {
            return;
        };
        let base = node_base_mut(child);
        base.x = Some(edges.left);
        base.y = Some(edges.top);
        ContainerProps {
            width: Some(SizingBehavior::Number(
                (width + edges.left + edges.right).max(0.0),
            )),
            height: Some(SizingBehavior::Number(
                (height + edges.top + edges.bottom).max(0.0),
            )),
            layout: Some(LayoutMode::Vertical),
            ..Default::default()
        }
    } else {
        let base = node_base_mut(child);
        base.x = None;
        base.y = None;
        ContainerProps {
            width: wrapper_axis(child_width.as_ref(), edges.left + edges.right),
            height: wrapper_axis(child_height.as_ref(), edges.top + edges.bottom),
            layout: Some(LayoutMode::Vertical),
            padding: padding_from(edges),
            ..Default::default()
        }
    };
}

fn wrapper_axis(axis: Option<&SizingBehavior>, extra: f64) -> Option<SizingBehavior> {
    match axis {
        Some(SizingBehavior::Number(value)) => Some(SizingBehavior::Number(value + extra)),
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)) => {
            Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
        }
        _ => Some(SizingBehavior::Keyword(SizingKeyword::FitContent)),
    }
}

fn negative_geometry_is_definite(node: &PenNode, edges: Edges) -> bool {
    let (Some(SizingBehavior::Number(width)), Some(SizingBehavior::Number(height))) =
        (node_width(node), node_height(node))
    else {
        return false;
    };
    width.is_finite()
        && height.is_finite()
        && width + edges.left + edges.right >= 0.0
        && height + edges.top + edges.bottom >= 0.0
}

fn attach_hint(
    node: &mut PenNode,
    edges: Edges,
    top: CollapseSet,
    bottom: CollapseSet,
    block: bool,
    empty: bool,
) {
    node_base_mut(node)
        .theme
        .get_or_insert_with(Default::default)
        .insert(
            MARGIN_HINT_KEY.to_string(),
            Hint {
                edges,
                top,
                bottom,
                block,
                empty,
            }
            .encode(),
        );
}

fn take_hint(node: &mut PenNode) -> Option<Hint> {
    let base = node_base_mut(node);
    let encoded = base.theme.as_mut()?.remove(MARGIN_HINT_KEY)?;
    if base.theme.as_ref().is_some_and(|theme| theme.is_empty()) {
        base.theme = None;
    }
    Hint::decode(&encoded)
}

fn reconfigure_margin_wrapper(node: &mut PenNode, edges: Edges) {
    let PenNode::Frame(frame) = node else {
        return;
    };
    if frame.base.name.as_deref() == Some("Margin") {
        configure_frame(frame, edges);
    }
}

fn has_collapsing_block_flow(style: &ComputedStyle) -> bool {
    !matches!(
        style.get("display"),
        Some(
            "flex"
                | "inline-flex"
                | "grid"
                | "inline-grid"
                | "table"
                | "inline-table"
                | "table-row"
        )
    )
}

fn is_block_level(style: &ComputedStyle) -> bool {
    !super::layout_heuristics::is_inline_level(style)
        && !matches!(
            style.get("display"),
            Some(
                "inline-table"
                    | "table-row"
                    | "table-cell"
                    | "table-row-group"
                    | "table-header-group"
                    | "table-footer-group"
            )
        )
}

fn is_internal_table_display(style: &ComputedStyle) -> bool {
    matches!(
        style.get("display"),
        Some(
            "table-row"
                | "table-cell"
                | "table-row-group"
                | "table-header-group"
                | "table-footer-group"
                | "table-column"
                | "table-column-group"
        )
    )
}

fn is_non_atomic_inline(
    tag: &str,
    style: &ComputedStyle,
    parent_style: Option<&ComputedStyle>,
) -> bool {
    if matches!(style.get("position"), Some("absolute" | "fixed"))
        || parent_style.is_some_and(|parent| {
            matches!(
                parent.get("display"),
                Some("flex" | "inline-flex" | "grid" | "inline-grid")
            )
        })
        || matches!(
            tag,
            "button" | "img" | "input" | "select" | "svg" | "textarea"
        )
    {
        return false;
    }
    match style.get("display").map(str::trim) {
        Some("inline" | "inline flow" | "ruby") => true,
        None => crate::text::is_inline_tag(tag) || matches!(tag, "::before" | "::after"),
        _ => false,
    }
}

fn parent_edges_collapse(
    style: &ComputedStyle,
    parent_style: Option<&ComputedStyle>,
    context: &MapCtx<'_>,
) -> (bool, bool) {
    if matches!(
        style.get("display"),
        Some(
            "flow-root"
                | "inline-block"
                | "flex"
                | "inline-flex"
                | "grid"
                | "inline-grid"
                | "table"
                | "inline-table"
                | "table-cell"
                | "table-caption"
        )
    ) || matches!(style.get("position"), Some("absolute" | "fixed"))
        || style
            .get("float")
            .is_some_and(|value| !value.eq_ignore_ascii_case("none"))
        || ["overflow", "overflow-x", "overflow-y"]
            .into_iter()
            .filter_map(|name| style.get(name))
            .any(|value| !matches!(value.trim(), "visible" | "clip"))
        || parent_style.is_some_and(|parent| {
            matches!(
                parent.get("display"),
                Some("flex" | "inline-flex" | "grid" | "inline-grid")
            )
        })
    {
        return (false, false);
    }
    let border = super::box_model::border_widths(style, context);
    let padding = padding_style_edges(style, context);
    let top = border[0] <= f64::EPSILON && padding.top <= f64::EPSILON;
    let bottom = border[2] <= f64::EPSILON
        && padding.bottom <= f64::EPSILON
        && style
            .get("height")
            .is_none_or(|value| value.eq_ignore_ascii_case("auto"))
        && style
            .get("min-height")
            .and_then(|value| resolve(value, style, context))
            .is_none_or(|value| value <= f64::EPSILON);
    (top, bottom)
}

fn padding_style_edges(style: &ComputedStyle, context: &MapCtx<'_>) -> Edges {
    let value = |name: &str| {
        style
            .get(name)
            .and_then(|value| resolve(value, style, context))
            .unwrap_or(0.0)
            .max(0.0)
    };
    Edges {
        top: value("padding-top"),
        right: value("padding-right"),
        bottom: value("padding-bottom"),
        left: value("padding-left"),
    }
}

fn resolve(value: &str, style: &ComputedStyle, context: &MapCtx<'_>) -> Option<f64> {
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
    value.is_finite().then_some(value)
}

fn first_flow_block(children: &[PenNode], hints: &[Option<Hint>]) -> Option<usize> {
    for (index, child) in children.iter().enumerate() {
        if is_out_of_flow(child) {
            continue;
        }
        return hints[index].is_some_and(|hint| hint.block).then_some(index);
    }
    None
}

fn last_flow_block(children: &[PenNode], hints: &[Option<Hint>]) -> Option<usize> {
    for (index, child) in children.iter().enumerate().rev() {
        if is_out_of_flow(child) {
            continue;
        }
        return hints[index].is_some_and(|hint| hint.block).then_some(index);
    }
    None
}

fn is_out_of_flow(node: &PenNode) -> bool {
    let base = node_base(node);
    base.x.is_some() || base.y.is_some()
}

fn padding_from(edges: Edges) -> Option<Padding> {
    let values = [edges.top, edges.right, edges.bottom, edges.left];
    if values.iter().all(|value| value.abs() <= f64::EPSILON) {
        None
    } else if values.iter().all(|value| *value == values[0]) {
        Some(Padding::Uniform(values[0]))
    } else {
        Some(Padding::LtrB(values))
    }
}

fn padding_edges(padding: Option<&Padding>) -> Edges {
    match padding {
        Some(Padding::Uniform(value)) => Edges {
            top: *value,
            right: *value,
            bottom: *value,
            left: *value,
        },
        Some(Padding::XY([vertical, horizontal])) => Edges {
            top: *vertical,
            right: *horizontal,
            bottom: *vertical,
            left: *horizontal,
        },
        Some(Padding::LtrB([top, right, bottom, left])) => Edges {
            top: *top,
            right: *right,
            bottom: *bottom,
            left: *left,
        },
        Some(Padding::Expression(_)) | None => Edges::default(),
    }
}

#[cfg(test)]
pub(crate) fn authored_node(mut node: &PenNode) -> &PenNode {
    while let PenNode::Frame(frame) = node {
        if frame.base.name.as_deref() != Some("Margin") {
            break;
        }
        let Some(child) = frame
            .children
            .as_deref()
            .and_then(|children| children.first())
        else {
            break;
        };
        node = child;
    }
    node
}
