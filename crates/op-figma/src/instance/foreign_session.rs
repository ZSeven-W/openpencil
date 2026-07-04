//! Foreign-session virtual entries — resolution by validated subtree
//! anchoring, with a uniform-family fallback for identical cosmetic
//! overrides.
//!
//! An instance's override entries usually share one virtual session
//! (the base). Files with nested library components carry EXTRA
//! sessions whose numbering is anchored at some SUBTREE of the symbol
//! (that component's own history space). The anchor is not encoded, so
//! it is searched: every (node, self|children) walk is tried, and one
//! is trusted only when it is the UNIQUE walk under which every
//! text-demanding entry lands on a TEXT node and every nested-head
//! entry lands on an INSTANCE node. Groups with no such unique anchor
//! fall back to [`resolve_uniform_family`]; failing that, dropped.

use super::{local_id, walk_virtual, TreeNode};
use crate::kiwi::FigValue;
use std::collections::HashMap;

/// One walk-missed pk of a foreign session and what its entry demands
/// of the node it maps to.
#[derive(Clone)]
pub(super) struct ForeignPk {
    pub pk: String,
    /// Entry carries text-targeted keys — must land on TEXT.
    pub demands_text: bool,
    /// pk heads a nested (multi-segment) path — must land on INSTANCE.
    pub demands_instance: bool,
}

/// One foreign-session group: pks sharing a non-base sessionID.
pub(super) struct ForeignGroup {
    pub session: u32,
    pub pks: Vec<ForeignPk>,
}

/// Split walk-missed pks into per-session groups.
pub(super) fn group_by_session(pks: &[ForeignPk]) -> Vec<ForeignGroup> {
    let mut by_session: HashMap<u32, Vec<ForeignPk>> = HashMap::new();
    for fp in pks {
        let Some(session) = fp.pk.split(':').next().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        by_session.entry(session).or_default().push(fp.clone());
    }
    let mut out: Vec<ForeignGroup> = by_session
        .into_iter()
        .map(|(session, pks)| ForeignGroup { session, pks })
        .collect();
    out.sort_by_key(|g| g.session);
    out
}

fn min_lid(group: &ForeignGroup) -> Option<u32> {
    group
        .pks
        .iter()
        .filter_map(|fp| fp.pk.split(':').nth(1).and_then(|s| s.parse::<u32>().ok()))
        .min()
}

fn collect<'a>(n: &'a TreeNode, out: &mut Vec<&'a TreeNode>) {
    out.push(n);
    for c in &n.children {
        collect(c, out);
    }
}

/// Resolve one foreign group to `pk → node guid`, or `None` when no
/// unique validated anchor exists. Requires ≥2 text-demanding entries
/// — a single hit is too weak to certify an anchor.
pub(super) fn resolve_group(
    group: &ForeignGroup,
    symbol_node: &TreeNode,
) -> Option<HashMap<String, String>> {
    let base = min_lid(group)?;
    let text_pks: Vec<&String> = group
        .pks
        .iter()
        .filter(|fp| fp.demands_text)
        .map(|fp| &fp.pk)
        .collect();
    let instance_pks: Vec<&String> = group
        .pks
        .iter()
        .filter(|fp| fp.demands_instance)
        .map(|fp| &fp.pk)
        .collect();
    if text_pks.len() < 2 {
        return None;
    }

    // Candidate anchors: every subtree node, in two modes — the walk
    // starting AT the node (self) and at its children.
    let mut anchors: Vec<&TreeNode> = Vec::new();
    collect(symbol_node, &mut anchors);

    let type_at = |map: &HashMap<String, String>,
                   pk: &String,
                   by_guid: &HashMap<String, &TreeNode>,
                   ty: &str| {
        map.get(pk)
            .and_then(|g| by_guid.get(g))
            .map(|n| n.figma.get_str("type") == Some(ty))
    };
    let mut by_guid: HashMap<String, &TreeNode> = HashMap::new();
    for n in &anchors {
        if let Some(g) = n.figma.get("guid").and_then(crate::tree::guid_to_string) {
            by_guid.insert(g, n);
        }
    }

    let mut winner: Option<HashMap<String, String>> = None;
    for anchor in &anchors {
        for children_mode in [false, true] {
            let mut map: HashMap<String, String> = HashMap::new();
            let mut idx: u32 = 0;
            if children_mode {
                let mut sorted: Vec<&TreeNode> = anchor.children.iter().collect();
                sorted.sort_by_key(|n| local_id(n));
                for c in sorted {
                    walk_virtual(c, group.session, base, &mut idx, &mut map);
                }
            } else {
                walk_virtual(anchor, group.session, base, &mut idx, &mut map);
            }
            // Every demand must map, and map to its demanded type.
            let texts_ok = text_pks
                .iter()
                .all(|pk| type_at(&map, pk, &by_guid, "TEXT") == Some(true));
            let instances_ok = instance_pks
                .iter()
                .all(|pk| type_at(&map, pk, &by_guid, "INSTANCE") == Some(true));
            if !texts_ok || !instances_ok {
                continue;
            }
            if winner.is_some() {
                // Ambiguous — two walks both validate. Not trustable.
                return None;
            }
            winner = Some(map);
        }
    }
    winner
}

