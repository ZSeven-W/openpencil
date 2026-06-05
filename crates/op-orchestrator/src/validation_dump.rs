//! Validation dump — node tree text dump + active-page node count.
//!
//! Faithful port of `apps/web/src/services/ai/design-validation.ts:69-131`
//! (`buildNodeTreeDump` + `countNodesInActivePage`).
//!
//! Pure functions over `&EditorState`.  Callers (C1/C2) land later.

#![allow(dead_code)]

use jian_ops_schema::node::text::TextContent;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::SizingBehavior;
use jian_ops_schema::style::{PenFill, PenStroke, StrokeThickness};
use op_editor_core::EditorState;

// ── active-page root nodes ────────────────────────────────────────────────────

/// Returns the root-level `PenNode` slice for the active page.
///
/// Mirrors TS `getActivePageChildren(doc, activePageId)`.
/// Falls back to `doc.children` when `pages` is absent or the active
/// index is out of range (single-page fallback).
fn active_page_roots(state: &EditorState) -> &[PenNode] {
    if let Some(pages) = state.doc.pages.as_ref() {
        let idx = state.ui.active_page_index;
        if let Some(page) = pages.get(idx) {
            return &page.children;
        }
    }
    &state.doc.children
}

// ── count_nodes_in_active_page ────────────────────────────────────────────────

/// Total node count of the active page — used by the §4.6 30-node gate
/// to decide whether to enter the vision-LLM validation loop.
///
/// Port of `countNodesInActivePage` in `design-validation.ts:54-67`.
pub(crate) fn count_nodes_in_active_page(state: &EditorState) -> usize {
    let roots = active_page_roots(state);
    let mut count = 0usize;
    for root in roots {
        walk_count(root, &mut count);
    }
    count
}

fn walk_count(node: &PenNode, count: &mut usize) {
    *count += 1;
    if let Some(children) = node_children(node) {
        for child in children {
            walk_count(child, count);
        }
    }
}

// ── build_node_tree_dump ──────────────────────────────────────────────────────

/// Build an indented text dump of the active page's node tree.
///
/// Each line represents one node; properties are appended in the same
/// order as the TS source.  Used as user-message context for the vision
/// LLM in `validate_design_screenshot`.
///
/// Port of `buildNodeTreeDump` in `design-validation.ts:69-131`.
pub(crate) fn build_node_tree_dump(state: &EditorState) -> String {
    let roots = active_page_roots(state);
    let mut lines: Vec<String> = Vec::new();
    for root in roots {
        walk_dump(root, 0, &mut lines);
    }
    lines.join("\n")
}

fn walk_dump(node: &PenNode, depth: usize, lines: &mut Vec<String>) {
    let indent = "  ".repeat(depth);
    let mut props: Vec<String> = Vec::new();

    // id + type — always first
    props.push(format!("id=\"{}\"", node_id(node)));
    props.push(format!("type={}", node_type_name(node)));

    // name
    if let Some(name) = node_name(node) {
        if !name.is_empty() {
            props.push(format!("name=\"{name}\""));
        }
    }

    // width / height — as JSON.stringify would render: number or quoted string
    if let Some(w) = node_width(node) {
        props.push(format!("w={}", format_sizing(w)));
    }
    if let Some(h) = node_height(node) {
        props.push(format!("h={}", format_sizing(h)));
    }

    // layout (container only)
    if let Some(layout) = node_layout(node) {
        props.push(format!("layout={}", format_layout(layout)));
    }

    // gap (container only)
    if let Some(gap) = node_gap(node) {
        props.push(format!("gap={}", format_number_or_expr(gap)));
    }

    // padding (container only)
    if let Some(pad) = node_padding(node) {
        props.push(format!("pad={}", format_padding(pad)));
    }

    // justifyContent
    if let Some(jc) = node_justify_content(node) {
        props.push(format!("justify={}", format_justify(jc)));
    }

    // alignItems
    if let Some(ai) = node_align_items(node) {
        props.push(format!("align={}", format_align(ai)));
    }

    // cornerRadius
    if let Some(cr) = node_corner_radius(node) {
        props.push(format!("cr={}", format_corner_radius(cr)));
    }

    // opacity (when not 1)
    if let Some(op) = node_opacity(node) {
        // TS: `if opacity != null && opacity !== 1` — skip default 1.0
        if (op - 1.0).abs() > 1e-9 {
            props.push(format!("opacity={op}"));
        }
    }

    // fill — first solid fill colour
    if let Some(fill_color) = first_fill_color(node) {
        props.push(format!("fill=\"{fill_color}\""));
    }

    // stroke
    if let Some(stroke) = node_stroke(node) {
        if let Some(stroke_color) = first_stroke_color(stroke) {
            let stroke_w = stroke_thickness_value(&stroke.thickness);
            props.push(format!("stroke=\"{stroke_color}\" strokeW={stroke_w}"));
        }
    }

    // text-specific fields
    if let PenNode::Text(t) = node {
        if let Some(fs) = t.font_size {
            props.push(format!("fontSize={fs}"));
        }
        if let Some(fw) = &t.font_weight {
            props.push(format!("fontWeight={}", format_font_weight(fw)));
        }
        if let Some(lh) = t.line_height {
            props.push(format!("lineHeight={lh}"));
        }
        if let Some(tg) = &t.text_growth {
            props.push(format!("textGrowth={}", format_text_growth(tg)));
        }
        if let Some(ta) = &t.text_align {
            props.push(format!("textAlign={}", format_text_align(ta)));
        }
        // text content — first 30 chars
        let content = extract_text_content(&t.content);
        props.push(format!("text=\"{}\"", &content[..content.len().min(30)]));
    }

    lines.push(format!("{indent}{}", props.join(" ")));

    // recurse into children
    if let Some(children) = node_children(node) {
        for child in children {
            walk_dump(child, depth + 1, lines);
        }
    }
}

