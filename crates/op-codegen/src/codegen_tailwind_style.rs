//! Node → Tailwind utility-class mapping for the `react-tailwind` target:
//! placement, box sizing, flex layout, colours, borders, radii, effects
//! and typography. Pure functions over one node plus the parent's layout
//! kind; the emission context only records which design tokens were
//! referenced so the theme files bind exactly those.

use jian_ops_schema::node::container::{AlignItems, ContainerProps, JustifyContent, LayoutMode};
use jian_ops_schema::node::text::{FontStyleKind, FontWeight, TextAlign, TextGrowth, TextNode};
use jian_ops_schema::node::{CornerRadius, NumberOrExpression, Padding, PenNode};
use jian_ops_schema::sizing::{SizeLimits, SizingBehavior, SizingKeyword};
use jian_ops_schema::style::{PenEffect, PenFill, PenStroke, StrokeThickness};
use op_editor_core::pen_node_ext::PenNodeExt;

use super::scale::{self, px, spacing};
use super::Ctx;
use crate::fmt_num;

/// How the parent lays this node out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Parent {
    /// A top-level node: canvas coordinates are not page layout.
    Root,
    /// A flex container (`horizontal == false` → column).
    Flex { horizontal: bool },
    /// A `layout: "none"` stack (single-cell grid).
    Stack,
}

/// Ordered, de-duplicated class list.
#[derive(Debug, Default, Clone)]
pub(super) struct Classes(Vec<String>);

impl Classes {
    pub(super) fn push(&mut self, class: impl Into<String>) {
        let class = class.into();
        if !class.is_empty() && !self.0.contains(&class) {
            self.0.push(class);
        }
    }

    pub(super) fn push_opt(&mut self, class: Option<impl Into<String>>) {
        if let Some(class) = class {
            self.push(class);
        }
    }

    pub(super) fn extend(&mut self, other: Classes) {
        for class in other.0 {
            self.push(class);
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// ` className="…"` attribute (leading space), or empty.
    pub(super) fn attr(&self) -> String {
        if self.0.is_empty() {
            String::new()
        } else {
            format!(" className=\"{}\"", self.0.join(" "))
        }
    }
}

// --- Accessors -------------------------------------------------------

pub(super) fn container(node: &PenNode) -> Option<&ContainerProps> {
    match node {
        PenNode::Frame(n) => Some(&n.container),
        PenNode::Group(n) => Some(&n.container),
        PenNode::Rectangle(n) => Some(&n.container),
        _ => None,
    }
}

type Sizing<'a> = (
    Option<&'a SizingBehavior>,
    Option<&'a SizingBehavior>,
    Option<&'a SizeLimits>,
);

fn sizing(node: &PenNode) -> Sizing<'_> {
    macro_rules! leaf {
        ($n:expr) => {
            ($n.width.as_ref(), $n.height.as_ref(), Some(&$n.limits))
        };
    }
    match node {
        PenNode::Frame(n) => leaf!(n.container),
        PenNode::Group(n) => leaf!(n.container),
        PenNode::Rectangle(n) => leaf!(n.container),
        PenNode::Ellipse(n) => leaf!(n),
        PenNode::Polygon(n) => leaf!(n),
        PenNode::Path(n) => leaf!(n),
        PenNode::Text(n) => leaf!(n),
        PenNode::TextInput(n) => leaf!(n),
        PenNode::Image(n) => leaf!(n),
        PenNode::IconFont(n) => leaf!(n),
        PenNode::TextArea(n) => leaf!(n),
        PenNode::Select(n) => leaf!(n),
        PenNode::Switch(n) => leaf!(n),
        PenNode::Checkbox(n) => leaf!(n),
        PenNode::Slider(n) => leaf!(n),
        PenNode::RadioGroup(n) => leaf!(n),
        PenNode::NumberInput(n) => leaf!(n),
        PenNode::Progress(n) => leaf!(n),
        PenNode::Tabs(n) => leaf!(n),
        PenNode::Line(_) | PenNode::Ref(_) => (None, None, None),
    }
}

