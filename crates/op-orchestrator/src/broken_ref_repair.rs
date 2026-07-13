//! A reference to a variable that does not exist must not render as an
//! invisible glyph.
//!
//! Measured (test0711-1-ds, 2026-07-12): the search bar's filter button was a
//! soft `#EA580C15` tint of the brand orange, and its glyph was painted
//! `$--white` — a token that is in no design system's table. The reference
//! resolved to nothing, the icon rendered as bare white on a near-white tint,
//! and the button read as empty. The model invented the token; the pipeline
//! then hid the consequence instead of surfacing it.
//!
//! The repair is a contract, not a guess: a glyph must be visible against the
//! surface it sits on, and the surface itself tells us which colour that is.
//! - A glyph on a soft (alpha) tint of a colour takes that colour, opaque —
//!   the standard tinted-icon-button pairing.
//! - A glyph on a `$--token` surface takes that token's `-foreground` partner
//!   when the table defines one.
//! - Otherwise it falls back to the document's primary text colour.
//! - A CONTAINER's broken fill is simply dropped: an unknown surface colour is
//!   not worth guessing, and transparent at least shows what is behind it.

use std::collections::BTreeSet;

use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::Value;

use crate::types::DocSink;

/// Text colours to try, in order, when the surface offers no better answer.
const FOREGROUND_FALLBACKS: [&str; 3] = ["--foreground", "--card-foreground", "--primary"];

pub(crate) fn repair_broken_variable_refs(sink: &mut dyn DocSink) {
    let known: BTreeSet<String> = sink
        .state()
        .doc
        .variables
        .as_ref()
        .map(|vars| vars.keys().cloned().collect())
        .unwrap_or_default();
    if known.is_empty() {
        // No variable table at all: every `$ref` is equally unresolvable, and
        // the design was never token-based. Nothing to reason from.
        return;
    }
    let patches: Vec<(NodeId, String)> = {
        let mut out = Vec::new();
        for root in sink.state().active_children() {
            collect(root, None, &known, &mut out);
        }
        out
    };
    for (node_id, patch_json) in patches {
        sink.apply(EditorCommand::PatchNodeData {
            node_id,
            patch_json,
            page_id: None,
        });
    }
}

fn collect(
    node: &PenNode,
    parent_fill: Option<&str>,
    known: &BTreeSet<String>,
    out: &mut Vec<(NodeId, String)>,
) {
    if let Some(reference) = broken_ref(node, known) {
        let is_glyph = matches!(node, PenNode::Text(_) | PenNode::IconFont(_));
        let replacement = if is_glyph {
            Some(readable_glyph_color(parent_fill, known))
        } else {
            None
        };
        let patch = match replacement {
            Some(color) => format!(r#"{{"fill":[{{"type":"solid","color":"{color}"}}]}}"#),
            None => r#"{"fill":[]}"#.to_string(),
        };
        let _ = reference;
        out.push((NodeId::new(node.id_str().to_string()), patch));
    }
    let own_fill = solid_fill_color(node);
    for child in node.children().into_iter().flatten() {
        collect(child, own_fill.as_deref().or(parent_fill), known, out);
    }
}

/// The node's own solid fill colour, whatever form it takes.
fn solid_fill_color(node: &PenNode) -> Option<String> {
    let value = serde_json::to_value(node).ok()?;
    first_solid_color(&value)
}

fn first_solid_color(value: &Value) -> Option<String> {
    value
        .get("fill")?
        .as_array()?
        .iter()
        .find_map(|fill| fill.get("color")?.as_str().map(str::to_string))
}

/// The `$name` this node paints with, when that name is in no table.
fn broken_ref(node: &PenNode, known: &BTreeSet<String>) -> Option<String> {
    let color = solid_fill_color(node)?;
    let name = color.strip_prefix('$')?;
    // A name the table knows, or one the built-in semantic palette can still
    // answer, resolves fine — leave it alone.
    if known.contains(name) || op_editor_core::variables_resolve::has_palette_fallback(name) {
        return None;
    }
    Some(name.to_string())
}

/// What a glyph on `surface` must be painted to be legible.
fn readable_glyph_color(surface: Option<&str>, known: &BTreeSet<String>) -> String {
    if let Some(surface) = surface {
        // A soft tint (#RRGGBBAA) of a colour: the glyph is that colour, opaque.
        if let Some(base) = opaque_base_of_tint(surface) {
            return base;
        }
        // A token surface: its `-foreground` partner, when the table has one.
        if let Some(name) = surface.strip_prefix('$') {
            let partner = format!("{name}-foreground");
            if known.contains(&partner) {
                return format!("${partner}");
            }
        }
    }
    for token in FOREGROUND_FALLBACKS {
        if known.contains(token) {
            return format!("${token}");
        }
    }
    "#111111".to_string()
}

/// `#EA580C15` → `#EA580C` — an 8-digit hex whose alpha is LOW is a soft tint,
/// and the glyph pairing for a soft tint is the tint's own colour at full
/// strength. A fully opaque 8-digit hex is just a colour; nothing to derive.
fn opaque_base_of_tint(color: &str) -> Option<String> {
    let hex = color.strip_prefix('#')?;
    if hex.len() != 8 {
        return None;
    }
    let alpha = u8::from_str_radix(&hex[6..8], 16).ok()?;
    if alpha >= 0xE0 {
        return None;
    }
    Some(format!("#{}", &hex[..6]))
}

#[cfg(test)]
#[path = "broken_ref_repair_tests.rs"]
mod tests;