// ── per-variant field accessors ───────────────────────────────────────────────

fn node_id(node: &PenNode) -> &str {
    match node {
        PenNode::Frame(n) => &n.base.id,
        PenNode::Group(n) => &n.base.id,
        PenNode::Rectangle(n) => &n.base.id,
        PenNode::Ellipse(n) => &n.base.id,
        PenNode::Line(n) => &n.base.id,
        PenNode::Polygon(n) => &n.base.id,
        PenNode::Path(n) => &n.base.id,
        PenNode::Text(n) => &n.base.id,
        PenNode::TextInput(n) => &n.base.id,
        PenNode::Image(n) => &n.base.id,
        PenNode::IconFont(n) => &n.base.id,
        PenNode::Ref(n) => &n.base.id,
    }
}

fn node_name(node: &PenNode) -> Option<&str> {
    let base = match node {
        PenNode::Frame(n) => &n.base,
        PenNode::Group(n) => &n.base,
        PenNode::Rectangle(n) => &n.base,
        PenNode::Ellipse(n) => &n.base,
        PenNode::Line(n) => &n.base,
        PenNode::Polygon(n) => &n.base,
        PenNode::Path(n) => &n.base,
        PenNode::Text(n) => &n.base,
        PenNode::TextInput(n) => &n.base,
        PenNode::Image(n) => &n.base,
        PenNode::IconFont(n) => &n.base,
        PenNode::Ref(n) => &n.base,
    };
    base.name.as_deref()
}

/// node_type_name returns the JSON tag string (same as TS `node.type`).
fn node_type_name(node: &PenNode) -> &'static str {
    match node {
        PenNode::Frame(_) => "frame",
        PenNode::Group(_) => "group",
        PenNode::Rectangle(_) => "rectangle",
        PenNode::Ellipse(_) => "ellipse",
        PenNode::Line(_) => "line",
        PenNode::Polygon(_) => "polygon",
        PenNode::Path(_) => "path",
        PenNode::Text(_) => "text",
        PenNode::TextInput(_) => "text_input",
        PenNode::Image(_) => "image",
        PenNode::IconFont(_) => "icon_font",
        PenNode::Ref(_) => "ref",
    }
}

fn node_width(node: &PenNode) -> Option<&SizingBehavior> {
    match node {
        PenNode::Frame(n) => n.container.width.as_ref(),
        PenNode::Group(n) => n.container.width.as_ref(),
        PenNode::Rectangle(n) => n.container.width.as_ref(),
        PenNode::Text(n) => n.width.as_ref(),
        PenNode::TextInput(n) => n.width.as_ref(),
        _ => None,
    }
}

fn node_height(node: &PenNode) -> Option<&SizingBehavior> {
    match node {
        PenNode::Frame(n) => n.container.height.as_ref(),
        PenNode::Group(n) => n.container.height.as_ref(),
        PenNode::Rectangle(n) => n.container.height.as_ref(),
        PenNode::Text(n) => n.height.as_ref(),
        PenNode::TextInput(n) => n.height.as_ref(),
        _ => None,
    }
}

fn node_layout(node: &PenNode) -> Option<&jian_ops_schema::node::LayoutMode> {
    match node {
        PenNode::Frame(n) => n.container.layout.as_ref(),
        PenNode::Group(n) => n.container.layout.as_ref(),
        PenNode::Rectangle(n) => n.container.layout.as_ref(),
        _ => None,
    }
}