pub(super) fn fills(node: &PenNode) -> Option<&[PenFill]> {
    let fills = match node {
        PenNode::Frame(n) => n.container.fill.as_ref(),
        PenNode::Group(n) => n.container.fill.as_ref(),
        PenNode::Rectangle(n) => n.container.fill.as_ref(),
        PenNode::Ellipse(n) => n.fill.as_ref(),
        PenNode::Polygon(n) => n.fill.as_ref(),
        PenNode::Path(n) => n.fill.as_ref(),
        PenNode::Text(n) => n.fill.as_ref(),
        PenNode::TextInput(n) => n.fill.as_ref(),
        PenNode::IconFont(n) => n.fill.as_ref(),
        PenNode::TextArea(n) => n.fill.as_ref(),
        PenNode::Select(n) => n.fill.as_ref(),
        PenNode::Switch(n) => n.fill.as_ref(),
        PenNode::Checkbox(n) => n.fill.as_ref(),
        PenNode::Slider(n) => n.fill.as_ref(),
        PenNode::RadioGroup(n) => n.fill.as_ref(),
        PenNode::NumberInput(n) => n.fill.as_ref(),
        PenNode::Progress(n) => n.fill.as_ref(),
        PenNode::Tabs(n) => n.fill.as_ref(),
        PenNode::Line(_) | PenNode::Image(_) | PenNode::Ref(_) => None,
    };
    fills.map(Vec::as_slice)
}

pub(super) fn stroke(node: &PenNode) -> Option<&PenStroke> {
    match node {
        PenNode::Frame(n) => n.container.stroke.as_ref(),
        PenNode::Group(n) => n.container.stroke.as_ref(),
        PenNode::Rectangle(n) => n.container.stroke.as_ref(),
        PenNode::Ellipse(n) => n.stroke.as_ref(),
        PenNode::Polygon(n) => n.stroke.as_ref(),
        PenNode::Path(n) => n.stroke.as_ref(),
        PenNode::Line(n) => n.stroke.as_ref(),
        PenNode::TextInput(n) => n.stroke.as_ref(),
        PenNode::IconFont(n) => n.stroke.as_ref(),
        PenNode::TextArea(n) => n.stroke.as_ref(),
        PenNode::Select(n) => n.stroke.as_ref(),
        PenNode::Switch(n) => n.stroke.as_ref(),
        PenNode::Checkbox(n) => n.stroke.as_ref(),
        PenNode::Slider(n) => n.stroke.as_ref(),
        PenNode::RadioGroup(n) => n.stroke.as_ref(),
        PenNode::NumberInput(n) => n.stroke.as_ref(),
        PenNode::Progress(n) => n.stroke.as_ref(),
        PenNode::Tabs(n) => n.stroke.as_ref(),
        PenNode::Text(_) | PenNode::Image(_) | PenNode::Ref(_) => None,
    }
}

fn effects(node: &PenNode) -> Option<&[PenEffect]> {
    let effects = match node {
        PenNode::Frame(n) => n.container.effects.as_ref(),
        PenNode::Group(n) => n.container.effects.as_ref(),
        PenNode::Rectangle(n) => n.container.effects.as_ref(),
        PenNode::Ellipse(n) => n.effects.as_ref(),
        PenNode::Polygon(n) => n.effects.as_ref(),
        PenNode::Path(n) => n.effects.as_ref(),
        PenNode::Line(n) => n.effects.as_ref(),
        PenNode::Text(n) => n.effects.as_ref(),
        PenNode::Image(n) => n.effects.as_ref(),
        PenNode::TextInput(n) => n.effects.as_ref(),
        PenNode::TextArea(n) => n.effects.as_ref(),
        PenNode::Select(n) => n.effects.as_ref(),
        PenNode::Switch(n) => n.effects.as_ref(),
        PenNode::Checkbox(n) => n.effects.as_ref(),
        PenNode::Slider(n) => n.effects.as_ref(),
        PenNode::RadioGroup(n) => n.effects.as_ref(),
        PenNode::NumberInput(n) => n.effects.as_ref(),
        PenNode::Progress(n) => n.effects.as_ref(),
        PenNode::Tabs(n) => n.effects.as_ref(),
        PenNode::IconFont(_) | PenNode::Ref(_) => None,
    };
    effects.map(Vec::as_slice)
}

