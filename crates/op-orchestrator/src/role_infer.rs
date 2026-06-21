//! Name-based semantic role inference — P2 increment **I1**.
//!
//! Port of `inferRoleFromName` + the name-inference path of `resolveNodeRole`
//! and the tree walk of `resolveTreeRoles` from `role-resolver.ts`. This
//! increment ONLY infers `node.role` from the node NAME (when the model didn't
//! emit a role) and writes it back; it does NOT inject role DEFAULTS — that is
//! increment I2 (`role_defaults` / `resolveTreeRoles`'s `applyDefaults`).
//!
//! Setting `role` already has real effect: the existing cleanup passes
//! (`is_nav_surface` / nav-surface repair, section logic in `cleanup.rs`) key
//! off `node.role`, so an inferred `navbar` / `footer` now gets repaired the
//! same way an explicitly-tagged one does. See the porting spec
//! `openpencil-docs/superpowers/specs/2026-06-06-role-resolver-rust-port.md`.

use std::sync::LazyLock;

use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::SizingBehavior;
use op_editor_core::PenNodeExt;
use regex::Regex;

use crate::role_defaults::{apply_role_defaults, node_layout_string, RoleCtx, Theme};

/// Exact (case-insensitive) name → role. Port of `NAME_EXACT_MAP`.
fn exact_role(lower: &str) -> Option<&'static str> {
    Some(match lower {
        "bottom nav" | "bottom navigation" | "bottom tab" | "bottom tab bar" | "bottom-tab-bar" => {
            "bottom-tab-bar"
        }
        "tab bar" | "tabbar" | "tab-bar" => "tab-bar",
        "navbar" | "navigation" | "navigation bar" | "nav bar" | "nav" | "header" | "top bar"
        | "topbar" => "navbar",
        "hero" | "hero section" => "hero",
        "footer" => "footer",
        "search bar" | "searchbar" | "search input" | "search" => "search-bar",
        "avatar" => "avatar",
        "divider" | "separator" => "divider",
        "spacer" => "spacer",
        "badge" => "badge",
        "tag" => "tag",
        "pill" => "pill",
        "table" => "table",
        _ => return None,
    })
}

/// Names that mark a node as a CONTAINER rather than an instance of a role
/// (e.g. "Button Group", "Card List"). Port of `CONTAINER_SUFFIXES`.
static CONTAINER_SUFFIXES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(group|row|container|wrapper|section|list|area|stack|grid|bar)s?\b").unwrap()
});

/// First word after a role keyword that turns the node into a PART of the role
/// ("Card Header", "Button Label"). Port of `ROLE_PART_WORDS`.
const ROLE_PART_WORDS: &[&str] = &[
    "header",
    "body",
    "footer",
    "title",
    "subtitle",
    "content",
    "wrapper",
    "container",
    "area",
    "label",
    "value",
    "caption",
    "description",
    "image",
    "media",
    "icon",
    "action",
    "actions",
    "meta",
    "row",
    "column",
    "stack",
    "grid",
];

/// Substring patterns → role, checked in order (first match wins). The `bool`
/// is `skipContainers`. Port of `NAME_PATTERN_MAP`.
static NAME_PATTERN_MAP: LazyLock<Vec<(Regex, &'static str, bool)>> = LazyLock::new(|| {
    vec![
        (
            Regex::new(r"\bbottom\s*(nav|navigation|tab)|bottom[-\s]*tab").unwrap(),
            "bottom-tab-bar",
            false,
        ),
        (
            Regex::new(r"\btab\s*bar\b|\btabbar\b").unwrap(),
            "tab-bar",
            false,
        ),
        (Regex::new(r"\bbtn\b|\bbutton\b").unwrap(), "button", true),
        (Regex::new(r"\bcard\b").unwrap(), "card", true),
        (
            Regex::new(r"\binput\b|text\s*field|text\s*box").unwrap(),
            "input",
            false,
        ),
        (Regex::new(r"\bform\b").unwrap(), "form-group", false),
        (Regex::new(r"\bsearch").unwrap(), "search-bar", false),
        (Regex::new(r"\bnav\s*link").unwrap(), "nav-link", false),
        (Regex::new(r"\bstat").unwrap(), "stat-card", true),
        (Regex::new(r"\bpricing").unwrap(), "pricing-card", true),
        (
            Regex::new(r"\btestimonial\b|\breview\b|\bquote\b").unwrap(),
            "testimonial",
            false,
        ),
        (
            Regex::new(r"\bcta\b|call\s*to\s*action").unwrap(),
            "cta-section",
            false,
        ),
        (Regex::new(r"\bfeature").unwrap(), "feature-card", true),
        (Regex::new(r"\bicon\b").unwrap(), "icon", false),
    ]
});

/// First alphabetic token (≥2 chars), lowercased. Port of `firstWordToken` —
/// skips numeric indices so "Card 1 Content" surfaces "content".
fn first_word_token(s: &str) -> Option<String> {
    static WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[a-zA-Z]{2,}").unwrap());
    WORD.find(s).map(|m| m.as_str().to_lowercase())
}