/// Keys a uniform-family payload may carry (beyond guidPath) — purely
/// cosmetic show/hide toggles where a permutation can't corrupt
/// anything.
const FAMILY_SAFE_KEYS: &[&str] = &["opacity", "visible"];

fn stripped(entry: &FigValue) -> Option<Vec<(&String, &FigValue)>> {
    let FigValue::Object(pairs) = entry else {
        return None;
    };
    let kept: Vec<(&String, &FigValue)> = pairs
        .iter()
        .filter(|(k, _)| k != "guidPath")
        .map(|(k, v)| (k, v))
        .collect();
    if kept.is_empty()
        || !kept
            .iter()
            .all(|(k, _)| FAMILY_SAFE_KEYS.contains(&k.as_str()))
    {
        return None;
    }
    Some(kept)
}

/// Fallback for groups with no validated anchor: when the group's K
/// (≥2) payload-carrying entries are IDENTICAL cosmetic toggles and
/// the symbol subtree holds exactly ONE family of K siblings sharing
/// (parent, name, type), the payload applies to the whole family —
/// order-independent, so no allocation-order guessing is involved.
pub(super) fn resolve_uniform_family(
    group: &ForeignGroup,
    symbol_node: &TreeNode,
    override_map: &HashMap<String, &FigValue>,
) -> Option<HashMap<String, String>> {
    let mut payload_pks: Vec<&String> = group
        .pks
        .iter()
        .filter(|fp| override_map.contains_key(&fp.pk))
        .map(|fp| &fp.pk)
        .collect();
    if payload_pks.len() < 2 {
        return None;
    }
    let first = stripped(override_map[payload_pks[0]])?;
    for pk in &payload_pks[1..] {
        if stripped(override_map[*pk])? != first {
            return None;
        }
    }

    // Families: nodes grouped by (parent guid, name, type).
    let mut families: HashMap<(String, String, String), Vec<&TreeNode>> = HashMap::new();
    fn go<'a>(
        n: &'a TreeNode,
        parent: &str,
        out: &mut HashMap<(String, String, String), Vec<&'a TreeNode>>,
    ) {
        let name = n.figma.get_str("name").unwrap_or_default();
        let ty = n.figma.get_str("type").unwrap_or_default();
        out.entry((parent.to_string(), name.to_string(), ty.to_string()))
            .or_default()
            .push(n);
        let own = n
            .figma
            .get("guid")
            .and_then(crate::tree::guid_to_string)
            .unwrap_or_default();
        for c in &n.children {
            go(c, &own, out);
        }
    }
    for c in &symbol_node.children {
        go(
            c,
            symbol_node
                .figma
                .get("guid")
                .and_then(crate::tree::guid_to_string)
                .unwrap_or_default()
                .as_str(),
            &mut families,
        );
    }
    let k = payload_pks.len();
    let mut hits: Vec<&Vec<&TreeNode>> = families.values().filter(|f| f.len() == k).collect();
    if hits.len() != 1 {
        return None;
    }
    let family = hits.pop()?;
    payload_pks.sort();
    let mut nodes: Vec<&TreeNode> = family.clone();
    nodes.sort_by_key(|n| local_id(n));
    let mut out = HashMap::new();
    for (pk, node) in payload_pks.iter().zip(nodes.iter()) {
        let g = node
            .figma
            .get("guid")
            .and_then(crate::tree::guid_to_string)?;
        out.insert((*pk).clone(), g);
    }
    Some(out)
}
