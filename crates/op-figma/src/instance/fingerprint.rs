//! Fingerprint-based assignment of virtual override entries onto
//! symbol-subtree nodes.
//!
//! Figma's virtual-GUID allocation order is edit-history dependent and
//! NOT recoverable from the tree (verified against real files where
//! parents number AFTER children), so any fixed walk order mis-keys
//! entries on some files. Instead, entries that carry discriminating
//! data — sizes, transforms, text content, fill overrides — are scored
//! against every candidate node and assigned greedily. Entries with no
//! discriminating signal fall back to the caller's walk-order mapping.

use crate::figma_types::FigVec2;
use crate::kiwi::FigValue;
use crate::tree::{guid_to_string, TreeNode};
use std::collections::{HashMap, HashSet};

/// One single-segment virtual entry: the merged view of its derived
/// data + override props.
pub(super) struct VirtualEntry<'a> {
    pub pk: String,
    /// `localID - firstLocalID` — walk-order proximity prior.
    pub rel_idx: f64,
    pub derived: Option<&'a FigValue>,
    pub overrides: Option<&'a FigValue>,
}

/// Keys that mark an entry as text-targeted.
const TEXT_DEMAND_KEYS: &[&str] = &[
    "textData",
    "derivedTextData",
    "fontSize",
    "styleIdForText",
    "fontName",
    "textAlignHorizontal",
    "textAlignVertical",
];

fn any_key(entry: Option<&FigValue>, keys: &[&str]) -> bool {
    let Some(FigValue::Object(pairs)) = entry else {
        return false;
    };
    pairs
        .iter()
        .any(|(k, v)| keys.contains(&k.as_str()) && !matches!(v, FigValue::Null))
}

pub(super) fn demands_text(e: &VirtualEntry) -> bool {
    any_key(e.derived, TEXT_DEMAND_KEYS) || any_key(e.overrides, TEXT_DEMAND_KEYS)
}

fn entry_size(e: &VirtualEntry) -> Option<FigVec2> {
    e.derived
        .and_then(|d| d.get("size"))
        .and_then(FigVec2::from_value)
}

fn entry_translation(e: &VirtualEntry) -> Option<(f64, f64)> {
    let t = e.derived.and_then(|d| d.get("transform"))?;
    Some((t.get_f64("m02")?, t.get_f64("m12")?))
}

fn entry_characters<'a>(e: &'a VirtualEntry) -> Option<&'a str> {
    if let Some(c) = e
        .derived
        .and_then(|d| d.get("derivedTextData"))
        .and_then(|t| t.get_str("characters"))
    {
        return Some(c);
    }
    e.overrides
        .and_then(|o| o.get("textData"))
        .and_then(|t| t.get_str("characters"))
}

/// How discriminating an entry's fill override is. IMAGE paints and
/// rare (low-opacity) solids identify their target on their own;
/// ordinary fills only nudge the score.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum FillHint {
    /// Override carries an IMAGE paint — only image-filled nodes
    /// (thumbnails, avatars) receive image overrides.
    Image,
    /// Override carries a translucent SOLID (opacity < 0.9) — a
    /// status-chip-style tint that matches its node's authored
    /// opacity.
    RareSolid(f64),
    /// Any other paint — presence is weak evidence.
    Plain,
}

pub(super) fn fill_hint(e: &VirtualEntry) -> Option<FillHint> {
    let paints = e.overrides.and_then(|o| o.get_array("fillPaints"))?;
    let first = paints.first()?;
    match first.get_str("type") {
        Some("IMAGE") => Some(FillHint::Image),
        Some("SOLID") => {
            let op = first.get_f64("opacity").unwrap_or(1.0);
            if op < 0.9 {
                Some(FillHint::RareSolid(op))
            } else {
                Some(FillHint::Plain)
            }
        }
        Some(_) => Some(FillHint::Plain),
        None => None,
    }
}

