//! shadcn/ui kit recognition for the `react-tailwind` target.
//!
//! Instances of the built-in shadcn UIKit (`shadcn-*` components) are
//! emitted as real shadcn/ui component usages (`<Button variant=…>`,
//! `<Card>…`) with an `@/components/ui/*` import, instead of their
//! drawn frame geometry. Recognition sources, strongest first:
//!
//! 1. A component instance (`ref`) whose target is a kit component —
//!    by kit id, or a document component carrying a kit id / kit name.
//!    Refs are tagged with [`KIT_ROLE_PREFIX`] before expansion so the
//!    identity survives `ref_resolve` (including nested instances).
//! 2. A reusable master frame whose id is a kit id.
//! 3. A plain frame named exactly like a kit component (what
//!    `insert_<component>` leaves behind).
//!
//! Every recognition is confirmed structurally by the emitter: when the
//! instance's subtree does not have the kit's shape (e.g. a "Badge"
//! frame holding an image) the caller falls back to generic markup, so
//! no authored content is dropped.

use jian_ops_schema::node::{PenNode, TextContent};
use op_editor_core::pen_node_ext::PenNodeExt;

use super::jsx::{jsx_attr, jsx_text, line};
use super::style::{self, Classes, Parent};
use super::Ctx;
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};

/// Role marker written onto kit instances before ref expansion.
pub(super) const KIT_ROLE_PREFIX: &str = "shadcn-kit:";

/// One recognized shadcn/ui component shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kit {
    /// `Button` with an optional `variant`.
    Button(Option<&'static str>),
    Badge,
    Input,
    Textarea,
    Checkbox,
    Switch,
    Radio,
    CardBasic,
    CardStats,
    Tabs,
    Breadcrumb,
    Alert,
    Avatar,
    Separator,
    Select,
    Slider,
    Progress,
    Skeleton,
    Table,
}

/// `(kit component id, kit component name, shadcn shape)`. Kit
/// components absent here (dialog, dropdown, toast, …) depict overlay
/// states and are emitted as generic markup.
pub(super) const KIT_COMPONENTS: &[(&str, &str, Kit)] = &[
    ("shadcn-btn-primary", "Primary Button", Kit::Button(None)),
    (
        "shadcn-btn-secondary",
        "Secondary Button",
        Kit::Button(Some("secondary")),
    ),
    (
        "shadcn-btn-ghost",
        "Ghost Button",
        Kit::Button(Some("ghost")),
    ),
    (
        "shadcn-btn-destructive",
        "Destructive Button",
        Kit::Button(Some("destructive")),
    ),
    (
        "shadcn-btn-outline",
        "Outline Button",
        Kit::Button(Some("outline")),
    ),
    ("shadcn-btn-link", "Link Button", Kit::Button(Some("link"))),
    ("shadcn-badge", "Badge", Kit::Badge),
    ("shadcn-input-text", "Text Input", Kit::Input),
    ("shadcn-input-textarea", "Textarea", Kit::Textarea),
    ("shadcn-input-checkbox", "Checkbox", Kit::Checkbox),
    ("shadcn-input-toggle", "Toggle Switch", Kit::Switch),
    ("shadcn-input-radio", "Radio Button", Kit::Radio),
    ("shadcn-card-basic", "Basic Card", Kit::CardBasic),
    ("shadcn-card-stats", "Stats Card", Kit::CardStats),
    ("shadcn-tab-bar", "Tab Bar", Kit::Tabs),
    ("shadcn-breadcrumb", "Breadcrumb", Kit::Breadcrumb),
    ("shadcn-alert-banner", "Alert Banner", Kit::Alert),
    ("shadcn-avatar", "Avatar", Kit::Avatar),
    ("shadcn-divider", "Divider", Kit::Separator),
    ("shadcn-input-select", "Select", Kit::Select),
    ("shadcn-input-slider", "Slider", Kit::Slider),
    ("shadcn-progress", "Progress", Kit::Progress),
    ("shadcn-skeleton", "Skeleton", Kit::Skeleton),
    ("shadcn-table", "Table", Kit::Table),
];

pub(super) fn kit_for_id(id: &str) -> Option<Kit> {
    KIT_COMPONENTS
        .iter()
        .find(|(kit_id, _, _)| *kit_id == id)
        .map(|(_, _, kit)| *kit)
}

pub(super) fn kit_id_for_name(name: &str) -> Option<&'static str> {
    KIT_COMPONENTS
        .iter()
        .find(|(_, kit_name, _)| *kit_name == name)
        .map(|(id, _, _)| *id)
}

