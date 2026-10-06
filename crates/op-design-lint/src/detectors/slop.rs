//! "AI slop" detectors — report-only rules derived from open-design's
//! `impeccable-design-polish` "AI tells" list (openpencil-docs
//! `generation/2026-09-06-mobile-expression-layer-sources.md` §七).
//!
//! Three detectors, each a pure walk over the `PenNode` tree, each emitting
//! `IssueSeverity::Warning` with `suggested_value: null` — there is no safe
//! auto-fix for a design-taste finding, so nothing here ever mutates:
//!
//! - `slop/purple-glow-gradient` — a large frame/rectangle filled with a
//!   saturated purple-blue gradient wash (the AI-default look).
//! - `slop/three-card-feature-row` — the generic row of exactly three
//!   structurally identical icon + text cards.
//! - `slop/rounded-card-wall` — a screen tiled with large-radius rounded
//!   cards instead of letting content sit on the page surface.

use jian_ops_schema::node::container::LayoutMode;
use jian_ops_schema::node::{CornerRadius, PenNode};
use jian_ops_schema::style::{GradientStop, PenFill};
use jian_ops_schema::PenDocument;
use serde_json::Value;

use crate::color::{hsl, parse_hex_color};
use crate::issue::{FixProperty, Issue, IssueCategory, IssueSeverity};
use crate::node_util::{
    children, default_theme, is_mobile_screen_chrome, is_node_visible, json_number, node_fills,
    node_id, node_kind, node_kind_str, numeric_height, numeric_width, resolve_color_ref, NodeKind,
    Theme, Variables,
};

/// Hue window (degrees) that reads as the purple-blue AI default. Violet
/// (#8B5CF6, hue ≈ 258°) sits in the middle; cyan (~190°) and blue (~217°)
/// stay outside.
const PURPLE_HUE_MIN: f64 = 235.0;
const PURPLE_HUE_MAX: f64 = 300.0;
/// Saturation floor — a desaturated lavender wash is a tint, not a glow.
const PURPLE_MIN_SATURATION: f64 = 0.45;
/// Share of the root's authored area past which the wash dominates the board.
const PURPLE_MIN_AREA_SHARE: f64 = 0.25;
/// Edge limit for an image to count as an icon in a feature card.
const CARD_ICON_MAX_EDGE: f64 = 64.0;
/// Corner radius (uniform, or smallest per-corner) past which a painted frame
/// reads as a "rounded card".
const CARD_WALL_MIN_RADIUS: f64 = 16.0;
/// Rounded-card count past which a screen is a wall of cards.
const CARD_WALL_MIN_CARDS: usize = 6;
/// Summed authored card area past which the wall covers the screen.
const CARD_WALL_MIN_AREA_SHARE: f64 = 0.60;

// ── slop/purple-glow-gradient ────────────────────────────────────────────────

/// Flag frames/rectangles whose fill is a linear or radial gradient with
/// `>= 2` resolvable solid stops, EVERY resolvable stop inside the purple-blue
/// hue window at saturation `>= 0.45`, AND the node covering `>= 25%` of the
/// root's authored area. Stops whose color cannot be resolved to a hex value
/// (`$--var` refs missing from the document table, named colors) are skipped,
/// not disqualifying. Nodes without authored numeric width/height are skipped.
pub fn detect_purple_glow_gradient(root: &PenNode, doc: &PenDocument) -> Vec<Issue> {
    let mut issues = Vec::new();
    let Some(root_area) = authored_area(root) else {
        return issues;
    };
    if root_area <= 0.0 {
        return issues;
    }
    let empty_vars = Variables::new();
    let variables = doc.variables.as_ref().unwrap_or(&empty_vars);
    let theme = default_theme(doc.themes.as_ref());
    walk_purple_glow(root, root_area, variables, &theme, &mut issues);
    issues
}

fn walk_purple_glow(
    node: &PenNode,
    root_area: f64,
    variables: &Variables,
    theme: &Theme,
    issues: &mut Vec<Issue>,
) {
    if !is_node_visible(node) {
        return;
    }
    if matches!(node_kind(node), NodeKind::Frame | NodeKind::Rectangle) {
        if let Some(share) = purple_glow_share(node, root_area, variables, theme) {
            let percent = (share * 100.0).round() as i64;
            issues.push(Issue {
                node_id: node_id(node).to_string(),
                category: IssueCategory::SlopPurpleGlowGradient,
                severity: IssueSeverity::Warning,
                property: FixProperty::Fill,
                current_value: json_number(percent as f64),
                suggested_value: Value::Null,
                reason: format!(
                    "purple-blue gradient wash covering {percent}% of the board — the AI-default look; use the style guide's accent on one element instead"
                ),
            });
        }
    }
    for child in children(node) {
        walk_purple_glow(child, root_area, variables, theme, issues);
    }
}