fn node_gap(node: &PenNode) -> Option<&jian_ops_schema::node::base::NumberOrExpression> {
    match node {
        PenNode::Frame(n) => n.container.gap.as_ref(),
        PenNode::Group(n) => n.container.gap.as_ref(),
        PenNode::Rectangle(n) => n.container.gap.as_ref(),
        _ => None,
    }
}

fn node_padding(node: &PenNode) -> Option<&jian_ops_schema::node::container::Padding> {
    match node {
        PenNode::Frame(n) => n.container.padding.as_ref(),
        PenNode::Group(n) => n.container.padding.as_ref(),
        PenNode::Rectangle(n) => n.container.padding.as_ref(),
        _ => None,
    }
}

fn node_justify_content(node: &PenNode) -> Option<&jian_ops_schema::node::JustifyContent> {
    match node {
        PenNode::Frame(n) => n.container.justify_content.as_ref(),
        PenNode::Group(n) => n.container.justify_content.as_ref(),
        PenNode::Rectangle(n) => n.container.justify_content.as_ref(),
        _ => None,
    }
}

fn node_align_items(node: &PenNode) -> Option<&jian_ops_schema::node::AlignItems> {
    match node {
        PenNode::Frame(n) => n.container.align_items.as_ref(),
        PenNode::Group(n) => n.container.align_items.as_ref(),
        PenNode::Rectangle(n) => n.container.align_items.as_ref(),
        _ => None,
    }
}

fn node_corner_radius(node: &PenNode) -> Option<&jian_ops_schema::node::container::CornerRadius> {
    match node {
        PenNode::Frame(n) => n.container.corner_radius.as_ref(),
        PenNode::Group(n) => n.container.corner_radius.as_ref(),
        PenNode::Rectangle(n) => n.container.corner_radius.as_ref(),
        _ => None,
    }
}

fn node_opacity(node: &PenNode) -> Option<f64> {
    use jian_ops_schema::node::base::NumberOrExpression;
    let base = match node {
        PenNode::Frame(n) => &n.base,
        PenNode::Group(n) => &n.base,
        PenNode::Rectangle(n) => &n.base,
        PenNode::Ellipse(n) => &n.base,
        PenNode::Line(n) => &n.base,
        PenNode::Polygon(n) => &n.base,
        PenNode::Path(n) => &n.base,
        PenNode::Text(n) => &n.base,
        PenNode::TextInput(n) => &n.base,
        PenNode::Image(n) => &n.base,
        PenNode::IconFont(n) => &n.base,
        PenNode::Ref(n) => &n.base,
    };
    match &base.opacity {
        Some(NumberOrExpression::Number(v)) => Some(*v),
        _ => None,
    }
}

fn node_fill(node: &PenNode) -> Option<&Vec<PenFill>> {
    match node {
        PenNode::Frame(n) => n.container.fill.as_ref(),
        PenNode::Group(n) => n.container.fill.as_ref(),
        PenNode::Rectangle(n) => n.container.fill.as_ref(),
        PenNode::Text(n) => n.fill.as_ref(),
        _ => None,
    }
}

fn first_fill_color(node: &PenNode) -> Option<&str> {
    let fills = node_fill(node)?;
    // TS: `firstFill && 'color' in firstFill && firstFill.color`
    // In Rust schema that's the `Solid` variant with a `color` field.
    for fill in fills {
        if let PenFill::Solid(s) = fill {
            return Some(&s.color);
        }
    }
    None
}

fn node_stroke(node: &PenNode) -> Option<&PenStroke> {
    match node {
        PenNode::Frame(n) => n.container.stroke.as_ref(),
        PenNode::Group(n) => n.container.stroke.as_ref(),
        PenNode::Rectangle(n) => n.container.stroke.as_ref(),
        _ => None,
    }
}

fn first_stroke_color(stroke: &PenStroke) -> Option<&str> {
    let fills = stroke.fill.as_ref()?;
    for fill in fills {
        if let PenFill::Solid(s) = fill {
            return Some(&s.color);
        }
    }
    None
}

fn stroke_thickness_value(thickness: &StrokeThickness) -> f32 {
    // TS: `typeof s.thickness === 'number' ? s.thickness : Array.isArray ? s.thickness[0] : 0`
    match thickness {
        StrokeThickness::Uniform(v) => *v,
        StrokeThickness::PerSide(arr) => arr[0],
        StrokeThickness::Sided(_) => 0.0,
    }
}

fn node_children(node: &PenNode) -> Option<&Vec<PenNode>> {
    match node {
        PenNode::Frame(n) => n.children.as_ref(),
        PenNode::Group(n) => n.children.as_ref(),
        PenNode::Rectangle(n) => n.children.as_ref(),
        _ => None,
    }
}

// ── formatting helpers ────────────────────────────────────────────────────────