fn corner_radius(node: &PenNode) -> Option<CornerRadius> {
    let uniform = |r: Option<f64>| r.map(CornerRadius::Uniform);
    match node {
        PenNode::Frame(n) => n.container.corner_radius.clone(),
        PenNode::Group(n) => n.container.corner_radius.clone(),
        PenNode::Rectangle(n) => n.container.corner_radius.clone(),
        PenNode::Polygon(n) => uniform(n.corner_radius),
        PenNode::Image(n) => n.corner_radius.clone(),
        PenNode::TextInput(n) => n.corner_radius.clone(),
        PenNode::TextArea(n) => n.corner_radius.clone(),
        PenNode::Select(n) => n.corner_radius.clone(),
        PenNode::Switch(n) => n.corner_radius.clone(),
        PenNode::Checkbox(n) => n.corner_radius.clone(),
        PenNode::Slider(n) => n.corner_radius.clone(),
        PenNode::RadioGroup(n) => n.corner_radius.clone(),
        PenNode::NumberInput(n) => n.corner_radius.clone(),
        PenNode::Progress(n) => n.corner_radius.clone(),
        PenNode::Tabs(n) => n.corner_radius.clone(),
        _ => None,
    }
}

/// True when the node carries an authored `x`/`y` — the layout engine
/// positions such nodes absolutely inside any parent.
pub(super) fn has_explicit_position(node: &PenNode) -> bool {
    let base = node.base();
    base.x.is_some() || base.y.is_some()
}

/// A `$token` / `$--token` reference → the bare token name (`primary`).
pub(super) fn token_name(value: &str) -> Option<&str> {
    let name = value.trim().strip_prefix('$')?.trim_start_matches('-');
    (!name.is_empty()).then_some(name)
}