/// The node's share of the root area when it carries a purple-glow gradient,
/// else `None`. The area gate runs first so a small purple accent (a button, a
/// badge) never flags.
fn purple_glow_share(
    node: &PenNode,
    root_area: f64,
    variables: &Variables,
    theme: &Theme,
) -> Option<f64> {
    let share = authored_area(node)? / root_area;
    if share < PURPLE_MIN_AREA_SHARE {
        return None;
    }
    let fills = node_fills(node)?;
    let is_wash = fills.iter().any(|fill| match fill {
        PenFill::LinearGradient(body) => stops_all_purple(&body.stops, variables, theme),
        PenFill::RadialGradient(body) => stops_all_purple(&body.stops, variables, theme),
        _ => false,
    });
    is_wash.then_some(share)
}

/// True when at least two stops resolve to solid hex colors and EVERY
/// resolvable stop sits in the purple-blue hue window with enough saturation.
/// A stop that does not resolve (unresolvable `$--var` ref, non-hex color)
/// is skipped — it says nothing about the wash either way.
fn stops_all_purple(stops: &[GradientStop], variables: &Variables, theme: &Theme) -> bool {
    let mut evaluated = 0usize;
    for stop in stops {
        let Some(raw) = resolve_color_ref(&stop.color, variables, theme) else {
            continue;
        };
        let Some(rgb) = parse_hex_color(&raw) else {
            continue;
        };
        let (hue, saturation, _) = hsl(rgb);
        if !(PURPLE_HUE_MIN..=PURPLE_HUE_MAX).contains(&hue) || saturation < PURPLE_MIN_SATURATION {
            return false;
        }
        evaluated += 1;
    }
    evaluated >= 2
}

// ── slop/three-card-feature-row ──────────────────────────────────────────────

/// Flag a horizontal-layout parent with EXACTLY 3 children that share one
/// structural signature (node kind + ordered child kinds; names ignored),
/// where each child is a frame containing, in order, an `icon_font` or small
/// image (both edges `<= 64`), then 1–2 text leaves. Rows of 2 or 4+ are not
/// the tell, and rows inside a bottom tab bar / status bar are chrome — the
/// icon+label tab is the legitimate version of exactly this shape.
pub fn detect_three_card_feature_row(root: &PenNode) -> Vec<Issue> {
    let mut issues = Vec::new();
    walk_card_rows(root, &mut issues);
    issues
}

fn walk_card_rows(node: &PenNode, issues: &mut Vec<Issue>) {
    if !is_node_visible(node) || is_mobile_screen_chrome(node) {
        return;
    }
    if is_horizontal_layout(node) {
        let kids = children(node);
        if kids.len() == 3 && kids.iter().all(is_feature_card) && share_one_signature(kids) {
            issues.push(Issue {
                node_id: node_id(node).to_string(),
                category: IssueCategory::SlopThreeCardFeatureRow,
                severity: IssueSeverity::Warning,
                property: FixProperty::Layout,
                current_value: Value::from(structural_signature(&kids[0])),
                suggested_value: Value::Null,
                reason:
                    "generic three-card feature row; vary weight (one lead card + two supporting) or use a list"
                        .to_string(),
            });
        }
    }
    for child in children(node) {
        walk_card_rows(child, issues);
    }
}

/// True for Frame / Group / Rectangle with `layout: horizontal`.
fn is_horizontal_layout(node: &PenNode) -> bool {
    let layout = match node {
        PenNode::Frame(n) => &n.container.layout,
        PenNode::Group(n) => &n.container.layout,
        PenNode::Rectangle(n) => &n.container.layout,
        _ => return false,
    };
    *layout == Some(LayoutMode::Horizontal)
}

/// A feature card: a frame whose children are, in order, an icon (an
/// `icon_font`, or an image with both authored edges `<= 64`) followed by
/// 1–2 text leaves.
fn is_feature_card(node: &PenNode) -> bool {
    if node_kind(node) != NodeKind::Frame {
        return false;
    }
    let kids = children(node);
    if !(2..=3).contains(&kids.len()) {
        return false;
    }
    if !is_card_icon(&kids[0]) {
        return false;
    }
    kids[1..].iter().all(|kid| node_kind(kid) == NodeKind::Text)
}

fn is_card_icon(node: &PenNode) -> bool {
    match node_kind(node) {
        NodeKind::IconFont => true,
        // "Small" needs both authored edges; an image without numeric
        // dimensions cannot be shown to be icon-sized, so it does not count.
        NodeKind::Image => match (numeric_width(node), numeric_height(node)) {
            (Some(width), Some(height)) => {
                width <= CARD_ICON_MAX_EDGE && height <= CARD_ICON_MAX_EDGE
            }
            _ => false,
        },
        _ => false,
    }
}