/// Format a `SizingBehavior` as TS `JSON.stringify(node.width)` would:
///   number  → bare number literal  e.g. `390`
///   keyword → quoted string        e.g. `"fit_content"`
///   expr    → quoted string        e.g. `"$spacing"`
fn format_sizing(s: &SizingBehavior) -> String {
    use jian_ops_schema::sizing::SizingKeyword;
    match s {
        SizingBehavior::Number(n) => format_f64(*n),
        SizingBehavior::Keyword(SizingKeyword::FitContent) => "\"fit_content\"".to_string(),
        SizingBehavior::Keyword(SizingKeyword::FillContainer) => "\"fill_container\"".to_string(),
        SizingBehavior::Expression(e) => format!("\"{e}\""),
    }
}

fn format_layout(layout: &jian_ops_schema::node::LayoutMode) -> &'static str {
    use jian_ops_schema::node::LayoutMode;
    match layout {
        LayoutMode::None => "none",
        LayoutMode::Vertical => "vertical",
        LayoutMode::Horizontal => "horizontal",
    }
}

fn format_number_or_expr(noe: &jian_ops_schema::node::base::NumberOrExpression) -> String {
    use jian_ops_schema::node::base::NumberOrExpression;
    match noe {
        NumberOrExpression::Number(n) => format_f64(*n),
        NumberOrExpression::Expression(e) => e.clone(),
    }
}

fn format_padding(p: &jian_ops_schema::node::container::Padding) -> String {
    use jian_ops_schema::node::container::Padding;
    // TS uses `JSON.stringify(node.padding)` — mirror that:
    //   Uniform(n)        → number literal
    //   XY([a,b])         → "[a,b]"
    //   LtrB([a,b,c,d])   → "[a,b,c,d]"
    //   Expression(s)     → quoted string
    match p {
        Padding::Uniform(n) => format_f64(*n),
        Padding::XY([a, b]) => format!("[{},{}]", format_f64(*a), format_f64(*b)),
        Padding::LtrB([a, b, c, d]) => format!(
            "[{},{},{},{}]",
            format_f64(*a),
            format_f64(*b),
            format_f64(*c),
            format_f64(*d)
        ),
        Padding::Expression(s) => format!("\"{s}\""),
    }
}

fn format_justify(jc: &jian_ops_schema::node::JustifyContent) -> &'static str {
    use jian_ops_schema::node::JustifyContent;
    match jc {
        JustifyContent::Start => "start",
        JustifyContent::Center => "center",
        JustifyContent::End => "end",
        JustifyContent::SpaceBetween => "space_between",
        JustifyContent::SpaceAround => "space_around",
    }
}

fn format_align(ai: &jian_ops_schema::node::AlignItems) -> &'static str {
    use jian_ops_schema::node::AlignItems;
    match ai {
        AlignItems::Start => "start",
        AlignItems::Center => "center",
        AlignItems::End => "end",
        AlignItems::Stretch => "stretch",
    }
}

fn format_corner_radius(cr: &jian_ops_schema::node::container::CornerRadius) -> String {
    use jian_ops_schema::node::container::CornerRadius;
    match cr {
        CornerRadius::Uniform(n) => format_f64(*n),
        CornerRadius::PerCorner([a, b, c, d]) => format!(
            "[{},{},{},{}]",
            format_f64(*a),
            format_f64(*b),
            format_f64(*c),
            format_f64(*d)
        ),
    }
}

fn format_font_weight(fw: &jian_ops_schema::node::FontWeight) -> String {
    use jian_ops_schema::node::FontWeight;
    match fw {
        FontWeight::Number(n) => n.to_string(),
        FontWeight::Keyword(k) => k.clone(),
    }
}

fn format_text_growth(tg: &jian_ops_schema::node::TextGrowth) -> &'static str {
    use jian_ops_schema::node::TextGrowth;
    match tg {
        TextGrowth::Auto => "auto",
        TextGrowth::FixedWidth => "fixed-width",
        TextGrowth::FixedWidthHeight => "fixed-width-and-height",
    }
}

fn format_text_align(ta: &jian_ops_schema::node::TextAlign) -> &'static str {
    use jian_ops_schema::node::TextAlign;
    match ta {
        TextAlign::Left => "left",
        TextAlign::Center => "center",
        TextAlign::Right => "right",
        TextAlign::Justify => "justify",
    }
}

fn extract_text_content(content: &TextContent) -> String {
    match content {
        TextContent::Plain(s) => s.clone(),
        TextContent::Styled(segs) => {
            // Join all segment texts
            segs.iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join("")
        }
    }
}

/// Format a float: drop the trailing `.0` when the value is a whole
/// number, to match JavaScript's `JSON.stringify` behaviour.
fn format_f64(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        // Use shortest decimal representation
        let s = format!("{v}");
        s
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "validation_dump_tests.rs"]
mod tests;