/// The kit shape a (ref-expanded) node claims to be, before structural
/// confirmation.
pub(super) fn detect(node: &PenNode) -> Option<Kit> {
    let base = node.base();
    if let Some(id) = base
        .role
        .as_deref()
        .and_then(|r| r.strip_prefix(KIT_ROLE_PREFIX))
    {
        return kit_for_id(id);
    }
    if !matches!(node, PenNode::Frame(_)) {
        return None;
    }
    kit_for_id(&base.id).or_else(|| {
        base.name
            .as_deref()
            .and_then(kit_id_for_name)
            .and_then(kit_for_id)
    })
}

// --- Subtree probes ---------------------------------------------------

pub(super) fn visible_children(node: &PenNode) -> impl Iterator<Item = &PenNode> {
    node.children()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter(|child| child.base().visible != Some(false))
}

fn plain_text(node: &PenNode) -> Option<String> {
    let PenNode::Text(text) = node else {
        return None;
    };
    Some(match &text.content {
        TextContent::Plain(s) => s.clone(),
        TextContent::Styled(segments) => segments.iter().map(|s| s.text.as_str()).collect(),
    })
}

/// Visible text contents in depth-first order.
pub(super) fn texts(node: &PenNode) -> Vec<String> {
    fn walk(node: &PenNode, out: &mut Vec<String>) {
        for child in visible_children(node) {
            match plain_text(child) {
                Some(text) => out.push(text),
                None => walk(child, out),
            }
        }
    }
    let mut out = Vec::new();
    walk(node, &mut out);
    out
}

/// True when every visible descendant is a text or a decorative leaf
/// (shape / icon) — i.e. the node carries no nested content that a
/// shadcn primitive would swallow.
fn only_text_and_decor(node: &PenNode) -> bool {
    visible_children(node).all(|child| match child {
        PenNode::Text(_)
        | PenNode::IconFont(_)
        | PenNode::Ellipse(_)
        | PenNode::Rectangle(_)
        | PenNode::Path(_) => child.children().is_none_or(Vec::is_empty),
        _ => false,
    })
}

fn numeric_width(node: &PenNode) -> Option<f64> {
    node.width_px()
}

pub(super) fn dom_id(node: &PenNode) -> String {
    node.base()
        .id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// Outer classes of a shadcn usage: placement in the parent, fill-axis
/// flex behaviour, and — for width-driven primitives — the authored
/// width. Visual styling is left to the shadcn component itself.
pub(super) fn outer_classes(node: &PenNode, parent: Parent, keep_width: bool) -> Classes {
    let mut classes = Classes::default();
    style::placement(node, parent, &mut classes);
    let width = match node {
        PenNode::Frame(frame) => frame.container.width.as_ref(),
        _ => None,
    };
    match width {
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)) => match parent {
            Parent::Flex { horizontal: true } => {
                classes.push("flex-1");
                classes.push("min-w-0");
            }
            _ => classes.push("w-full"),
        },
        _ if keep_width => {
            if let Some(w) = numeric_width(node) {
                classes.push(super::scale::spacing("w", w));
            }
        }
        _ => {}
    }
    classes
}