/// True when all siblings share one structural signature.
fn share_one_signature(kids: &[PenNode]) -> bool {
    let Some(first) = kids.first() else {
        return false;
    };
    let signature = structural_signature(first);
    kids.iter()
        .skip(1)
        .all(|kid| structural_signature(kid) == signature)
}

/// Structural signature of a node: its kind plus the ordered kinds of its
/// direct children (`frame(icon_font,text,text)`). Names and ids are
/// intentionally ignored — the signature describes structure only.
fn structural_signature(node: &PenNode) -> String {
    let kid_kinds: Vec<&str> = children(node).iter().map(node_kind_str).collect();
    format!("{}({})", node_kind_str(node), kid_kinds.join(","))
}

// ── slop/rounded-card-wall ───────────────────────────────────────────────────

/// Flag a root whose painted frames with `cornerRadius >= 16` (uniform, or
/// smallest per-corner) number `>= 6` AND whose summed authored area covers
/// `>= 60%` of the root's authored area. Emits ONE issue on the root. A root
/// without authored numeric dimensions cannot be measured and is skipped.
pub fn detect_rounded_card_wall(root: &PenNode) -> Vec<Issue> {
    let Some(root_area) = authored_area(root) else {
        return Vec::new();
    };
    if root_area <= 0.0 {
        return Vec::new();
    }
    let mut cards = 0usize;
    let mut card_area = 0.0;
    for child in children(root) {
        collect_rounded_cards(child, &mut cards, &mut card_area);
    }
    if cards < CARD_WALL_MIN_CARDS {
        return Vec::new();
    }
    let share = card_area / root_area;
    if share < CARD_WALL_MIN_AREA_SHARE {
        return Vec::new();
    }
    let percent = (share * 100.0).round() as i64;
    vec![Issue {
        node_id: node_id(root).to_string(),
        category: IssueCategory::SlopRoundedCardWall,
        severity: IssueSeverity::Warning,
        property: FixProperty::CornerRadius,
        current_value: json_number(cards as f64),
        suggested_value: Value::Null,
        reason: format!(
            "{cards} rounded cards cover {percent}% of the screen; let some content sit on the page surface"
        ),
    }]
}

/// Count painted, large-radius frames in the subtree and sum their authored
/// areas. Cards without authored numeric dimensions still count toward the
/// card total but contribute no area.
fn collect_rounded_cards(node: &PenNode, cards: &mut usize, card_area: &mut f64) {
    if !is_node_visible(node) {
        return;
    }
    if node_kind(node) == NodeKind::Frame && is_painted(node) {
        if let Some(radius) = min_corner_radius(node) {
            if radius >= CARD_WALL_MIN_RADIUS {
                *cards += 1;
                *card_area += authored_area(node).unwrap_or(0.0);
            }
        }
    }
    for child in children(node) {
        collect_rounded_cards(child, cards, card_area);
    }
}

/// True when the node has at least one fill that is not fully transparent
/// (a fill-level `opacity: 0`, or a `#RRGGBBAA` solid with alpha `00`, is
/// not painted).
fn is_painted(node: &PenNode) -> bool {
    node_fills(node).is_some_and(|fills| {
        fills.iter().any(|fill| match fill {
            PenFill::Solid(body) => {
                body.opacity.unwrap_or(1.0) > 0.0 && !is_transparent_hex(&body.color)
            }
            PenFill::LinearGradient(body) => body.opacity.unwrap_or(1.0) > 0.0,
            PenFill::RadialGradient(body) => body.opacity.unwrap_or(1.0) > 0.0,
            PenFill::MeshGradient(body) => body.opacity.unwrap_or(1.0) > 0.0,
            PenFill::Shader(body) => body.opacity.unwrap_or(1.0) > 0.0,
            PenFill::Image(body) => body.opacity.unwrap_or(1.0) > 0.0,
        })
    })
}

/// True for a 9-char `#RRGGBBAA` hex whose alpha pair is `00`.
fn is_transparent_hex(color: &str) -> bool {
    color.len() == 9 && color[7..].eq_ignore_ascii_case("00")
}

/// The effective corner radius of a frame for the card-wall rule: the uniform
/// value, or the smallest of the four per-corner values. Non-frames yield
/// `None`.
fn min_corner_radius(node: &PenNode) -> Option<f64> {
    let radius = match node {
        PenNode::Frame(n) => n.container.corner_radius.as_ref(),
        _ => None,
    }?;
    match radius {
        CornerRadius::Uniform(value) => Some(*value),
        CornerRadius::PerCorner(corners) => corners.iter().copied().reduce(f64::min),
    }
}

// ── shared helpers ───────────────────────────────────────────────────────────

/// A node's authored area from its numeric width/height, or `None` when
/// either dimension is not a plain number (`fill_container` / `hug` / a
/// `$variable` expression cannot be measured here).
fn authored_area(node: &PenNode) -> Option<f64> {
    Some(numeric_width(node)? * numeric_height(node)?)
}