/// Whether this entry carries data the scorer can actually GROUND an
/// assignment on: a size, a translation, text characters, or a
/// STRONG fill hint (image / rare solid). Entries without one of
/// these (bare `name` / `visible` / `fontSize` / plain-fill tweaks)
/// can never clear [`ASSIGN_THRESHOLD`], so the caller must route
/// them through the walk-order fallback instead — otherwise they'd
/// silently vanish. Plain fills and text-key demands still
/// contribute to SCORING, just not to routing.
pub(super) fn has_signal(e: &VirtualEntry) -> bool {
    has_hard_signal(e) || matches!(fill_hint(e), Some(FillHint::Image | FillHint::RareSolid(_)))
}

/// Signals that positively CONTRADICT candidates on a rejection:
/// size / translation / text content. A rejection of an entry whose
/// only signal is a fill hint means the hint was merely INAPPLICABLE
/// (no candidate carries such a fill) — the caller should walk-order
/// it instead of dropping it.
pub(super) fn has_hard_signal(e: &VirtualEntry) -> bool {
    entry_size(e).is_some() || entry_translation(e).is_some() || entry_characters(e).is_some()
}

/// Coarse content class for text matching: plain prose (0), numeric
/// "1,250" (1), percent/delta "+15.80%" (2), or currency
/// "₦730,000.00 x 1" (3). Currency is checked first so an embedded
/// unit letter (the "x 1") doesn't demote it to plain.
fn text_class(s: &str) -> u8 {
    let t = s.trim();
    // Unicode currency-symbol category (Sc) covers ₦ $ € £ ¥ ₩ ₹ ฿ ₫ …
    if t.chars()
        .any(|c| matches!(c, '\u{20A0}'..='\u{20CF}' | '$' | '£' | '¥' | '¢'))
    {
        return 3;
    }
    if t.contains('%') || t.contains('％') {
        return 2;
    }
    let has_digit = t.chars().any(|c| c.is_numeric());
    let has_alpha = t.chars().any(|c| c.is_alphabetic());
    if has_digit && !has_alpha {
        1
    } else {
        0
    }
}

/// Class-pair affinity: exact class +2.0; the numeric/currency cousin
/// pair still attracts (+1.5 — a "₦4,000,000.00" override routinely
/// lands on a bare "0" placeholder); anything else repels.
fn class_affinity(a: u8, b: u8) -> f64 {
    if a == b {
        2.0
    } else if matches!((a, b), (1, 3) | (3, 1)) {
        1.5
    } else {
        -1.0
    }
}

/// Minimum score an assignment needs — pure walk-order priors (max
/// 0.4) can never clear it; real evidence is required.
const ASSIGN_THRESHOLD: f64 = 1.5;

/// Whether a node's authored fills satisfy a strong hint.
fn hint_matches_node(hint: FillHint, figma: &FigValue) -> bool {
    let node_paints = figma.get_array("fillPaints");
    match hint {
        FillHint::Image => node_paints
            .map(|a| a.iter().any(|p| p.get_str("type") == Some("IMAGE")))
            .unwrap_or(false),
        FillHint::RareSolid(op) => node_paints
            .map(|a| {
                a.iter().any(|p| {
                    p.get_str("type") == Some("SOLID")
                        && (p.get_f64("opacity").unwrap_or(1.0) - op).abs() <= 0.05
                })
            })
            .unwrap_or(false),
        FillHint::Plain => false,
    }
}

/// Whether ANY node in `nodes` satisfies this entry's strong fill
/// hint. Distinguishes the two rejection flavours for a
/// fill-hint-only entry: no match anywhere → the hint was
/// inapplicable (walk-order fallback is safe); a match exists → the
/// entry LOST a conflict and must stay dropped.
pub(super) fn hint_matches_any(e: &VirtualEntry, nodes: &[&TreeNode]) -> bool {
    let Some(hint) = fill_hint(e) else {
        return false;
    };
    nodes.iter().any(|n| hint_matches_node(hint, &n.figma))
}