/// Emit `node` as the shadcn usage for `kit`. Returns `false` (writing
/// nothing) when the subtree does not have the kit's shape.
pub(super) fn emit(
    ctx: &mut Ctx,
    out: &mut String,
    node: &PenNode,
    kit: Kit,
    parent: Parent,
    depth: usize,
) -> bool {
    let texts = texts(node);
    match kit {
        Kit::Button(_) | Kit::Badge if texts.len() == 1 && only_text_and_decor(node) => {
            let name = if kit == Kit::Badge { "Badge" } else { "Button" };
            ctx.ui(name);
            let variant = variant_attr(kit);
            let cls = outer_classes(node, parent, false).attr();
            line(
                out,
                depth,
                &format!("<{name}{variant}{cls}>{}</{name}>", jsx_text(&texts[0])),
            );
        }
        Kit::Input | Kit::Textarea if texts.len() <= 1 && only_text_and_decor(node) => {
            let name = if kit == Kit::Input {
                "Input"
            } else {
                "Textarea"
            };
            ctx.ui(name);
            let placeholder = texts
                .first()
                .map(|t| format!(" placeholder=\"{}\"", jsx_attr(t)))
                .unwrap_or_default();
            let cls = outer_classes(node, parent, true).attr();
            line(out, depth, &format!("<{name}{placeholder}{cls} />"));
        }
        Kit::Switch if texts.is_empty() && only_text_and_decor(node) => {
            ctx.ui("Switch");
            let cls = outer_classes(node, parent, false).attr();
            line(out, depth, &format!("<Switch{cls} />"));
        }
        Kit::Checkbox | Kit::Radio if texts.len() == 1 && only_text_and_decor(node) => {
            emit_labelled_choice(ctx, out, node, kit, parent, depth, &texts[0]);
        }
        Kit::CardBasic if (1..=2).contains(&texts.len()) && only_text_and_decor(node) => {
            ctx.ui("Card");
            ctx.ui("CardHeader");
            ctx.ui("CardTitle");
            let cls = outer_classes(node, parent, true).attr();
            line(out, depth, &format!("<Card{cls}>"));
            line(out, depth + 1, "<CardHeader>");
            line(
                out,
                depth + 2,
                &format!("<CardTitle>{}</CardTitle>", jsx_text(&texts[0])),
            );
            if let Some(description) = texts.get(1) {
                ctx.ui("CardDescription");
                line(
                    out,
                    depth + 2,
                    &format!(
                        "<CardDescription>{}</CardDescription>",
                        jsx_text(description)
                    ),
                );
            }
            line(out, depth + 1, "</CardHeader>");
            line(out, depth, "</Card>");
        }
        Kit::CardStats if texts.len() == 3 && only_text_and_decor(node) => {
            emit_stats_card(ctx, out, node, parent, depth, &texts);
        }
        Kit::Alert if (1..=2).contains(&texts.len()) && only_text_and_decor(node) => {
            ctx.ui("Alert");
            ctx.ui("AlertDescription");
            let cls = outer_classes(node, parent, true).attr();
            line(out, depth, &format!("<Alert{cls}>"));
            if texts.len() == 2 {
                ctx.ui("AlertTitle");
                line(
                    out,
                    depth + 1,
                    &format!("<AlertTitle>{}</AlertTitle>", jsx_text(&texts[0])),
                );
            }
            let body = texts.last().map(String::as_str).unwrap_or_default();
            line(
                out,
                depth + 1,
                &format!("<AlertDescription>{}</AlertDescription>", jsx_text(body)),
            );
            line(out, depth, "</Alert>");
        }
        Kit::Avatar if texts.len() == 1 && only_text_and_decor(node) => {
            ctx.ui("Avatar");
            ctx.ui("AvatarFallback");
            let cls = outer_classes(node, parent, false).attr();
            line(
                out,
                depth,
                &format!(
                    "<Avatar{cls}><AvatarFallback>{}</AvatarFallback></Avatar>",
                    jsx_text(&texts[0])
                ),
            );
        }
        Kit::Separator if visible_children(node).next().is_none() => {
            ctx.ui("Separator");
            let vertical = node.height_px().unwrap_or(0.0) > node.width_px().unwrap_or(0.0);
            let orientation = if vertical {
                " orientation=\"vertical\""
            } else {
                ""
            };
            let cls = outer_classes(node, parent, !vertical).attr();
            line(out, depth, &format!("<Separator{orientation}{cls} />"));
        }
        Kit::Select if (1..=2).contains(&texts.len()) && only_text_and_decor(node) => {
            for name in ["Select", "SelectContent", "SelectTrigger", "SelectValue"] {
                ctx.ui(name);
            }
            let cls = outer_classes(node, parent, true).attr();
            line(out, depth, "<Select>");
            line(out, depth + 1, &format!("<SelectTrigger{cls}>"));
            line(
                out,
                depth + 2,
                &format!("<SelectValue placeholder=\"{}\" />", jsx_attr(&texts[0])),
            );
            line(out, depth + 1, "</SelectTrigger>");
            line(out, depth + 1, "<SelectContent />");
            line(out, depth, "</Select>");
        }
        Kit::Slider | Kit::Progress if texts.is_empty() && only_text_and_decor(node) => {
            let percent = fill_percent(node);
            let cls = outer_classes(node, parent, true).attr();
            if kit == Kit::Slider {
                ctx.ui("Slider");
                line(
                    out,
                    depth,
                    &format!("<Slider defaultValue={{[{percent}]}} max={{100}} step={{1}}{cls} />"),
                );
            } else {
                ctx.ui("Progress");
                line(
                    out,
                    depth,
                    &format!("<Progress value={{{percent}}}{cls} />"),
                );
            }
        }
        Kit::Tabs => return super::shadcn_blocks::emit_tabs(ctx, out, node, parent, depth),
        Kit::Breadcrumb => {
            return super::shadcn_blocks::emit_breadcrumb(ctx, out, node, parent, depth)
        }
        Kit::Skeleton => return super::shadcn_blocks::emit_skeleton(ctx, out, node, parent, depth),
        Kit::Table => return super::shadcn_blocks::emit_table(ctx, out, node, parent, depth),
        _ => return false,
    }
    true
}