fn is_class_ident(name: &str) -> bool {
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// `var(--token)` for arbitrary values; records the token as used.
fn var_ref(ctx: &mut Ctx, name: &str) -> String {
    ctx.tokens.insert(name.to_string());
    format!("var(--{name})")
}

/// Arbitrary-value body for an expression-valued length.
fn expression_length(ctx: &mut Ctx, value: &str) -> Option<String> {
    let name = token_name(value)?;
    is_class_ident(name).then(|| var_ref(ctx, name))
}

// --- Colour ---------------------------------------------------------

/// CSS colour value usable inside an arbitrary Tailwind value (no
/// spaces); `$token` refs become `var(--token)`.
pub(super) fn css_color(ctx: &mut Ctx, color: &str) -> Option<String> {
    let color = color.trim();
    if let Some(name) = token_name(color) {
        return is_class_ident(name).then(|| var_ref(ctx, name));
    }
    if color.is_empty() || color.contains(['[', ']', '"', '\'', '`', '{', '}']) {
        return None;
    }
    Some(
        color
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_ascii_lowercase(),
    )
}

/// A colour utility: `bg-primary`, `text-muted-foreground/50`,
/// `border-[#e4e4e7]`, `bg-white`.
pub(super) fn color_class(
    ctx: &mut Ctx,
    prefix: &str,
    color: &str,
    alpha: Option<f32>,
) -> Option<String> {
    let suffix = alpha.map(scale::color_alpha_suffix).unwrap_or_default();
    let trimmed = color.trim();
    if let Some(name) = token_name(trimmed) {
        if is_class_ident(name) {
            ctx.tokens.insert(name.to_string());
            return Some(format!("{prefix}-{name}{suffix}"));
        }
        return None;
    }
    let value = css_color(ctx, trimmed)?;
    let named = match value.as_str() {
        "transparent" => Some("transparent"),
        "#fff" | "#ffffff" | "#ffffffff" | "white" => Some("white"),
        "#000" | "#000000" | "#000000ff" | "black" => Some("black"),
        _ => None,
    };
    if let Some(named) = named {
        return Some(format!("{prefix}-{named}{suffix}"));
    }
    if value.starts_with('#') || value.starts_with("rgb") || value.starts_with("hsl") {
        Some(format!("{prefix}-[{value}]{suffix}"))
    } else {
        Some(format!("{prefix}-[color:{value}]{suffix}"))
    }
}

fn gradient_stops(ctx: &mut Ctx, stops: &[jian_ops_schema::style::GradientStop]) -> String {
    stops
        .iter()
        .filter_map(|stop| {
            let color = css_color(ctx, &stop.color)?;
            let offset = fmt_num((f64::from(stop.offset) * 1000.0).round() / 10.0);
            Some(format!("{color}_{offset}%"))
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Background utilities for the first paintable fill.
fn background(ctx: &mut Ctx, node: &PenNode, classes: &mut Classes) {
    let Some(fills) = fills(node) else {
        return;
    };
    for fill in fills {
        match fill {
            PenFill::Solid(body) => {
                classes.push_opt(color_class(ctx, "bg", &body.color, body.opacity));
                return;
            }
            PenFill::LinearGradient(body) if body.stops.len() >= 2 => {
                // `.op` angles run clockwise from +x (0° = left→right);
                // CSS angles run clockwise from +y-up (90° = left→right).
                let css_angle = (f64::from(body.angle.unwrap_or(0.0)) + 90.0).rem_euclid(360.0);
                let stops = gradient_stops(ctx, &body.stops);
                classes.push(format!(
                    "bg-[linear-gradient({}deg,{stops})]",
                    fmt_num((css_angle * 100.0).round() / 100.0)
                ));
                return;
            }
            PenFill::RadialGradient(body) if body.stops.len() >= 2 => {
                let cx = fmt_num((f64::from(body.cx.unwrap_or(0.5)) * 1000.0).round() / 10.0);
                let cy = fmt_num((f64::from(body.cy.unwrap_or(0.5)) * 1000.0).round() / 10.0);
                let stops = gradient_stops(ctx, &body.stops);
                classes.push(format!(
                    "bg-[radial-gradient(circle_at_{cx}%_{cy}%,{stops})]"
                ));
                return;
            }
            PenFill::Image(body) => {
                let url: &str = body.url.as_ref();
                if !url.is_empty()
                    && !url.starts_with("data:")
                    && !url.contains(|c: char| c.is_whitespace() || "'\"()[]".contains(c))
                {
                    classes.push(format!("bg-[url('{url}')]"));
                    classes.push("bg-cover");
                    classes.push("bg-center");
                }
                return;
            }
            _ => {}
        }
    }
}

/// First solid fill colour → `text-*` (text / icon foreground).
pub(super) fn foreground(ctx: &mut Ctx, node: &PenNode) -> Option<String> {
    fills(node)?.iter().find_map(|fill| match fill {
        PenFill::Solid(body) => color_class(ctx, "text", &body.color, body.opacity),
        _ => None,
    })
}

// --- Placement, sizing, layout ----------------------------------------

/// Absolute placement / stack-cell classes relative to the parent.
pub(super) fn placement(node: &PenNode, parent: Parent, classes: &mut Classes) {
    if parent == Parent::Root {
        return;
    }
    if has_explicit_position(node) {
        let base = node.base();
        classes.push("absolute");
        classes.push(spacing("left", base.x.unwrap_or(0.0)));
        classes.push(spacing("top", base.y.unwrap_or(0.0)));
    } else if parent == Parent::Stack {
        classes.push("col-start-1");
        classes.push("row-start-1");
    }
}

fn axis_size(
    ctx: &mut Ctx,
    axis: &str,
    value: Option<&SizingBehavior>,
    parent: Parent,
    classes: &mut Classes,
) {
    let horizontal_axis = axis == "w";
    match value {
        Some(SizingBehavior::Number(n)) => classes.push(spacing(axis, *n)),
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)) => match parent {
            Parent::Flex { horizontal } if horizontal == horizontal_axis => {
                classes.push("flex-1");
                classes.push(if horizontal_axis {
                    "min-w-0"
                } else {
                    "min-h-0"
                });
            }
            Parent::Flex { .. } => classes.push("self-stretch"),
            Parent::Root | Parent::Stack => classes.push(format!("{axis}-full")),
        },
        Some(SizingBehavior::Keyword(SizingKeyword::FitContent)) => {}
        Some(SizingBehavior::Expression(expr)) => {
            if let Some(value) = expression_length(ctx, expr) {
                classes.push(format!("{axis}-[{value}]"));
            }
        }
        None => {}
    }
}

/// Width / height / limits, honouring text growth modes.
pub(super) fn size(ctx: &mut Ctx, node: &PenNode, parent: Parent, classes: &mut Classes) {
    let (width, height, limits) = sizing(node);
    let (width, height) = match node {
        PenNode::Text(text) => text_box(text, width, height),
        _ => (width, height),
    };
    axis_size(ctx, "w", width, parent, classes);
    axis_size(ctx, "h", height, parent, classes);
    let fixed = matches!(width, Some(SizingBehavior::Number(_)))
        && matches!(height, Some(SizingBehavior::Number(_)));
    if fixed && matches!(parent, Parent::Flex { .. }) && !has_explicit_position(node) {
        classes.push("shrink-0");
    }
    if let Some(limits) = limits {
        for (prefix, value) in [
            ("min-w", limits.min_width),
            ("max-w", limits.max_width),
            ("min-h", limits.min_height),
            ("max-h", limits.max_height),
        ] {
            if let Some(value) = value.filter(|v| v.is_finite() && *v >= 0.0) {
                classes.push(format!("{prefix}-[{}]", px(value)));
            }
        }
    }
}

/// Text boxes: auto growth hugs its content, fixed-width wraps at the
/// authored width, fixed-width-height also pins the height.
fn text_box<'a>(
    text: &TextNode,
    width: Option<&'a SizingBehavior>,
    height: Option<&'a SizingBehavior>,
) -> (Option<&'a SizingBehavior>, Option<&'a SizingBehavior>) {
    let keyword =
        |v: Option<&'a SizingBehavior>| v.filter(|v| !matches!(v, SizingBehavior::Number(_)));
    match text.text_growth {
        Some(TextGrowth::Auto) => (keyword(width), keyword(height)),
        Some(TextGrowth::FixedWidthHeight) => (width, height),
        Some(TextGrowth::FixedWidth) | None => (width, keyword(height)),
    }
}

/// The layout kind this container imposes on its children.
pub(super) fn child_parent_kind(node: &PenNode) -> Parent {
    match container(node).and_then(|c| c.layout.as_ref()) {
        Some(LayoutMode::None) => Parent::Stack,
        Some(LayoutMode::Vertical) => Parent::Flex { horizontal: false },
        Some(LayoutMode::Horizontal) | None => Parent::Flex { horizontal: true },
    }
}

/// Flex / grid / padding / overflow classes for a container.
pub(super) fn layout(ctx: &mut Ctx, node: &PenNode, classes: &mut Classes) {
    let Some(c) = container(node) else {
        return;
    };
    let children = node.children().map(Vec::as_slice).unwrap_or_default();
    let any_absolute = children.iter().any(has_explicit_position);
    let any_flow = children.iter().any(|child| !has_explicit_position(child));
    if any_absolute || matches!(c.layout, Some(LayoutMode::None)) && !children.is_empty() {
        classes.push("relative");
    }
    match c.layout {
        Some(LayoutMode::None) => {
            if any_flow {
                classes.push("grid");
            }
        }
        Some(LayoutMode::Vertical) | Some(LayoutMode::Horizontal) | None if any_flow => {
            classes.push("flex");
            if matches!(c.layout, Some(LayoutMode::Vertical)) {
                classes.push("flex-col");
            }
            classes.push(align(c.align_items.as_ref()));
            classes.push_opt(justify(c.justify_content.as_ref()));
            match &c.gap {
                Some(NumberOrExpression::Number(g)) if *g != 0.0 => {
                    classes.push(spacing("gap", *g))
                }
                Some(NumberOrExpression::Expression(e)) => {
                    if let Some(value) = expression_length(ctx, e) {
                        classes.push(format!("gap-[{value}]"));
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    padding(ctx, c.padding.as_ref(), classes);
    if c.clip_content == Some(true) {
        classes.push("overflow-hidden");
    }
}

fn justify(value: Option<&JustifyContent>) -> Option<&'static str> {
    Some(match value? {
        JustifyContent::Start => return None,
        JustifyContent::Center => "justify-center",
        JustifyContent::End => "justify-end",
        JustifyContent::SpaceBetween => "justify-between",
        JustifyContent::SpaceAround => "justify-around",
    })
}

/// The layout engine aligns unset cross axes to the start (CSS would
/// stretch), so the default is spelled out.
fn align(value: Option<&AlignItems>) -> &'static str {
    match value {
        Some(AlignItems::Center) => "items-center",
        Some(AlignItems::End) => "items-end",
        Some(AlignItems::Stretch) => "items-stretch",
        Some(AlignItems::Start) | None => "items-start",
    }
}

fn padding(ctx: &mut Ctx, value: Option<&Padding>, classes: &mut Classes) {
    let (t, r, b, l) = match value {
        None => return,
        Some(Padding::Uniform(p)) => (*p, *p, *p, *p),
        Some(Padding::XY([v, h])) => (*v, *h, *v, *h),
        Some(Padding::LtrB([t, r, b, l])) => (*t, *r, *b, *l),
        Some(Padding::Expression(e)) => {
            if let Some(value) = expression_length(ctx, e) {
                classes.push(format!("p-[{value}]"));
            }
            return;
        }
    };
    if t == r && r == b && b == l {
        if t != 0.0 {
            classes.push(spacing("p", t));
        }
        return;
    }
    if t == b && l == r {
        if t != 0.0 {
            classes.push(spacing("py", t));
        }
        if l != 0.0 {
            classes.push(spacing("px", l));
        }
        return;
    }
    for (prefix, value) in [("pt", t), ("pr", r), ("pb", b), ("pl", l)] {
        if value != 0.0 {
            classes.push(spacing(prefix, value));
        }
    }
}

// --- Visuals ----------------------------------------------------------

/// Fill, border, radius, effects, opacity and transform classes.
pub(super) fn visuals(ctx: &mut Ctx, node: &PenNode, classes: &mut Classes) {
    if !matches!(
        node,
        PenNode::Text(_) | PenNode::IconFont(_) | PenNode::Path(_)
    ) {
        background(ctx, node, classes);
    }
    border(ctx, node, classes);
    if matches!(node, PenNode::Ellipse(_)) {
        classes.push("rounded-full");
    } else {
        radius(ctx, node, classes);
    }
    shadow_and_blur(ctx, node, classes);
    let base = node.base();
    if let Some(NumberOrExpression::Number(o)) = &base.opacity {
        classes.push_opt(scale::opacity(*o));
    }
    if let Some(rotation) = base.rotation {
        classes.push_opt(scale::rotate(rotation));
    }
    if base.flip_x == Some(true) {
        classes.push("-scale-x-100");
    }
    if base.flip_y == Some(true) {
        classes.push("-scale-y-100");
    }
}

fn border(ctx: &mut Ctx, node: &PenNode, classes: &mut Classes) {
    let Some(stroke) = stroke(node) else {
        return;
    };
    if matches!(node, PenNode::Path(_) | PenNode::Line(_)) {
        return;
    }
    let color = stroke.fill.as_ref().and_then(|fills| {
        fills.iter().find_map(|f| match f {
            PenFill::Solid(body) => Some((body.color.clone(), body.opacity)),
            _ => None,
        })
    });
    let sides: [(&str, f32); 4] = match &stroke.thickness {
        StrokeThickness::Uniform(w) => [("t", *w), ("r", *w), ("b", *w), ("l", *w)],
        StrokeThickness::PerSide([t, r, b, l]) => [("t", *t), ("r", *r), ("b", *b), ("l", *l)],
        StrokeThickness::Sided(s) => [
            ("t", s.top.unwrap_or(0.0)),
            ("r", s.right.unwrap_or(0.0)),
            ("b", s.bottom.unwrap_or(0.0)),
            ("l", s.left.unwrap_or(0.0)),
        ],
    };
    let uniform = sides.iter().all(|(_, w)| *w == sides[0].1);
    let mut any = false;
    if uniform {
        if let Some(class) = scale::border_width("", f64::from(sides[0].1)) {
            classes.push(class);
            any = true;
        }
    } else {
        for (side, width) in sides {
            if let Some(class) = scale::border_width(side, f64::from(width)) {
                classes.push(class);
                any = true;
            }
        }
    }
    if !any {
        return;
    }
    if let Some((color, alpha)) = color {
        classes.push_opt(color_class(ctx, "border", &color, alpha));
    }
    if stroke.dash_pattern.as_ref().is_some_and(|d| !d.is_empty()) {
        classes.push("border-dashed");
    }
}

fn radius(ctx: &mut Ctx, node: &PenNode, classes: &mut Classes) {
    match corner_radius(node) {
        Some(CornerRadius::Uniform(r)) => classes.push_opt(ctx.radius.class("", r)),
        Some(CornerRadius::PerCorner([tl, tr, br, bl])) => {
            if tl == tr && tr == br && br == bl {
                classes.push_opt(ctx.radius.class("", tl));
            } else {
                for (corner, value) in [("tl", tl), ("tr", tr), ("br", br), ("bl", bl)] {
                    classes.push_opt(ctx.radius.class(corner, value));
                }
            }
        }
        None => {}
    }
}

fn shadow_and_blur(ctx: &mut Ctx, node: &PenNode, classes: &mut Classes) {
    let Some(effects) = effects(node) else {
        return;
    };
    let mut shadows = Vec::new();
    for effect in effects {
        match effect {
            PenEffect::Shadow(s) if s.visible != Some(false) => {
                let Some(color) = css_color(ctx, &s.color) else {
                    continue;
                };
                let inset = if s.inner == Some(true) { "inset_" } else { "" };
                shadows.push(format!(
                    "{inset}{}_{}_{}_{}_{color}",
                    px(f64::from(s.offset_x)),
                    px(f64::from(s.offset_y)),
                    px(f64::from(s.blur)),
                    px(f64::from(s.spread)),
                ));
            }
            PenEffect::Blur(b) if b.visible != Some(false) && b.radius > 0.0 => {
                classes.push(format!("blur-[{}]", px(f64::from(b.radius))));
            }
            PenEffect::BackgroundBlur(b) if b.visible != Some(false) && b.radius > 0.0 => {
                classes.push(format!("backdrop-blur-[{}]", px(f64::from(b.radius))));
            }
            _ => {}
        }
    }
    if !shadows.is_empty() {
        classes.push(format!("shadow-[{}]", shadows.join(",")));
    }
}

// --- Typography -------------------------------------------------------

/// Font / colour / decoration classes for a text node.
pub(super) fn typography(ctx: &mut Ctx, text: &TextNode, node: &PenNode, classes: &mut Classes) {
    if let Some(family) = text.font_family.as_deref() {
        classes.push_opt(font_family(ctx, family));
    }
    if let Some(size) = text.font_size.filter(|s| s.is_finite() && *s > 0.0) {
        classes.push(scale::font_size(size));
    }
    match &text.font_weight {
        Some(FontWeight::Number(w)) => classes.push_opt(scale::font_weight(*w)),
        Some(FontWeight::Keyword(k)) => classes.push_opt(scale::font_weight_keyword(k)),
        None => {}
    }
    if let Some(lh) = text.line_height {
        classes.push_opt(scale::leading(lh));
    }
    if let Some(ls) = text.letter_spacing.filter(|v| v.is_finite() && *v != 0.0) {
        classes.push(format!("tracking-[{}]", px(ls)));
    }
    classes.push_opt(match text.text_align {
        Some(TextAlign::Center) => Some("text-center"),
        Some(TextAlign::Right) => Some("text-right"),
        Some(TextAlign::Justify) => Some("text-justify"),
        Some(TextAlign::Left) | None => None,
    });
    if text.font_style == Some(FontStyleKind::Italic) {
        classes.push("italic");
    }
    if text.underline == Some(true) {
        classes.push("underline");
    }
    if text.strikethrough == Some(true) {
        classes.push("line-through");
    }
    classes.push_opt(foreground(ctx, node));
}

/// `font-['Inter']`, or `font-[family-name:var(--font-sans)]` for a
/// token (the type hint keeps Tailwind from reading it as a weight).
pub(super) fn font_family(ctx: &mut Ctx, family: &str) -> Option<String> {
    if let Some(name) = token_name(family) {
        return is_class_ident(name).then(|| format!("font-[family-name:{}]", var_ref(ctx, name)));
    }
    let family = family.trim();
    if family.is_empty() || family.contains(['\'', '"', '[', ']', '{', '}', '\\']) {
        return None;
    }
    Some(format!("font-['{}']", family.replace(' ', "_")))
}