fn score(e: &VirtualEntry, node: &TreeNode, node_idx: usize, ratios: (f64, f64)) -> Option<f64> {
    let figma = &node.figma;
    let is_text = figma.get_str("type") == Some("TEXT");
    if demands_text(e) && !is_text {
        return None;
    }
    let (rx, ry) = ratios;
    let mut s = 0.0;
    let mut evidence = false;

    if let (Some(ds), Some(ns)) = (
        entry_size(e),
        figma.get("size").and_then(FigVec2::from_value),
    ) {
        let dx = (ds.x - ns.x).abs().min((ds.x - ns.x * rx).abs());
        let dy = (ds.y - ns.y).abs().min((ds.y - ns.y * ry).abs());
        let d = dx + dy;
        s += if d <= 4.0 {
            3.0
        } else if d <= 24.0 {
            1.5
        } else if d > 40.0 && d > 0.6 * (ns.x + ns.y) {
            -3.0
        } else {
            0.0
        };
        evidence = true;
    }

    if let Some((tx, ty)) = entry_translation(e) {
        if let Some(t) = figma.get("transform") {
            if let (Some(ax), Some(ay)) = (t.get_f64("m02"), t.get_f64("m12")) {
                let dx = (tx - ax).abs().min((tx - ax * rx).abs());
                let dy = (ty - ay).abs().min((ty - ay * ry).abs());
                let d = dx + dy;
                s += if d <= 4.0 {
                    3.0
                } else if d <= 8.0 {
                    2.0
                } else if d <= 24.0 {
                    // Instance-side layout drift (wider values push
                    // siblings ~10-20px) still identifies the node —
                    // clears the threshold, but loses to any closer
                    // candidate in the greedy pass.
                    1.5
                } else if (dx <= 2.0 && dy <= 100.0) || (dy <= 2.0 && dx <= 100.0) {
                    // Single-axis drift: justify/space-between moves a
                    // node along one axis while the other matches
                    // exactly — still identifying.
                    1.5
                } else if d > 150.0 {
                    -1.5
                } else {
                    0.0
                };
                evidence = true;
            }
        }
    }

    if let Some(dc) = entry_characters(e) {
        if let Some(nc) = figma.get("textData").and_then(|t| t.get_str("characters")) {
            s += if dc == nc {
                4.0
            } else {
                class_affinity(text_class(dc), text_class(nc))
            };
            evidence = true;
        }
    }

    if let Some(hint) = fill_hint(e) {
        let node_paints = figma.get_array("fillPaints");
        let node_has_fill = node_paints.map(|a| !a.is_empty()).unwrap_or(false);
        s += match hint {
            FillHint::Image => {
                if hint_matches_node(hint, figma) {
                    2.5
                } else {
                    -1.0
                }
            }
            FillHint::RareSolid(_) => {
                if hint_matches_node(hint, figma) {
                    2.0
                } else if node_has_fill {
                    0.0
                } else {
                    -0.25
                }
            }
            FillHint::Plain => {
                if node_has_fill {
                    0.5
                } else {
                    -0.25
                }
            }
        };
        evidence = true;
    }

    if !evidence {
        return None;
    }
    // Small walk-order proximity prior — tie-break only, can't clear
    // the threshold alone.
    s += 0.4 / (1.0 + (e.rel_idx - node_idx as f64).abs());
    Some(s)
}

/// One virtual pk's evidence pooled across EVERY instance of a
/// symbol: all text values that ever targeted it, plus whether any
/// entry demanded a text node.
pub(crate) struct PooledEntry {
    pub pk: String,
    pub rel_idx: f64,
    pub char_values: Vec<String>,
    pub demands_text: bool,
}