fn variant_attr(kit: Kit) -> String {
    match kit {
        Kit::Button(Some(variant)) => format!(" variant=\"{variant}\""),
        _ => String::new(),
    }
}

fn emit_labelled_choice(
    ctx: &mut Ctx,
    out: &mut String,
    node: &PenNode,
    kit: Kit,
    parent: Parent,
    depth: usize,
    label: &str,
) {
    let id = dom_id(node);
    ctx.ui("Label");
    let mut classes = outer_classes(node, parent, false);
    let label_line = format!("<Label htmlFor=\"{id}\">{}</Label>", jsx_text(label));
    if kit == Kit::Checkbox {
        ctx.ui("Checkbox");
        let mut row = Classes::default();
        row.push("flex");
        row.push("items-center");
        row.push("gap-2");
        row.extend(classes);
        line(out, depth, &format!("<div{}>", row.attr()));
        line(out, depth + 1, &format!("<Checkbox id=\"{id}\" />"));
        line(out, depth + 1, &label_line);
        line(out, depth, "</div>");
        return;
    }
    ctx.ui("RadioGroup");
    ctx.ui("RadioGroupItem");
    let value = "option-1";
    line(
        out,
        depth,
        &format!("<RadioGroup defaultValue=\"{value}\"{}>", classes.attr()),
    );
    classes = Classes::default();
    classes.push("flex");
    classes.push("items-center");
    classes.push("gap-2");
    line(out, depth + 1, &format!("<div{}>", classes.attr()));
    line(
        out,
        depth + 2,
        &format!("<RadioGroupItem value=\"{value}\" id=\"{id}\" />"),
    );
    line(out, depth + 2, &label_line);
    line(out, depth + 1, "</div>");
    line(out, depth, "</RadioGroup>");
}

fn emit_stats_card(
    ctx: &mut Ctx,
    out: &mut String,
    node: &PenNode,
    parent: Parent,
    depth: usize,
    texts: &[String],
) {
    for name in [
        "Card",
        "CardContent",
        "CardDescription",
        "CardHeader",
        "CardTitle",
    ] {
        ctx.ui(name);
    }
    ctx.tokens.insert("muted-foreground".into());
    let cls = outer_classes(node, parent, true).attr();
    line(out, depth, &format!("<Card{cls}>"));
    line(out, depth + 1, "<CardHeader>");
    line(
        out,
        depth + 2,
        &format!("<CardDescription>{}</CardDescription>", jsx_text(&texts[0])),
    );
    line(
        out,
        depth + 2,
        &format!(
            "<CardTitle className=\"text-2xl\">{}</CardTitle>",
            jsx_text(&texts[1])
        ),
    );
    line(out, depth + 1, "</CardHeader>");
    line(out, depth + 1, "<CardContent>");
    line(
        out,
        depth + 2,
        &format!(
            "<p className=\"text-xs text-muted-foreground\">{}</p>",
            jsx_text(&texts[2])
        ),
    );
    line(out, depth + 1, "</CardContent>");
    line(out, depth, "</Card>");
}

/// Filled share (0..=100) of a slider / progress drawing: the widest
/// rectangle is the track (or the node itself), the narrowest the fill.
fn fill_percent(node: &PenNode) -> i64 {
    let track = node.width_px().unwrap_or(0.0);
    let rects: Vec<f64> = visible_children(node)
        .filter(|child| matches!(child, PenNode::Rectangle(_)))
        .filter_map(PenNodeExt::width_px)
        .collect();
    let (track, fill) = match rects.as_slice() {
        [] => return 50,
        [fill] => (track, *fill),
        many => {
            let max = many.iter().copied().fold(f64::MIN, f64::max);
            let min = many.iter().copied().fold(f64::MAX, f64::min);
            (max, min)
        }
    };
    if track <= 0.0 {
        return 50;
    }
    ((fill / track) * 100.0).round().clamp(0.0, 100.0) as i64
}