/// Infer a role from a frame node's NAME. Frames only (text/path/image don't
/// need role inference). Port of `inferRoleFromName`.
pub fn infer_role_from_name(node: &PenNode) -> Option<&'static str> {
    if !matches!(node, PenNode::Frame(_)) {
        return None;
    }
    let name = node.base().name.as_deref()?;
    let lowered = name.to_lowercase();
    let lower = lowered.trim();

    if let Some(role) = exact_role(lower) {
        return Some(role);
    }

    for (pattern, role, skip_containers) in NAME_PATTERN_MAP.iter() {
        let Some(m) = pattern.find(lower) else {
            continue;
        };
        if *skip_containers {
            // "Button Group" / "Card List" → a container, not an instance.
            if CONTAINER_SUFFIXES.is_match(lower) {
                continue;
            }
            // "Card Header" / "Button Label" → a PART of the role. Only the
            // FIRST word AFTER the keyword is checked, so "Card with Icon"
            // (prepositional) keeps its role.
            let after = &lower[m.end()..];
            if let Some(next) = first_word_token(after) {
                if ROLE_PART_WORDS.contains(&next.as_str()) {
                    continue;
                }
            }
        }
        return Some(role);
    }

    None
}

const CARD_LIKE_ROLES: &[&str] = &[
    "card",
    "stat-card",
    "pricing-card",
    "feature-card",
    "image-card",
    "testimonial",
];
const PAGE_CHROME_ROLES: &[&str] = &["navbar", "footer", "hero", "cta-section"];
const CARD_LIKE_MIN_DIMENSION: f64 = 40.0;

fn declared_pixel(behavior: &Option<SizingBehavior>) -> Option<f64> {
    match behavior {
        Some(SizingBehavior::Number(n)) if n.is_finite() => Some(*n),
        _ => None,
    }
}

/// A node too small to plausibly be a card. Only rejects when BOTH dimensions
/// are declared numbers and at least one is below the threshold. Port of
/// `isAbsurdlyTinyForCardRole`.
fn is_absurdly_tiny_for_card_role(node: &PenNode) -> bool {
    let PenNode::Frame(frame) = node else {
        return false;
    };
    match (
        declared_pixel(&frame.container.width),
        declared_pixel(&frame.container.height),
    ) {
        (Some(w), Some(h)) => w < CARD_LIKE_MIN_DIMENSION || h < CARD_LIKE_MIN_DIMENSION,
        _ => false,
    }
}

/// Infer + write back a single node's role (I1 — no defaults injection).
///
/// An explicit author role always wins and is left untouched. For an inferred
/// role we apply the two TS guards before writing it:
/// - page-chrome inference (`navbar`/`footer`/`hero`/`cta-section`) is dropped
///   inside a card-family parent — a card's inner "Header" is a card piece, not
///   page chrome (and would otherwise get a navbar fill from cleanup).
/// - a card-family role on an absurdly tiny node ("Status Dot" → stat-card) is
///   dropped.
fn resolve_node_role(node: &mut PenNode, parent_role: Option<&str>) {
    // Effective role: an explicit (AI-emitted) role wins; otherwise infer one
    // from the node name.
    let mut role: Option<String> = node.base().role.clone();
    if role.is_none() {
        if let Some(inferred) = infer_role_from_name(node) {
            // The page-chrome-in-card guard applies ONLY to INFERRED roles (TS
            // resolveNodeRole runs it inside the `if (!role)` branch): a card's
            // inner "Header" must not be inferred as a navbar.
            let drop_page_chrome = PAGE_CHROME_ROLES.contains(&inferred)
                && parent_role
                    .map(|p| CARD_LIKE_ROLES.contains(&p))
                    .unwrap_or(false);
            if !drop_page_chrome {
                role = Some(inferred.to_string());
            }
        }
    }

    let Some(role_str) = role else {
        return;
    };

    // Tiny-card guard — applies to BOTH explicit and inferred roles, matching
    // the post-inference guard in TS resolveNodeRole (which `delete`s the role).
    // A card-family role on a node too small to be a card (a name-inferred
    // "Status Dot" → stat-card, OR an LLM-emitted `role:"card"` on a 6×6 frame)
    // is STRIPPED so the I2 defaults pass injects no card padding/shadow onto a
    // dot. (Codex review 2026-06-06.)
    if CARD_LIKE_ROLES.contains(&role_str.as_str()) && is_absurdly_tiny_for_card_role(node) {
        node.base_mut().role = None;
        return;
    }

    node.base_mut().role = Some(role_str);
}

/// Walk the tree depth-first: infer + write the role (I1), then inject the
/// role's defaults (I2). Threads the [`RoleCtx`] down so the page-chrome-in-card
/// guard and the parent-aware / theme-aware / canvas-width-aware defaults see
/// the right context. Port of `resolveTreeRoles`.
pub fn resolve_tree_roles(node: &mut PenNode, ctx: &RoleCtx) {
    resolve_node_role(node, ctx.parent_role.as_deref());
    if let Some(role) = node.base().role.clone() {
        apply_role_defaults(node, &role, ctx);
    }
    // The defaults pass may have just set this node's layout, so read it AFTER.
    let child_ctx = RoleCtx {
        parent_role: node.base().role.clone(),
        parent_layout: node_layout_string(node),
        canvas_width: ctx.canvas_width,
        theme: ctx.theme,
    };
    if let Some(children) = node.children_mut() {
        for child in children.iter_mut() {
            resolve_tree_roles(child, &child_ctx);
        }
    }
}

/// Resolve a forest of section roots (a sub-agent subtree). Each root's parent
/// is the page, whose role we don't track here, so `parent_role = None`.
/// `canvas_width` + `theme` come from the plan's root frame.
pub fn resolve_forest_roles(nodes: &mut [PenNode], canvas_width: f64, theme: Theme) {
    let ctx = RoleCtx::root(canvas_width, theme);
    for node in nodes.iter_mut() {
        resolve_tree_roles(node, &ctx);
    }
}

#[cfg(test)]
#[path = "role_infer_tests.rs"]
mod tests;