/// Assign pooled per-pk text evidence onto symbol nodes. Same greedy
/// scheme as [`assign`], but the char affinity is SUMMED across all
/// instances — one weak first instance can no longer seed a wrong
/// pin. Only entries with at least one text value participate.
pub(crate) fn assign_pooled(
    entries: &[PooledEntry],
    nodes: &[&TreeNode],
) -> HashMap<String, String> {
    if entries.len().saturating_mul(nodes.len()) > MAX_PAIRS {
        return HashMap::new();
    }
    let mut pairs: Vec<(f64, usize, usize)> = Vec::new();
    for (ei, e) in entries.iter().enumerate() {
        if e.char_values.is_empty() {
            continue;
        }
        for (ni, n) in nodes.iter().enumerate() {
            let figma = &n.figma;
            let is_text = figma.get_str("type") == Some("TEXT");
            if e.demands_text && !is_text {
                continue;
            }
            let Some(nc) = figma.get("textData").and_then(|t| t.get_str("characters")) else {
                continue;
            };
            let mut s = 0.0;
            for dc in &e.char_values {
                s += if dc == nc {
                    4.0
                } else {
                    class_affinity(text_class(dc), text_class(nc))
                };
            }
            s += 0.4 / (1.0 + (e.rel_idx - ni as f64).abs());
            if s >= ASSIGN_THRESHOLD {
                pairs.push((s, ei, ni));
            }
        }
    }
    pairs.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.cmp(&b.1))
            .then(a.2.cmp(&b.2))
    });
    let mut taken_entries: HashSet<usize> = HashSet::new();
    let mut taken_nodes: HashSet<usize> = HashSet::new();
    let mut out = HashMap::new();
    for (_, ei, ni) in pairs {
        if taken_entries.contains(&ei) || taken_nodes.contains(&ni) {
            continue;
        }
        let Some(ng) = nodes[ni].figma.get("guid").and_then(guid_to_string) else {
            continue;
        };
        taken_entries.insert(ei);
        taken_nodes.insert(ni);
        out.insert(entries[ei].pk.clone(), ng);
    }
    out
}

/// Pair-count ceiling: beyond this the quadratic scoring pass isn't
/// worth it — the caller must use its walk-order fallback instead.
const MAX_PAIRS: usize = 50_000;

/// Greedy unique assignment of signal-bearing entries onto nodes.
/// Returns `pk → node guid` for every pair whose score clears
/// [`ASSIGN_THRESHOLD`]. Entries absent from the map were rejected
/// because their evidence contradicts every candidate — the caller
/// must DROP them, not walk-order them. `None` means the pair count
/// exceeded [`MAX_PAIRS`] and no scoring ran at all — the caller
/// should route everything through the walk-order fallback.
pub(super) fn assign(
    entries: &[VirtualEntry],
    nodes: &[&TreeNode],
    ratios: (f64, f64),
) -> Option<HashMap<String, String>> {
    if entries.len().saturating_mul(nodes.len()) > MAX_PAIRS {
        return None;
    }
    let mut pairs: Vec<(f64, usize, usize)> = Vec::new();
    for (ei, e) in entries.iter().enumerate() {
        for (ni, n) in nodes.iter().enumerate() {
            if let Some(sc) = score(e, n, ni, ratios) {
                if sc >= ASSIGN_THRESHOLD {
                    pairs.push((sc, ei, ni));
                }
            }
        }
    }
    // Stable order: score desc, then entry order, then node order.
    pairs.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.cmp(&b.1))
            .then(a.2.cmp(&b.2))
    });
    let mut taken_entries: HashSet<usize> = HashSet::new();
    let mut taken_nodes: HashSet<usize> = HashSet::new();
    let mut out = HashMap::new();
    for (_, ei, ni) in pairs {
        if taken_entries.contains(&ei) || taken_nodes.contains(&ni) {
            continue;
        }
        let Some(ng) = nodes[ni].figma.get("guid").and_then(guid_to_string) else {
            continue;
        };
        taken_entries.insert(ei);
        taken_nodes.insert(ni);
        out.insert(entries[ei].pk.clone(), ng);
    }
    Some(out)
}
